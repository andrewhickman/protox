use std::{
    collections::{hash_map, HashMap},
    fmt::Display,
};

use logos::Span;
use prost_types::{
    descriptor_proto::{ExtensionRange, ReservedRange},
    enum_descriptor_proto::EnumReservedRange,
    field_descriptor_proto, DescriptorProto, EnumDescriptorProto, EnumOptions,
    EnumValueDescriptorProto, EnumValueOptions, ExtensionRangeOptions, FieldDescriptorProto,
    FieldOptions, FileDescriptorProto, FileOptions, MessageOptions, MethodDescriptorProto,
    MethodOptions, OneofDescriptorProto, OneofOptions, ServiceDescriptorProto, ServiceOptions,
};

use crate::{
    ast,
    case::{to_json_name, to_lower_without_underscores},
    s,
};

use super::{
    ir, names::DefinitionKind, CheckError, NameMap, MAX_MESSAGE_FIELD_NUMBER,
    RESERVED_MESSAGE_FIELD_NUMBERS,
};

impl<'a> ir::File<'a> {
    pub fn check(
        &self,
        name_map: Option<&NameMap>,
    ) -> Result<FileDescriptorProto, Vec<CheckError>> {
        let mut context = Context {
            syntax: self.ast.syntax,
            name_map,
            scope: Vec::new(),
            errors: Vec::new(),
        };

        let file = context.check_file(self);

        debug_assert!(context.scope.is_empty());

        if context.errors.is_empty() {
            Ok(file)
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

    fn full_name(&self, name: impl Display) -> String {
        for def in self.scope.iter().rev() {
            match def {
                Scope::Message { full_name, .. } | Scope::Package { full_name } => {
                    return format!("{}.{}", full_name, name)
                }
                _ => continue,
            }
        }

        name.to_string()
    }

    fn resolve_ast_type_name(
        &mut self,
        type_name: &ast::TypeName,
    ) -> (String, Option<&DefinitionKind>) {
        self.resolve_type_name(
            type_name.leading_dot.is_some(),
            type_name.name.to_string(),
            type_name.span(),
        )
    }

    fn resolve_type_name(
        &mut self,
        absolute: bool,
        name: String,
        span: Span,
    ) -> (String, Option<&DefinitionKind>) {
        if let Some(name_map) = &self.name_map {
            if absolute {
                if let Some(def) = name_map.get(&name) {
                    (name, Some(def))
                } else {
                    self.errors.push(CheckError::TypeNameNotFound {
                        name: name.clone(),
                        span,
                    });
                    (name, None)
                }
            } else {
                for scope in self.scope.iter().rev() {
                    let full_name = match scope {
                        Scope::Message { full_name, .. } | Scope::Package { full_name } => {
                            format!(".{}.{}", full_name, name)
                        }
                        _ => continue,
                    };

                    if let Some(def) = name_map.get(&full_name[1..]) {
                        return (full_name, Some(def));
                    }
                }

                if let Some(def) = name_map.get(&name) {
                    return (format!(".{}", name), Some(def));
                }

                self.errors.push(CheckError::TypeNameNotFound {
                    name: name.clone(),
                    span,
                });
                (name, None)
            }
        } else {
            (name, None)
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
        if let Some(package) = &file.ast.package {
            for part in &package.name.parts {
                self.enter(Scope::Package {
                    full_name: self.full_name(part),
                });
            }
        }

        let package = file.ast.package.as_ref().map(|p| p.name.to_string());

        let dependency = file.ast.imports.iter().map(|i| i.value.clone()).collect();
        let public_dependency = file.ast.public_imports().map(|(index, _)| index).collect();
        let weak_dependency = file.ast.weak_imports().map(|(index, _)| index).collect();

        let message_type = file
            .messages
            .iter()
            .map(|message| self.check_message(message))
            .collect();

        let mut enum_type = Vec::new();
        let mut service = Vec::new();
        let mut extension = Vec::new();

        for item in &file.ast.items {
            match item {
                ast::FileItem::Message(_) => continue,
                ast::FileItem::Enum(e) => enum_type.push(self.check_enum(e)),
                ast::FileItem::Extend(e) => self.check_extend(e, &mut extension),
                ast::FileItem::Service(s) => service.push(self.check_service(s)),
            }
        }

        let options = self.check_file_options(&file.ast.options);

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
        self.enter(Scope::Message {
            full_name: self.full_name(&message.ast.name()),
        });

        let field = message
            .fields
            .iter()
            .map(|field| self.check_message_field(field))
            .collect();
        let nested_type = message
            .messages
            .iter()
            .map(|nested| self.check_message(nested))
            .collect();
        let oneof_decl = message
            .oneofs
            .iter()
            .map(|oneof| self.check_oneof(oneof))
            .collect();

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
                    ast::MessageItem::Enum(e) => enum_type.push(self.check_enum(e)),
                    ast::MessageItem::Extend(e) => self.check_extend(e, &mut extension),
                }
            }

            for reserved in &body.reserved {
                match &reserved.kind {
                    ast::ReservedKind::Ranges(ranges) => reserved_range.extend(
                        ranges
                            .iter()
                            .map(|range| self.check_message_reserved_range(range)),
                    ),
                    ast::ReservedKind::Names(names) => {
                        reserved_name.extend(names.iter().map(|name| name.value.to_owned()))
                    }
                }
            }

            for extension in &body.extensions {
                let extension_options =
                    self.check_extension_range_options(extension.options.as_ref());

                extension_range.extend(extension.ranges.iter().map(|e| ExtensionRange {
                    options: extension_options.clone(),
                    ..self.check_message_extension_range(e)
                }));
            }

            options = self.check_message_options(&body.options);
        };

        if let ir::MessageSource::Map(_) = &message.ast {
            options.get_or_insert_with(Default::default).map_entry = Some(true);
        };

        self.exit();
        DescriptorProto {
            name: s(message.ast.name()),
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
        let (ty, type_name) = self.check_type(&field.ast.ty());

        let oneof_index = field.oneof_index;

        if oneof_index.is_some() {
            self.enter(Scope::Oneof {
                synthetic: field.is_synthetic_oneof,
            });
        }
        let descriptor = match &field.ast {
            ir::FieldSource::Field(ast) => self.check_field(ast, ty),
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
    ) -> FieldDescriptorProto {
        let label = self.check_field_label(field);

        let options = self.check_field_options(field.options.as_ref());

        let default_value = self.check_field_default_value(field, ty);

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
    ) -> (Option<field_descriptor_proto::Type>, Option<String>) {
        match ty {
            ast::Ty::Double => (Some(field_descriptor_proto::Type::Double), None),
            ast::Ty::Float => (Some(field_descriptor_proto::Type::Float), None),
            ast::Ty::Int32 => (Some(field_descriptor_proto::Type::Int32), None),
            ast::Ty::Int64 => (Some(field_descriptor_proto::Type::Int64), None),
            ast::Ty::Uint32 => (Some(field_descriptor_proto::Type::Uint32), None),
            ast::Ty::Uint64 => (Some(field_descriptor_proto::Type::Uint64), None),
            ast::Ty::Sint32 => (Some(field_descriptor_proto::Type::Sint32), None),
            ast::Ty::Sint64 => (Some(field_descriptor_proto::Type::Sint64), None),
            ast::Ty::Fixed32 => (Some(field_descriptor_proto::Type::Fixed32), None),
            ast::Ty::Fixed64 => (Some(field_descriptor_proto::Type::Fixed64), None),
            ast::Ty::Sfixed32 => (Some(field_descriptor_proto::Type::Sfixed32), None),
            ast::Ty::Sfixed64 => (Some(field_descriptor_proto::Type::Sfixed64), None),
            ast::Ty::Bool => (Some(field_descriptor_proto::Type::Bool), None),
            ast::Ty::String => (Some(field_descriptor_proto::Type::String), None),
            ast::Ty::Bytes => (Some(field_descriptor_proto::Type::Bytes), None),
            ast::Ty::Named(type_name) => match self.resolve_ast_type_name(type_name) {
                (name, None) => (None, Some(name)),
                (name, Some(DefinitionKind::Message)) => {
                    (Some(field_descriptor_proto::Type::Message as _), Some(name))
                }
                (name, Some(DefinitionKind::Enum)) => {
                    (Some(field_descriptor_proto::Type::Enum as _), Some(name))
                }
                (name, Some(DefinitionKind::Group)) => {
                    (Some(field_descriptor_proto::Type::Group as _), Some(name))
                }
                (name, Some(_)) => {
                    self.errors.push(CheckError::InvalidMessageFieldTypeName {
                        name: type_name.to_string(),
                        span: type_name.span(),
                    });
                    (None, Some(name))
                }
            },
        }
    }

    fn check_field_default_value(
        &mut self,
        field: &ast::Field,
        ty: Option<field_descriptor_proto::Type>,
    ) -> Option<String> {
        // TODO check type
        if let Some(option) = field.default_value() {
            if field.is_map() {
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
        match oneof.ast {
            ir::OneofSource::Oneof(oneof) => {
                let options = self.check_oneof_options(&oneof.options);

                OneofDescriptorProto {
                    name: s(&oneof.name.value),
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
        let (extendee, def) = self.resolve_ast_type_name(&extend.extendee);
        if def.is_some() && def != Some(&DefinitionKind::Message) {
            self.errors.push(CheckError::InvalidExtendeeTypeName {
                name: extend.extendee.to_string(),
                span: extend.extendee.span(),
            })
        }

        self.enter(Scope::Extend);
        result.extend(extend.fields.iter().map(|field| {
            let name = field.field_name();
            let json_name = Some(to_json_name(&name));
            let number = self.check_field_number(&field.number);
            let (ty, type_name) = self.check_type(&field.ty());
            FieldDescriptorProto {
                name: Some(name.into_owned()),
                json_name,
                number,
                r#type: ty.map(|ty| ty as i32),
                type_name,
                extendee: Some(extendee.clone()),
                ..self.check_field(field, ty)
            }
        }));
        self.exit();
    }

    fn check_enum(&mut self, e: &ast::Enum) -> EnumDescriptorProto {
        let name = s(&e.name.value);

        let value = e
            .values
            .iter()
            .map(|value| self.check_enum_value(value))
            .collect();

        let mut reserved_range = Vec::new();
        let mut reserved_name = Vec::new();
        for reserved in &e.reserved {
            match &reserved.kind {
                ast::ReservedKind::Ranges(ranges) => reserved_range.extend(
                    ranges
                        .iter()
                        .map(|range| self.check_enum_reserved_range(range)),
                ),
                ast::ReservedKind::Names(names) => {
                    reserved_name.extend(names.iter().map(|name| name.value.to_owned()))
                }
            }
        }

        let options = self.check_enum_options(&e.options);

        EnumDescriptorProto {
            name,
            value,
            reserved_name,
            reserved_range,
            options,
        }
    }

    fn check_enum_value(&mut self, value: &ast::EnumValue) -> EnumValueDescriptorProto {
        let name = s(&value.name.value);
        let number = self.check_enum_number(&value.number);

        let options = self.check_enum_value_options(value.options.as_ref());

        EnumValueDescriptorProto {
            name,
            number,
            options,
        }
    }

    fn check_service(&mut self, service: &ast::Service) -> ServiceDescriptorProto {
        let name = s(&service.name.value);

        let method = service
            .methods
            .iter()
            .map(|method| self.check_method(method))
            .collect();

        let options = self.check_service_options(&service.options);

        ServiceDescriptorProto {
            name,
            method,
            options,
        }
    }

    fn check_method(&mut self, method: &ast::Method) -> MethodDescriptorProto {
        let name = s(&method.name);

        let (input_type, kind) = self.resolve_ast_type_name(&method.input_ty);
        if !matches!(
            kind,
            None | Some(DefinitionKind::Message) | Some(DefinitionKind::Group)
        ) {
            self.errors.push(CheckError::InvalidMethodTypeName {
                name: method.input_ty.to_string(),
                kind: "input",
                span: method.input_ty.span(),
            })
        }

        let (output_type, kind) = self.resolve_ast_type_name(&method.output_ty);
        if !matches!(
            kind,
            None | Some(DefinitionKind::Message) | Some(DefinitionKind::Group)
        ) {
            self.errors.push(CheckError::InvalidMethodTypeName {
                name: method.output_ty.to_string(),
                kind: "output",
                span: method.output_ty.span(),
            })
        }

        let options = self.check_method_options(&method.options);

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

    fn check_message_extension_range(&mut self, range: &ast::ReservedRange) -> ExtensionRange {
        let start = self.check_field_number(&range.start);
        let end = match &range.end {
            ast::ReservedRangeEnd::None => start.map(|n| n + 1),
            ast::ReservedRangeEnd::Int(value) => self.check_field_number(value),
            ast::ReservedRangeEnd::Max(_) => Some(MAX_MESSAGE_FIELD_NUMBER + 1),
        };

        ExtensionRange {
            start,
            end,
            ..Default::default()
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

    fn check_file_options(&mut self, options: &[ast::Option]) -> Option<FileOptions> {
        if options.is_empty() {
            return None;
        }

        #[allow(clippy::all)]
        if self.name_map.is_some() {
            // build options set
        } else {
            // set uninterpreted option
        }
        // todo!()
        None
    }

    fn check_message_options(&mut self, options: &[ast::Option]) -> Option<MessageOptions> {
        if options.is_empty() {
            return None;
        }

        // todo!()
        None
    }

    fn check_field_options(&mut self, options: Option<&ast::OptionList>) -> Option<FieldOptions> {
        let _options = options?;

        // todo!()
        None
    }

    fn check_extension_range_options(
        &mut self,
        options: Option<&ast::OptionList>,
    ) -> Option<ExtensionRangeOptions> {
        let _options = options?;

        // todo!()
        None
    }

    fn check_oneof_options(&self, options: &[ast::Option]) -> Option<OneofOptions> {
        if options.is_empty() {
            return None;
        }

        // todo!()
        None
    }

    fn check_enum_options(&self, options: &[ast::Option]) -> Option<EnumOptions> {
        if options.is_empty() {
            return None;
        }

        // todo!()
        None
    }

    fn check_enum_value_options(
        &self,
        options: Option<&ast::OptionList>,
    ) -> Option<EnumValueOptions> {
        let _options = options?;

        // todo!()
        None
    }

    fn check_service_options(&self, options: &[ast::Option]) -> Option<ServiceOptions> {
        if options.is_empty() {
            return None;
        }

        // todo!()
        None
    }

    fn check_method_options(&self, options: &[ast::Option]) -> Option<MethodOptions> {
        if options.is_empty() {
            return None;
        }

        // todo!()
        None
    }
}
