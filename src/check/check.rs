use std::{
    borrow::Cow,
    collections::{hash_map, HashMap},
    fmt::Display,
};

use logos::Span;

use crate::{
    ast,
    case::{to_json_name, to_lower_without_underscores},
    index_to_i32,
    lines::LineResolver,
    make_name,
    options::{self, OptionSet},
    parse_namespace,
    types::{
        descriptor_proto::{ExtensionRange, ReservedRange},
        enum_descriptor_proto::EnumReservedRange,
        field_descriptor_proto,
        source_code_info::Location,
        DescriptorProto, EnumDescriptorProto, EnumValueDescriptorProto, FieldDescriptorProto,
        FileDescriptorProto, MethodDescriptorProto, OneofDescriptorProto, ServiceDescriptorProto,
        SourceCodeInfo,
    },
};

use super::{
    ir, names::DefinitionKind, CheckError, NameMap, MAX_MESSAGE_FIELD_NUMBER,
    RESERVED_MESSAGE_FIELD_NUMBERS,
};

impl<'a> ir::File<'a> {
    pub fn check(
        &self,
        file_name: Option<&str>,
        name_map: Option<&NameMap>,
        source: Option<&str>,
    ) -> Result<FileDescriptorProto, Vec<CheckError>> {
        let mut context = Context {
            syntax: self.ast.syntax,
            name_map,
            scope: Vec::new(),
            errors: Vec::new(),
            path: Vec::new(),
            locations: Vec::new(),
            lines: source.map(LineResolver::new),
            is_google_descriptor: file_name == Some("google/protobuf/descriptor.proto"),
        };

        let file = context.check_file(self);
        debug_assert!(context.scope.is_empty());

        let source_code_info = if source.is_some() {
            Some(SourceCodeInfo {
                location: context.locations,
            })
        } else {
            None
        };

        if context.errors.is_empty() {
            Ok(FileDescriptorProto {
                name: file_name.map(ToOwned::to_owned),
                source_code_info,
                ..file
            })
        } else {
            Err(context.errors)
        }
    }
}

struct Context<'a> {
    syntax: ast::Syntax,
    name_map: Option<&'a NameMap>,
    scope: Vec<Scope>,
    errors: Vec<CheckError>,
    path: Vec<i32>,
    locations: Vec<Location>,
    lines: Option<LineResolver>,
    is_google_descriptor: bool,
}

enum Scope {
    Package { full_name: String },
    Message { full_name: String },
    Oneof { synthetic: bool },
    Extend,
}

impl<'a> Context<'a> {
    fn enter(&mut self, scope: Scope) {
        self.scope.push(scope);
    }

    fn exit(&mut self) {
        self.scope.pop().expect("unbalanced scope stack");
    }

    fn add_location(&mut self, span: Span) {
        if let Some(lines) = &self.lines {
            let span = lines.resolve_span(span);
            self.locations.push(Location {
                path: self.path.clone(),
                span,
                ..Default::default()
            });
        }
    }

    fn add_comments(&mut self, span: Span, comments: ast::Comments) {
        if let Some(lines) = &self.lines {
            let span = lines.resolve_span(span);
            self.locations.push(Location {
                path: self.path.clone(),
                span,
                leading_comments: comments.leading_comment,
                trailing_comments: comments.trailing_comment,
                leading_detached_comments: comments.leading_detached_comments,
            });
        }
    }

    fn add_location_for(&mut self, path_items: &[i32], span: Span) {
        self.path.extend_from_slice(path_items);
        self.add_location(span);
        self.pop_path(path_items.len());
    }

    fn add_comments_for(&mut self, path_items: &[i32], span: Span, comments: ast::Comments) {
        self.path.extend_from_slice(path_items);
        self.add_comments(span, comments);
        self.pop_path(path_items.len());
    }

    fn pop_path(&mut self, n: usize) {
        self.path.truncate(self.path.len() - n);
    }

    fn replace_path(&mut self, path_items: &[i32]) {
        self.pop_path(path_items.len());
        self.path.extend(path_items);
    }

    fn scope_name(&self) -> &str {
        for def in self.scope.iter().rev() {
            match def {
                Scope::Message { full_name, .. } | Scope::Package { full_name } => {
                    return full_name.as_str()
                }
                _ => continue,
            }
        }

        ""
    }

    fn full_name(&self, name: impl Display) -> String {
        make_name(self.scope_name(), name)
    }

    fn resolve_type_name(
        &mut self,
        type_name: &ast::TypeName,
    ) -> (String, Option<&DefinitionKind>) {
        if let Some(name_map) = &self.name_map {
            let name = type_name.name.to_string();
            if let Some((name, def)) = name_map.resolve(self.scope_name(), &name) {
                (name.into_owned(), Some(def))
            } else {
                self.errors.push(CheckError::TypeNameNotFound {
                    name: name.clone(),
                    span: type_name.span(),
                });
                (name, None)
            }
        } else {
            (type_name.to_string(), None)
        }
    }

    fn in_extend(&self) -> bool {
        matches!(self.scope.last(), Some(Scope::Extend { .. }))
    }

    fn in_oneof(&self) -> bool {
        matches!(self.scope.last(), Some(Scope::Oneof { synthetic: false }))
    }

    fn in_synthetic_oneof(&self) -> bool {
        matches!(self.scope.last(), Some(Scope::Oneof { synthetic: true }))
    }

    fn check_file(&mut self, file: &ir::File) -> FileDescriptorProto {
        const PACKAGE: i32 = 2;
        const DEPENDENCY: i32 = 3;
        const PUBLIC_DEPENDENCY: i32 = 10;
        const WEAK_DEPENDENCY: i32 = 11;
        const MESSAGE_TYPE: i32 = 4;
        const ENUM_TYPE: i32 = 5;
        const SERVICE: i32 = 6;
        const EXTENSION: i32 = 7;
        const OPTIONS: i32 = 8;
        const SYNTAX: i32 = 12;

        self.add_location(file.ast.span.clone());

        if let Some(package) = &file.ast.package {
            self.add_comments_for(&[PACKAGE], package.span.clone(), package.comments.clone());

            for part in &package.name.parts {
                self.enter(Scope::Package {
                    full_name: self.full_name(part),
                });
            }
        }

        let package = file.ast.package.as_ref().map(|p| p.name.to_string());

        let mut dependency = Vec::with_capacity(file.ast.imports.len());
        let mut public_dependency = Vec::new();
        let mut weak_dependency = Vec::new();

        for import in file.ast.imports.iter() {
            let index = index_to_i32(dependency.len());

            self.add_comments_for(
                &[DEPENDENCY, index],
                import.span.clone(),
                import.comments.clone(),
            );

            dependency.push(import.value.clone());
            match &import.kind {
                Some((ast::ImportKind::Public, _)) => {
                    self.add_location_for(
                        &[PUBLIC_DEPENDENCY, index_to_i32(public_dependency.len())],
                        import.span.clone(),
                    );
                    public_dependency.push(index);
                }
                Some((ast::ImportKind::Weak, _)) => {
                    self.add_location_for(
                        &[WEAK_DEPENDENCY, index_to_i32(public_dependency.len())],
                        import.span.clone(),
                    );
                    weak_dependency.push(index);
                }
                _ => (),
            }
        }

        self.path.push(MESSAGE_TYPE);
        let message_type = file
            .messages
            .iter()
            .enumerate()
            .map(|(index, message)| {
                self.path.push(index_to_i32(index));
                let desc = self.check_message(message);
                self.path.pop();
                desc
            })
            .collect();
        self.path.pop();

        let mut enum_type = Vec::new();
        let mut service = Vec::new();
        let mut extension = Vec::new();

        for item in &file.ast.items {
            match item {
                ast::FileItem::Message(_) => continue,
                ast::FileItem::Enum(e) => {
                    self.path
                        .extend(&[ENUM_TYPE, index_to_i32(enum_type.len())]);
                    enum_type.push(self.check_enum(e));
                    self.pop_path(2);
                }
                ast::FileItem::Extend(e) => {
                    self.path.push(EXTENSION);
                    self.check_extend(e, &mut extension);
                    self.path.pop();
                }
                ast::FileItem::Service(s) => {
                    self.path.extend(&[SERVICE, index_to_i32(service.len())]);
                    service.push(self.check_service(s));
                    self.pop_path(2);
                }
            }
        }

        self.path.push(OPTIONS);
        let options = self.check_file_options(&file.ast.options);
        self.path.pop();

        if let Some((syntax_span, syntax_comments)) = &file.ast.syntax_span {
            self.add_comments_for(&[SYNTAX], syntax_span.clone(), syntax_comments.clone());
        }
        let syntax = if self.syntax == ast::Syntax::default() {
            None
        } else {
            Some(self.syntax.to_string())
        };

        if let Some(package) = &file.ast.package {
            for _ in &package.name.parts {
                self.exit();
            }
        }

        FileDescriptorProto {
            name: None,
            package,
            dependency,
            public_dependency,
            weak_dependency,
            message_type,
            enum_type,
            service,
            extension,
            options,
            source_code_info: None,
            syntax,
        }
    }

    fn check_message(&mut self, message: &ir::Message) -> DescriptorProto {
        const NAME: i32 = 1;
        const FIELD: i32 = 2;
        const EXTENSION: i32 = 6;
        const NESTED_TYPE: i32 = 3;
        const ENUM_TYPE: i32 = 4;
        const EXTENSION_RANGE: i32 = 5;
        const OPTIONS: i32 = 7;
        const ONEOF_DECL: i32 = 8;
        const RESERVED_RANGE: i32 = 9;
        const RESERVED_NAME: i32 = 10;

        self.enter(Scope::Message {
            full_name: self.full_name(&message.ast.name()),
        });

        match message.ast {
            ir::MessageSource::Message(message) => {
                self.add_comments(message.span.clone(), message.comments.clone());
                self.add_location_for(&[NAME], message.name.span.clone());
            }
            ir::MessageSource::Group(field, _) => {
                self.add_comments(field.span.clone(), field.comments.clone());
                self.add_location_for(&[NAME], field.name.span.clone());
            }
            ir::MessageSource::Map(_) => (),
        }

        self.path.extend(&[FIELD, 0]);
        let field = message
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                self.replace_path(&[index_to_i32(index)]);
                self.check_message_field(field)
            })
            .collect();
        self.replace_path(&[NESTED_TYPE, 0]);
        let nested_type = message
            .messages
            .iter()
            .enumerate()
            .map(|(index, message)| {
                self.replace_path(&[index_to_i32(index)]);
                self.check_message(message)
            })
            .collect();
        self.replace_path(&[ONEOF_DECL, 0]);
        let oneof_decl = message
            .oneofs
            .iter()
            .enumerate()
            .map(|(index, oneof)| {
                self.replace_path(&[index_to_i32(index)]);
                self.check_oneof(oneof)
            })
            .collect();
        self.pop_path(2);

        self.check_message_field_camel_case_names(message.fields.iter());

        let mut enum_type = Vec::new();
        let mut extension = Vec::new();
        let mut extension_range = Vec::new();
        let mut reserved_range = Vec::new();
        let mut reserved_name = Vec::new();
        let mut options = None;
        if let Some(body) = message.ast.body() {
            for item in &body.items {
                match item {
                    ast::MessageItem::Field(_)
                    | ast::MessageItem::Message(_)
                    | ast::MessageItem::Oneof(_) => continue,
                    ast::MessageItem::Enum(e) => {
                        self.path
                            .extend(&[ENUM_TYPE, index_to_i32(enum_type.len())]);
                        enum_type.push(self.check_enum(e));
                        self.pop_path(2);
                    }
                    ast::MessageItem::Extend(e) => {
                        self.path.push(EXTENSION);
                        self.check_extend(e, &mut extension);
                        self.path.pop();
                    }
                }
            }

            for reserved in &body.reserved {
                match &reserved.kind {
                    ast::ReservedKind::Ranges(ranges) => {
                        self.path.push(RESERVED_RANGE);
                        self.add_comments(reserved.span.clone(), reserved.comments.clone());
                        for range in ranges {
                            self.path.push(index_to_i32(reserved_range.len()));
                            reserved_range.push(self.check_message_reserved_range(range));
                            self.path.pop();
                        }
                        self.path.pop();
                    }

                    ast::ReservedKind::Names(names) => {
                        self.path.push(RESERVED_NAME);
                        self.add_comments(reserved.span.clone(), reserved.comments.clone());
                        for name in names {
                            self.path.push(index_to_i32(reserved_name.len()));
                            reserved_name.push(name.value.to_owned());
                            self.path.pop();
                        }
                        self.path.pop();
                    }
                }
            }

            self.path.push(EXTENSION_RANGE);
            for extensions in &body.extensions {
                self.add_comments(extensions.span.clone(), extensions.comments.clone());

                for range in &extensions.ranges {
                    self.path.push(index_to_i32(extension_range.len()));
                    extension_range.push(
                        self.check_message_extension_range(range, extensions.options.as_ref()),
                    );
                    self.path.pop();
                }
            }
            self.path.pop();

            self.path.push(OPTIONS);
            options = self.check_message_options(&body.options);
            self.path.pop();
        };

        if let ir::MessageSource::Map(_) = &message.ast {
            // TODO this might panic if user defines their own google.protobuf.MessageOptions.
            options
                .get_or_insert_with(Default::default)
                .set(options::MESSAGE_MAP_ENTRY, options::Value::Bool(true));
        };

        self.exit();
        DescriptorProto {
            name: Some(message.ast.name().into_owned()),
            field,
            nested_type,
            extension,
            enum_type,
            extension_range,
            oneof_decl,
            options,
            reserved_range,
            reserved_name,
        }
    }

    fn check_message_field(&mut self, field: &ir::Field) -> FieldDescriptorProto {
        let name = field.ast.name();
        let json_name = Some(to_json_name(&name));
        let number = self.check_field_number(&field.ast.number());
        let (ty, type_name) = self.check_type(&field.ast.ty(), field.ast.is_group());

        let oneof_index = field.oneof_index;

        if oneof_index.is_some() {
            self.enter(Scope::Oneof {
                synthetic: field.is_synthetic_oneof,
            });
        }
        let descriptor = match &field.ast {
            ir::FieldSource::Field(ast) => self.check_field(ast, ty, type_name.as_deref()),
            ir::FieldSource::MapKey(ty, span) => self.check_map_key(ty, span.clone()),
            ir::FieldSource::MapValue(..) => self.check_map_value(),
        };
        if oneof_index.is_some() {
            self.exit();
        }

        FieldDescriptorProto {
            name: Some(name.into_owned()),
            json_name,
            number,
            r#type: ty.map(|ty| ty as i32),
            type_name,
            oneof_index: field.oneof_index,
            ..descriptor
        }
    }

    fn check_field(
        &mut self,
        field: &ast::Field,
        ty: Option<field_descriptor_proto::Type>,
        type_name: Option<&str>,
    ) -> FieldDescriptorProto {
        const NAME: i32 = 1;
        const NUMBER: i32 = 3;
        const LABEL: i32 = 4;
        const TYPE: i32 = 5;
        const TYPE_NAME: i32 = 6;
        const DEFAULT_VALUE: i32 = 7;
        const OPTIONS: i32 = 8;

        self.add_comments(field.span.clone(), field.comments.clone());

        self.add_location_for(&[NAME], field.name.span.clone());
        self.add_location_for(&[NUMBER], field.number.span.clone());

        let label = self.check_field_label(field);
        if let Some((_, label_span)) = &field.label {
            self.add_location_for(&[LABEL], label_span.clone());
        }

        match &field.kind {
            ast::FieldKind::Normal {
                ty: ast::Ty::Named(name),
                ..
            } => {
                self.add_location_for(&[TYPE_NAME], name.span());
            }
            ast::FieldKind::Normal { ty_span, .. } => {
                self.add_location_for(&[TYPE], ty_span.clone());
            }
            ast::FieldKind::Group { ty_span, .. } => {
                self.add_location_for(&[TYPE], ty_span.clone());
                self.add_location_for(&[TYPE_NAME], field.name.span.clone());
            }
            ast::FieldKind::Map { ty_span, .. } => {
                self.add_location_for(&[TYPE_NAME], ty_span.clone());
            }
        }

        self.path.push(OPTIONS);
        let options = self.check_field_options(field.options.as_ref());
        self.path.pop();

        let default_value = self.check_field_default_value(field, ty, type_name);
        if let Some(default_value) = &field.default_value() {
            self.add_location_for(&[DEFAULT_VALUE], default_value.value.span());
        }

        let proto3_optional = if self.in_synthetic_oneof() {
            Some(true)
        } else {
            None
        };

        FieldDescriptorProto {
            label: label.map(|l| l as i32),
            default_value,
            options,
            proto3_optional,
            ..Default::default()
        }
    }

    fn check_field_label(&mut self, field: &ast::Field) -> Option<field_descriptor_proto::Label> {
        let (label, span) = match field.label.clone() {
            Some((label, span)) => (Some(label), span),
            None => (None, field.span.clone()),
        };

        if let ast::FieldKind::Map { .. } = &field.kind {
            if self.in_oneof() {
                self.errors.push(CheckError::InvalidOneofFieldKind {
                    kind: "map",
                    span: field.span.clone(),
                });
                return None;
            } else if self.in_extend() {
                self.errors.push(CheckError::InvalidExtendFieldKind {
                    kind: "map",
                    span: field.span.clone(),
                });
                return None;
            } else if label.is_some() {
                self.errors.push(CheckError::MapFieldWithLabel { span });
                return None;
            } else {
                return Some(field_descriptor_proto::Label::Repeated);
            }
        } else if let ast::FieldKind::Group { .. } = &field.kind {
            if self.syntax != ast::Syntax::Proto2 {
                self.errors.push(CheckError::Proto3GroupField {
                    span: field.span.clone(),
                });
                return None;
            }
        }

        match (self.in_extend(), self.in_oneof(), label) {
            (true, true, _) => unreachable!(),
            (true, false, Some(ast::FieldLabel::Required)) => {
                self.errors.push(CheckError::RequiredExtendField { span });
                None
            }
            (false, true, Some(_)) => {
                self.errors.push(CheckError::OneofFieldWithLabel { span });
                None
            }
            (_, false, None) if self.syntax == ast::Syntax::Proto2 => {
                self.errors
                    .push(CheckError::Proto2FieldMissingLabel { span });
                None
            }
            (_, _, Some(ast::FieldLabel::Required)) if self.syntax == ast::Syntax::Proto3 => {
                self.errors.push(CheckError::Proto3RequiredField { span });
                None
            }
            (_, _, Some(ast::FieldLabel::Required)) => {
                Some(field_descriptor_proto::Label::Required)
            }
            (_, _, Some(ast::FieldLabel::Repeated)) => {
                Some(field_descriptor_proto::Label::Repeated)
            }
            (_, _, Some(ast::FieldLabel::Optional) | None) => {
                Some(field_descriptor_proto::Label::Optional)
            }
        }
    }

    fn check_map_key(&mut self, ty: &ast::Ty, span: Span) -> FieldDescriptorProto {
        match ty {
            ast::Ty::Double => Some(field_descriptor_proto::Type::Double),
            ast::Ty::Float => Some(field_descriptor_proto::Type::Float),
            ast::Ty::Int32 => Some(field_descriptor_proto::Type::Int32),
            ast::Ty::Int64 => Some(field_descriptor_proto::Type::Int64),
            ast::Ty::Uint32 => Some(field_descriptor_proto::Type::Uint32),
            ast::Ty::Uint64 => Some(field_descriptor_proto::Type::Uint64),
            ast::Ty::Sint32 => Some(field_descriptor_proto::Type::Sint32),
            ast::Ty::Sint64 => Some(field_descriptor_proto::Type::Sint64),
            ast::Ty::Fixed32 => Some(field_descriptor_proto::Type::Fixed32),
            ast::Ty::Fixed64 => Some(field_descriptor_proto::Type::Fixed64),
            ast::Ty::Sfixed32 => Some(field_descriptor_proto::Type::Sfixed32),
            ast::Ty::Sfixed64 => Some(field_descriptor_proto::Type::Sfixed64),
            ast::Ty::Bool => Some(field_descriptor_proto::Type::Bool),
            ast::Ty::String => Some(field_descriptor_proto::Type::String),
            _ => {
                self.errors
                    .push(CheckError::InvalidMapFieldKeyType { span });
                None
            }
        };

        FieldDescriptorProto {
            label: Some(field_descriptor_proto::Label::Optional as i32),
            ..Default::default()
        }
    }

    fn check_map_value(&mut self) -> FieldDescriptorProto {
        FieldDescriptorProto {
            label: Some(field_descriptor_proto::Label::Optional as i32),
            ..Default::default()
        }
    }

    fn check_type(
        &mut self,
        ty: &ast::Ty,
        is_group: bool,
    ) -> (Option<field_descriptor_proto::Type>, Option<String>) {
        match ty {
            ast::Ty::Named(type_name) => match self.resolve_type_name(type_name) {
                (name, None) => (None, Some(name)),
                (name, Some(DefinitionKind::Message)) => {
                    if is_group {
                        (Some(field_descriptor_proto::Type::Group as _), Some(name))
                    } else {
                        (Some(field_descriptor_proto::Type::Message as _), Some(name))
                    }
                }
                (name, Some(DefinitionKind::Enum)) => {
                    (Some(field_descriptor_proto::Type::Enum as _), Some(name))
                }
                (name, Some(_)) => {
                    self.errors.push(CheckError::InvalidMessageFieldTypeName {
                        name: type_name.to_string(),
                        span: type_name.span(),
                    });
                    (None, Some(name))
                }
            },
            _ => (ty.proto_ty(), None),
        }
    }

    fn check_field_default_value(
        &mut self,
        field: &ast::Field,
        ty: Option<field_descriptor_proto::Type>,
        type_name: Option<&str>,
    ) -> Option<String> {
        if let Some(option) = field.default_value() {
            if self.syntax != ast::Syntax::Proto2 {
                self.errors.push(CheckError::Proto3DefaultValue {
                    span: option.span(),
                })
            } else if field.is_map() {
                self.errors.push(CheckError::InvalidDefault {
                    kind: "map",
                    span: option.span(),
                })
            } else if ty == Some(field_descriptor_proto::Type::Group) {
                self.errors.push(CheckError::InvalidDefault {
                    kind: "group",
                    span: option.span(),
                })
            } else if ty == Some(field_descriptor_proto::Type::Message) {
                self.errors.push(CheckError::InvalidDefault {
                    kind: "message",
                    span: option.span(),
                })
            } else if matches!(field.label, Some((ast::FieldLabel::Repeated, _))) {
                self.errors.push(CheckError::InvalidDefault {
                    kind: "repeated",
                    span: option.span(),
                })
            } else if self.name_map.is_some() || type_name.is_none() {
                let scope_name = self.scope_name().to_owned();
                let _ = self.check_option_value(ty, option, &scope_name, type_name);
            }

            Some(option.value.to_string())
        } else {
            None
        }
    }

    fn check_message_field_camel_case_names<'b>(
        &mut self,
        fields: impl Iterator<Item = &'b ir::Field<'b>>,
    ) {
        if self.syntax != ast::Syntax::Proto2 {
            let mut names: HashMap<String, (String, Span)> = HashMap::new();
            for field in fields {
                let name = field.ast.name().into_owned();
                let span = field.ast.name_span();

                match names.entry(to_lower_without_underscores(&name)) {
                    hash_map::Entry::Occupied(entry) => {
                        self.errors.push(CheckError::DuplicateCamelCaseFieldName {
                            first_name: entry.get().0.clone(),
                            first: entry.get().1.clone(),
                            second_name: name,
                            second: span,
                        })
                    }
                    hash_map::Entry::Vacant(entry) => {
                        entry.insert((name, span));
                    }
                }
            }
        }
    }

    fn check_oneof(&mut self, oneof: &ir::Oneof) -> OneofDescriptorProto {
        const NAME: i32 = 1;
        const OPTIONS: i32 = 2;

        match oneof.ast {
            ir::OneofSource::Oneof(oneof) => {
                self.add_location(oneof.span.clone());
                self.add_location_for(&[NAME], oneof.name.span.clone());

                self.path.push(OPTIONS);
                let options = self.check_oneof_options(&oneof.options);
                self.path.pop();

                OneofDescriptorProto {
                    name: Some(oneof.name.value.clone()),
                    options,
                }
            }
            ir::OneofSource::Field(field) => OneofDescriptorProto {
                name: Some(field.synthetic_oneof_name()),
                options: None,
            },
        }
    }

    fn check_extend(&mut self, extend: &ast::Extend, result: &mut Vec<FieldDescriptorProto>) {
        const FIELD_EXTENDEE: i32 = 2;

        self.add_comments(extend.span.clone(), extend.comments.clone());

        let (extendee, def) = self.resolve_type_name(&extend.extendee);
        if def.is_some() && def != Some(&DefinitionKind::Message) {
            self.errors.push(CheckError::InvalidExtendeeTypeName {
                name: extend.extendee.to_string(),
                span: extend.extendee.span(),
            })
        }

        self.enter(Scope::Extend);
        for field in &extend.fields {
            self.path.push(index_to_i32(result.len()));
            self.add_location_for(&[FIELD_EXTENDEE], extend.extendee.span());

            let name = field.field_name();
            let json_name = Some(to_json_name(&name));
            let number = self.check_field_number(&field.number);
            let (ty, type_name) = self.check_type(&field.ty(), field.is_group());

            let desc = self.check_field(field, ty, type_name.as_deref());
            result.push(FieldDescriptorProto {
                name: Some(name.into_owned()),
                json_name,
                number,
                r#type: ty.map(|ty| ty as i32),
                type_name,
                extendee: Some(extendee.clone()),
                ..desc
            });
            self.path.pop();
        }
        self.exit();
    }

    fn check_enum(&mut self, e: &ast::Enum) -> EnumDescriptorProto {
        const NAME: i32 = 1;
        const VALUE: i32 = 2;
        const OPTIONS: i32 = 3;
        const RESERVED_RANGE: i32 = 4;
        const RESERVED_NAME: i32 = 5;

        self.add_comments(e.span.clone(), e.comments.clone());
        self.add_location_for(&[NAME], e.name.span.clone());
        let name = Some(e.name.value.clone());

        self.path.extend(&[VALUE, 0]);
        let value = e
            .values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                self.replace_path(&[index_to_i32(index)]);
                self.check_enum_value(value)
            })
            .collect();
        self.pop_path(2);

        let mut reserved_range = Vec::new();
        let mut reserved_name = Vec::new();
        for reserved in &e.reserved {
            match &reserved.kind {
                ast::ReservedKind::Ranges(ranges) => {
                    self.path.push(RESERVED_RANGE);
                    self.add_comments(reserved.span.clone(), reserved.comments.clone());
                    for range in ranges {
                        self.path.push(index_to_i32(reserved_range.len()));
                        reserved_range.push(self.check_enum_reserved_range(range));
                        self.path.pop();
                    }
                    self.path.pop();
                }

                ast::ReservedKind::Names(names) => {
                    self.path.push(RESERVED_NAME);
                    self.add_comments(reserved.span.clone(), reserved.comments.clone());
                    for name in names {
                        self.path.push(index_to_i32(reserved_name.len()));
                        reserved_name.push(name.value.to_owned());
                        self.path.pop();
                    }
                    self.path.pop();
                }
            }
        }

        self.path.push(OPTIONS);
        let options = self.check_enum_options(&e.options);
        self.path.pop();

        EnumDescriptorProto {
            name,
            value,
            reserved_name,
            reserved_range,
            options,
        }
    }

    fn check_enum_value(&mut self, value: &ast::EnumValue) -> EnumValueDescriptorProto {
        const NAME: i32 = 1;
        const NUMBER: i32 = 2;
        const OPTIONS: i32 = 3;

        self.add_comments(value.span.clone(), value.comments.clone());
        self.add_location_for(&[NAME], value.name.span.clone());
        let name = Some(value.name.value.clone());

        self.add_location_for(&[NUMBER], value.number.span.clone());
        let number = self.check_enum_number(&value.number);

        self.path.push(OPTIONS);
        let options = self.check_enum_value_options(value.options.as_ref());
        self.path.pop();

        EnumValueDescriptorProto {
            name,
            number,
            options,
        }
    }

    fn check_service(&mut self, service: &ast::Service) -> ServiceDescriptorProto {
        const NAME: i32 = 1;
        const METHOD: i32 = 2;
        const OPTIONS: i32 = 3;

        self.add_comments(service.span.clone(), service.comments.clone());
        self.add_location_for(&[NAME], service.name.span.clone());
        let name = Some(service.name.value.clone());

        self.path.extend(&[METHOD, 0]);
        let method = service
            .methods
            .iter()
            .enumerate()
            .map(|(index, method)| {
                self.replace_path(&[index_to_i32(index)]);
                self.check_method(method)
            })
            .collect();
        self.pop_path(2);

        self.path.push(OPTIONS);
        let options = self.check_service_options(&service.options);
        self.path.pop();

        ServiceDescriptorProto {
            name,
            method,
            options,
        }
    }

    fn check_method(&mut self, method: &ast::Method) -> MethodDescriptorProto {
        const NAME: i32 = 1;
        const INPUT_TYPE: i32 = 2;
        const OUTPUT_TYPE: i32 = 3;
        const OPTIONS: i32 = 4;
        const CLIENT_STREAMING: i32 = 5;
        const SERVER_STREAMING: i32 = 6;

        self.add_comments(method.span.clone(), method.comments.clone());
        self.add_location_for(&[NAME], method.name.span.clone());
        let name = Some(method.name.value.clone());

        self.add_location_for(&[INPUT_TYPE], method.input_ty.span());
        let (input_type, kind) = self.resolve_type_name(&method.input_ty);
        if !matches!(kind, None | Some(DefinitionKind::Message)) {
            self.errors.push(CheckError::InvalidMethodTypeName {
                name: method.input_ty.to_string(),
                kind: "input",
                span: method.input_ty.span(),
            })
        }

        self.add_location_for(&[OUTPUT_TYPE], method.output_ty.span());
        let (output_type, kind) = self.resolve_type_name(&method.output_ty);
        if !matches!(kind, None | Some(DefinitionKind::Message)) {
            self.errors.push(CheckError::InvalidMethodTypeName {
                name: method.output_ty.to_string(),
                kind: "output",
                span: method.output_ty.span(),
            })
        }

        if let Some(span) = &method.client_streaming {
            self.add_location_for(&[CLIENT_STREAMING], span.clone());
        }
        if let Some(span) = &method.server_streaming {
            self.add_location_for(&[SERVER_STREAMING], span.clone());
        }

        self.path.push(OPTIONS);
        let options = self.check_method_options(&method.options);
        self.path.pop();

        MethodDescriptorProto {
            name,
            input_type: Some(input_type),
            output_type: Some(output_type),
            options,
            client_streaming: Some(method.client_streaming.is_some()),
            server_streaming: Some(method.server_streaming.is_some()),
        }
    }

    fn check_message_reserved_range(&mut self, range: &ast::ReservedRange) -> ReservedRange {
        let start = self.check_field_number(&range.start);
        let end = match &range.end {
            ast::ReservedRangeEnd::None => start.map(|n| n + 1),
            ast::ReservedRangeEnd::Int(value) => self.check_field_number(value),
            ast::ReservedRangeEnd::Max(_) => Some(MAX_MESSAGE_FIELD_NUMBER + 1),
        };

        ReservedRange { start, end }
    }

    fn check_message_extension_range(
        &mut self,
        range: &ast::ReservedRange,
        options: Option<&ast::OptionList>,
    ) -> ExtensionRange {
        const START: i32 = 1;
        const END: i32 = 2;
        const OPTIONS: i32 = 3;

        self.add_location(range.span());
        self.add_location_for(&[START], range.start_span());
        self.add_location_for(&[END], range.end_span());

        self.path.push(OPTIONS);
        let options = self.check_extension_range_options(options);
        self.path.pop();

        let start = self.check_field_number(&range.start);
        let end = match &range.end {
            ast::ReservedRangeEnd::None => start.map(|n| n + 1),
            ast::ReservedRangeEnd::Int(value) => self.check_field_number(value),
            ast::ReservedRangeEnd::Max(_) => Some(MAX_MESSAGE_FIELD_NUMBER + 1),
        };

        ExtensionRange {
            start,
            end,
            options,
        }
    }

    fn check_enum_reserved_range(&mut self, range: &ast::ReservedRange) -> EnumReservedRange {
        let start = self.check_enum_number(&range.start);
        let end = match &range.end {
            ast::ReservedRangeEnd::None => start,
            ast::ReservedRangeEnd::Int(value) => self.check_enum_number(value),
            ast::ReservedRangeEnd::Max(_) => Some(i32::MAX),
        };

        EnumReservedRange { start, end }
    }

    fn check_field_number(&mut self, int: &ast::Int) -> Option<i32> {
        match int.as_i32() {
            Some(number @ 1..=MAX_MESSAGE_FIELD_NUMBER) => {
                if RESERVED_MESSAGE_FIELD_NUMBERS.contains(&number) {
                    self.errors.push(CheckError::ReservedMessageNumber {
                        span: int.span.clone(),
                    });
                }

                Some(number)
            }
            _ => {
                self.errors.push(CheckError::InvalidMessageNumber {
                    span: int.span.clone(),
                });
                None
            }
        }
    }

    fn check_enum_number(&mut self, int: &ast::Int) -> Option<i32> {
        match int.as_i32() {
            Some(number) => Some(number),
            None => {
                self.errors.push(CheckError::InvalidEnumNumber {
                    span: int.span.clone(),
                });
                None
            }
        }
    }

    fn check_file_options(&mut self, options: &[ast::Option]) -> Option<OptionSet> {
        self.check_options("google.protobuf.FileOptions", options)
    }

    fn check_message_options(&mut self, options: &[ast::Option]) -> Option<OptionSet> {
        self.check_options("google.protobuf.MessageOptions", options)
    }

    fn check_field_options(&mut self, options: Option<&ast::OptionList>) -> Option<OptionSet> {
        self.check_option_list("google.protobuf.FieldOptions", options)
    }

    fn check_extension_range_options(
        &mut self,
        options: Option<&ast::OptionList>,
    ) -> Option<OptionSet> {
        self.check_option_list("google.protobuf.ExtensionRangeOptions", options)
    }

    fn check_oneof_options(&mut self, options: &[ast::Option]) -> Option<OptionSet> {
        self.check_options("google.protobuf.OneofOptions", options)
    }

    fn check_enum_options(&mut self, options: &[ast::Option]) -> Option<OptionSet> {
        self.check_options("google.protobuf.EnumOptions", options)
    }

    fn check_enum_value_options(&mut self, options: Option<&ast::OptionList>) -> Option<OptionSet> {
        self.check_option_list("google.protobuf.EnumValueOptions", options)
    }

    fn check_service_options(&mut self, options: &[ast::Option]) -> Option<OptionSet> {
        self.check_options("google.protobuf.ServiceOptions", options)
    }

    fn check_method_options(&mut self, options: &[ast::Option]) -> Option<OptionSet> {
        self.check_options("google.protobuf.MethodOptions", options)
    }

    fn check_options(&mut self, message_name: &str, options: &[ast::Option]) -> Option<OptionSet> {
        let mut result = OptionSet::new();

        for option in options {
            self.add_location(option.span.clone());

            let _ = self.check_option(&mut result, message_name, &option.body, option.span.clone());
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    fn check_option_list(
        &mut self,
        message_name: &str,
        options: Option<&ast::OptionList>,
    ) -> Option<OptionSet> {
        let mut result = OptionSet::new();

        if let Some(options) = options {
            self.add_location(options.span.clone());
            for option in &options.options {
                let _ = self.check_option(&mut result, message_name, option, option.span());
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    fn check_option(
        &mut self,
        mut result: &mut OptionSet,
        namespace: &str,
        option: &ast::OptionBody,
        option_span: Span,
    ) -> Result<(), ()> {
        use field_descriptor_proto::Type;

        // Special cases for things which use option syntax but aren't options
        if namespace == "google.protobuf.FieldOptions" && option.is("default") {
            return Ok(());
        }

        if self.name_map.is_none() && !option.is_simple() {
            todo!("set uninterpreted_option")
        }

        let mut numbers = vec![];
        let mut ty = None;
        let mut type_name = Some(Cow::Borrowed(namespace));
        let mut type_name_context = None;

        for part in &option.name {
            let namespace = match (ty, &type_name) {
                (None | Some(Type::Message | Type::Group), Some(name)) => name.as_ref(),
                _ => {
                    self.errors
                        .push(CheckError::OptionScalarFieldAccess { span: part.span() });
                    return Err(());
                }
            };

            if let Some(&number) = numbers.last() {
                result = result.get_message_mut(number);
            }

            match part {
                ast::OptionNamePart::Ident(name) => {
                    let full_name = make_name(namespace, name);
                    if let Some(DefinitionKind::Field {
                        number: field_number,
                        ty: field_ty,
                        type_name: field_type_name,
                        extendee: None,
                        ..
                    }) = self.get_option_def(&full_name)
                    {
                        numbers.push(*field_number);
                        ty = *field_ty;
                        type_name_context = Some(namespace.to_owned());
                        type_name = field_type_name.clone().map(Cow::Owned);
                    } else {
                        self.errors.push(CheckError::OptionUnknownField {
                            name: name.to_string(),
                            namespace: namespace.to_owned(),
                            span: name.span.clone(),
                        });
                        return Err(());
                    }
                }
                ast::OptionNamePart::Extension(_, _) => todo!(),
            }
        }

        let value = self.check_option_value(
            ty,
            option,
            type_name_context.as_deref().unwrap_or_default(),
            type_name.as_deref(),
        )?;

        self.add_location_for(&numbers, option_span);
        result.set(
            *numbers.last().expect("expected at least one field access"),
            value,
        );
        Ok(())
    }

    fn check_option_value(
        &mut self,
        ty: Option<field_descriptor_proto::Type>,
        option: &ast::OptionBody,
        type_name_context: &str,
        type_name: Option<&str>,
    ) -> Result<options::Value, ()> {
        use field_descriptor_proto::Type;

        Ok(match ty {
            Some(Type::Double) => {
                options::Value::Double(self.check_option_value_f64(&option.value)?)
            }
            Some(Type::Float) => {
                options::Value::Float(self.check_option_value_f64(&option.value)? as f32)
            }
            Some(Type::Int32) => options::Value::Int32(self.check_option_value_i32(&option.value)?),
            Some(Type::Int64) => options::Value::Int64(self.check_option_value_i64(&option.value)?),
            Some(Type::Uint32) => {
                options::Value::Uint32(self.check_option_value_u32(&option.value)?)
            }
            Some(Type::Uint64) => {
                options::Value::Uint64(self.check_option_value_u64(&option.value)?)
            }
            Some(Type::Sint32) => {
                options::Value::Sint32(self.check_option_value_i32(&option.value)?)
            }
            Some(Type::Sint64) => {
                options::Value::Sint64(self.check_option_value_i64(&option.value)?)
            }
            Some(Type::Fixed32) => {
                options::Value::Fixed32(self.check_option_value_u32(&option.value)?)
            }
            Some(Type::Fixed64) => {
                options::Value::Fixed64(self.check_option_value_u64(&option.value)?)
            }
            Some(Type::Sfixed32) => {
                options::Value::Sfixed32(self.check_option_value_i32(&option.value)?)
            }
            Some(Type::Sfixed64) => {
                options::Value::Sfixed64(self.check_option_value_i64(&option.value)?)
            }
            Some(Type::Bool) => options::Value::Bool(self.check_option_value_bool(&option.value)?),
            Some(Type::String) => {
                options::Value::String(self.check_option_value_string(&option.value)?)
            }
            Some(Type::Bytes) => {
                options::Value::Bytes(self.check_option_value_bytes(&option.value)?)
            }
            Some(Type::Enum) => {
                let value = self.check_option_value_enum(
                    &option.value,
                    type_name_context,
                    type_name.unwrap_or_default(),
                )?;
                options::Value::Int32(value)
            }
            None | Some(Type::Message | Type::Group) => {
                let type_name = type_name.unwrap_or_default();
                match self.resolve_option_def(type_name_context, type_name) {
                    Some(DefinitionKind::Message) => {
                        let value = self.check_option_value_message(&option.value, type_name)?;
                        if ty == Some(Type::Group) {
                            options::Value::Group(value)
                        } else {
                            options::Value::Message(value)
                        }
                    }
                    Some(DefinitionKind::Enum) => {
                        let value = self.check_option_value_enum(
                            &option.value,
                            type_name_context,
                            type_name,
                        )?;
                        options::Value::Int32(value)
                    }
                    Some(_) | None => {
                        self.errors.push(CheckError::OptionInvalidTypeName {
                            name: type_name.to_owned(),
                            span: option.name_span(),
                        });
                        return Err(());
                    }
                }
            }
        })
    }

    fn check_option_value_f64(&mut self, value: &ast::OptionValue) -> Result<f64, ()> {
        match value {
            ast::OptionValue::Float(float) => Ok(float.value),
            _ => {
                self.errors.push(CheckError::ValueInvalidType {
                    expected: "a float".to_owned(),
                    actual: value.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_i32(&mut self, value: &ast::OptionValue) -> Result<i32, ()> {
        match self.check_option_value_int(value)?.as_i32() {
            Some(value) => Ok(value),
            None => {
                self.errors.push(CheckError::IntegerValueOutOfRange {
                    expected: "a signed 32-bit integer".to_owned(),
                    actual: value.to_string(),
                    min: i32::MIN.to_string(),
                    max: i32::MAX.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_i64(&mut self, value: &ast::OptionValue) -> Result<i64, ()> {
        match self.check_option_value_int(value)?.as_i64() {
            Some(value) => Ok(value),
            None => {
                self.errors.push(CheckError::IntegerValueOutOfRange {
                    expected: "a signed 64-bit integer".to_owned(),
                    actual: value.to_string(),
                    min: i64::MIN.to_string(),
                    max: i64::MAX.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_u32(&mut self, value: &ast::OptionValue) -> Result<u32, ()> {
        match self.check_option_value_int(value)?.as_u32() {
            Some(value) => Ok(value),
            None => {
                self.errors.push(CheckError::IntegerValueOutOfRange {
                    expected: "an unsigned 32-bit integer".to_owned(),
                    actual: value.to_string(),
                    min: u32::MIN.to_string(),
                    max: u32::MAX.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_u64(&mut self, value: &ast::OptionValue) -> Result<u64, ()> {
        match self.check_option_value_int(value)?.as_u64() {
            Some(value) => Ok(value),
            None => {
                self.errors.push(CheckError::IntegerValueOutOfRange {
                    expected: "an unsigned 64-bit integer".to_owned(),
                    actual: value.to_string(),
                    min: u64::MIN.to_string(),
                    max: u64::MAX.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_int<'b>(
        &mut self,
        value: &'b ast::OptionValue,
    ) -> Result<&'b ast::Int, ()> {
        match value {
            ast::OptionValue::Int(int) => Ok(int),
            _ => {
                self.errors.push(CheckError::ValueInvalidType {
                    expected: "an integer".to_owned(),
                    actual: value.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_bool(&mut self, value: &ast::OptionValue) -> Result<bool, ()> {
        match value {
            ast::OptionValue::Ident(ident) if ident.value.as_str() == "false" => Ok(false),
            ast::OptionValue::Ident(ident) if ident.value.as_str() == "true" => Ok(true),
            _ => {
                self.errors.push(CheckError::ValueInvalidType {
                    expected: "either 'true' or 'false'".to_owned(),
                    actual: value.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_string(&mut self, value: &ast::OptionValue) -> Result<String, ()> {
        let bytes = self.check_option_value_bytes(value)?;
        match String::from_utf8(bytes) {
            Ok(string) => Ok(string),
            Err(_) => {
                self.errors
                    .push(CheckError::StringValueInvalidUtf8 { span: value.span() });
                Err(())
            }
        }
    }

    fn check_option_value_bytes(&mut self, value: &ast::OptionValue) -> Result<Vec<u8>, ()> {
        match value {
            ast::OptionValue::String(string) => Ok(string.value.clone()),
            _ => {
                self.errors.push(CheckError::ValueInvalidType {
                    expected: "a string".to_owned(),
                    actual: value.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_message(
        &mut self,
        _value: &ast::OptionValue,
        _type_name: &str,
    ) -> Result<OptionSet, ()> {
        todo!()
    }

    fn check_option_value_enum(
        &mut self,
        value: &ast::OptionValue,
        context: &str,
        type_name: &str,
    ) -> Result<i32, ()> {
        let type_namespace = parse_namespace(type_name);

        match value {
            ast::OptionValue::Ident(ident) => {
                match self.resolve_option_def(context, &make_name(type_namespace, &ident.value)) {
                    Some(DefinitionKind::EnumValue { number }) => Ok(*number),
                    _ => {
                        self.errors.push(CheckError::InvalidEnumValue {
                            value_name: ident.value.clone(),
                            enum_name: type_name.to_owned(),
                            span: ident.span.clone(),
                        });
                        Err(())
                    }
                }
            }
            _ => {
                self.errors.push(CheckError::ValueInvalidType {
                    expected: "an enum value identifier".to_owned(),
                    actual: value.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn get_option_def(&self, name: &str) -> Option<&DefinitionKind> {
        if let Some(name_map) = &self.name_map {
            if let Some(def) = name_map.get(name) {
                return Some(def);
            }
        }

        if !self.is_google_descriptor {
            return NameMap::google_descriptor().get(name);
        }

        None
    }

    fn resolve_option_def(&self, context: &str, name: &str) -> Option<&DefinitionKind> {
        if let Some(name_map) = &self.name_map {
            if let Some((_, def)) = name_map.resolve(context, name) {
                return Some(def);
            }
        }

        if !self.is_google_descriptor {
            if let Some((_, def)) = NameMap::google_descriptor().resolve(context, name) {
                return Some(def);
            }
        }

        None
    }
}
