use logos::Span;
use prost_types::UninterpretedOption;

use crate::{
    ast,
    case::{to_json_name, to_pascal_case},
    index_to_i32,
    lines::LineResolver,
    options::{self, OptionSet},
    tag,
    types::{
        descriptor_proto, enum_descriptor_proto, field_descriptor_proto,
        source_code_info::Location, uninterpreted_option, DescriptorProto, EnumDescriptorProto,
        EnumValueDescriptorProto, FieldDescriptorProto, FileDescriptorProto, MethodDescriptorProto,
        OneofDescriptorProto, ServiceDescriptorProto, SourceCodeInfo,
    },
};

use super::{CheckError, MAX_MESSAGE_FIELD_NUMBER, RESERVED_MESSAGE_FIELD_NUMBERS};

/// Convert the AST to a FileDescriptorProto, performing basic checks and generate group and map messages, and synthetic oneofs.
pub(crate) fn generate(
    ast: ast::File,
    lines: &LineResolver,
) -> Result<FileDescriptorProto, Vec<CheckError>> {
    let mut ctx = Context {
        syntax: ast.syntax,
        errors: vec![],
        path: vec![],
        locations: vec![],
        lines,
    };

    let file = ctx.generate_file_descriptor(ast);

    if ctx.errors.is_empty() {
        ctx.locations.sort_unstable_by(|l, r| l.path.cmp(&r.path));

        Ok(FileDescriptorProto {
            source_code_info: Some(SourceCodeInfo {
                location: ctx.locations,
            }),
            ..file
        })
    } else {
        Err(ctx.errors)
    }
}

struct Context<'a> {
    syntax: ast::Syntax,
    errors: Vec<CheckError>,
    path: Vec<i32>,
    locations: Vec<Location>,
    lines: &'a LineResolver,
}

enum FieldScope {
    Message,
    Oneof,
    Extend,
}

impl<'a> Context<'a> {
    fn generate_file_descriptor(&mut self, ast: ast::File) -> FileDescriptorProto {
        self.add_span(ast.span);

        let package = if let Some(package) = ast.package {
            self.add_comments_for(&[tag::file::PACKAGE], package.span, package.comments);
            Some(package.name.to_string())
        } else {
            None
        };

        let mut dependency = Vec::with_capacity(ast.imports.len());
        let mut public_dependency = Vec::new();
        let mut weak_dependency = Vec::new();
        for import in ast.imports {
            let index = index_to_i32(dependency.len());

            self.add_comments_for(
                &[tag::file::DEPENDENCY, index],
                import.span.clone(),
                import.comments,
            );

            dependency.push(import.value.clone());
            match &import.kind {
                Some((ast::ImportKind::Public, _)) => {
                    self.add_span_for(
                        &[
                            tag::file::PUBLIC_DEPENDENCY,
                            index_to_i32(public_dependency.len()),
                        ],
                        import.span,
                    );
                    public_dependency.push(index);
                }
                Some((ast::ImportKind::Weak, _)) => {
                    self.add_span_for(
                        &[
                            tag::file::WEAK_DEPENDENCY,
                            index_to_i32(public_dependency.len()),
                        ],
                        import.span,
                    );
                    weak_dependency.push(index);
                }
                _ => (),
            }
        }

        let mut message_type = Vec::new();
        let mut enum_type = Vec::new();
        let mut service = Vec::new();
        let mut extension = Vec::new();

        for item in ast.items {
            match item {
                ast::FileItem::Message(message_ast) => {
                    self.path
                        .extend(&[tag::file::MESSAGE_TYPE, index_to_i32(message_type.len())]);
                    message_type.push(self.generate_message_descriptor(message_ast));
                    self.pop_path(2);
                }
                ast::FileItem::Enum(service_ast) => {
                    self.path
                        .extend(&[tag::file::ENUM_TYPE, index_to_i32(enum_type.len())]);
                    enum_type.push(self.generate_enum_descriptor(service_ast));
                    self.pop_path(2);
                }
                ast::FileItem::Service(service_ast) => {
                    self.path
                        .extend(&[tag::file::SERVICE, index_to_i32(service.len())]);
                    service.push(self.generate_service_descriptor(service_ast));
                    self.pop_path(2);
                }
                ast::FileItem::Extend(extend_ast) => {
                    self.generate_extend_descriptors(
                        extend_ast,
                        tag::file::EXTENSION,
                        &mut extension,
                        tag::file::MESSAGE_TYPE,
                        &mut message_type,
                    );
                }
            }
        }

        self.path.push(tag::file::OPTIONS);
        let options = self.generate_options(ast.options);
        self.path.pop();

        if let Some((syntax_span, syntax_comments)) = ast.syntax_span {
            self.add_comments_for(&[tag::file::SYNTAX], syntax_span, syntax_comments);
        }
        let syntax = if ast.syntax == ast::Syntax::default() {
            None
        } else {
            Some(ast.syntax.to_string())
        };

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

    fn generate_message_descriptor(&mut self, ast: ast::Message) -> DescriptorProto {
        self.add_comments(ast.span, ast.comments);

        let name = Some(ast.name.value.to_string());
        self.add_span_for(&[tag::message::NAME], ast.name.span);

        DescriptorProto {
            name,
            ..self.generate_message_body_descriptor(ast.body)
        }
    }

    fn generate_message_body_descriptor(&mut self, ast: ast::MessageBody) -> DescriptorProto {
        let mut field = Vec::new();
        let mut extension = Vec::new();
        let mut nested_type = Vec::new();
        let mut enum_type = Vec::new();
        let mut oneof_decl = Vec::new();
        let mut extension_range = Vec::new();
        let mut reserved_range = Vec::new();
        let mut reserved_name = Vec::new();

        // Real oneofs must be ordered before any synthetic oneofs generated by fields
        let real_oneof_count = ast
            .items
            .iter()
            .filter(|item| matches!(item, ast::MessageItem::Oneof(_)))
            .count();
        oneof_decl.resize(real_oneof_count, OneofDescriptorProto::default());

        let mut real_oneof_index = 0;
        for item in ast.items {
            match item {
                ast::MessageItem::Field(field_ast) => {
                    field.push(self.generate_field_descriptor(
                        field_ast,
                        field.len(),
                        tag::message::FIELD,
                        tag::message::NESTED_TYPE,
                        &mut nested_type,
                        Some(tag::message::ONEOF_DECL),
                        &mut oneof_decl,
                        FieldScope::Message,
                    ));
                }
                ast::MessageItem::Enum(enum_ast) => {
                    self.path
                        .extend(&[tag::message::ENUM_TYPE, index_to_i32(enum_type.len())]);
                    enum_type.push(self.generate_enum_descriptor(enum_ast));
                    self.pop_path(2);
                }
                ast::MessageItem::Message(message_ast) => {
                    self.path
                        .extend(&[tag::message::NESTED_TYPE, index_to_i32(nested_type.len())]);
                    nested_type.push(self.generate_message_descriptor(message_ast));
                    self.pop_path(2);
                }
                ast::MessageItem::Extend(extend_ast) => {
                    self.generate_extend_descriptors(
                        extend_ast,
                        tag::message::EXTENSION,
                        &mut extension,
                        tag::message::NESTED_TYPE,
                        &mut nested_type,
                    );
                }
                ast::MessageItem::Oneof(oneof) => {
                    oneof_decl[real_oneof_index] = self.generate_oneof_descriptor(
                        oneof,
                        real_oneof_index,
                        tag::message::ONEOF_DECL,
                        tag::message::NESTED_TYPE,
                        &mut nested_type,
                        tag::message::FIELD,
                        &mut field,
                    );
                    real_oneof_index += 1;
                }
            }
        }

        for reserved in ast.reserved {
            match reserved.kind {
                ast::ReservedKind::Ranges(ranges) => {
                    self.path.push(tag::message::RESERVED_RANGE);
                    self.add_comments(reserved.span, reserved.comments);
                    for range in ranges {
                        self.path.push(index_to_i32(reserved_range.len()));
                        reserved_range.push(self.generate_message_reserved_range(range));
                        self.path.pop();
                    }
                    self.path.pop();
                }
                ast::ReservedKind::Names(names) => {
                    self.path.push(tag::message::RESERVED_NAME);
                    self.add_comments(reserved.span, reserved.comments);
                    for name in names {
                        self.path.push(index_to_i32(reserved_name.len()));
                        reserved_name.push(name.value);
                        self.path.pop();
                    }
                    self.path.pop();
                }
            }
        }

        self.path.push(tag::message::EXTENSION_RANGE);
        for extensions in ast.extensions {
            self.add_comments(extensions.span.clone(), extensions.comments.clone());

            for range in extensions.ranges {
                self.path.push(index_to_i32(extension_range.len()));
                extension_range
                    .push(self.generate_message_extension_range(range, extensions.options.clone()));
                self.path.pop();
            }
        }
        self.path.pop();

        self.path.push(tag::message::OPTIONS);
        let options = self.generate_options(ast.options);
        self.path.pop();

        DescriptorProto {
            name: None,
            field,
            extension,
            nested_type,
            enum_type,
            extension_range,
            oneof_decl,
            options,
            reserved_range,
            reserved_name,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn generate_field_descriptor(
        &mut self,
        ast: ast::Field,
        field_index: usize,
        field_tag: i32,
        message_tag: i32,
        messages: &mut Vec<DescriptorProto>,
        oneof_tag: Option<i32>,
        oneofs: &mut Vec<OneofDescriptorProto>,
        scope: FieldScope,
    ) -> FieldDescriptorProto {
        self.path.extend(&[field_tag, index_to_i32(field_index)]);
        self.add_span_for(&[tag::field::NAME], ast.name.span.clone());
        self.add_span_for(&[tag::field::NUMBER], ast.number.span.clone());
        let number = self.generate_message_number(ast.number.clone());
        if let Some(number) = number {
            if RESERVED_MESSAGE_FIELD_NUMBERS.contains(&number) {
                self.errors.push(CheckError::ReservedMessageNumber {
                    span: ast.number.span.clone(),
                });
            }
        }

        let (proto3_optional, oneof_index) = if self.syntax != ast::Syntax::Proto2
            && matches!(ast.label, Some((ast::FieldLabel::Optional, _)))
        {
            if let Some(oneof_tag) = oneof_tag {
                let oneof_name = if ast.name.value.starts_with('_') {
                    format!("X{}", &ast.name.value)
                } else {
                    format!("_{}", &ast.name.value)
                };

                let oneof_index = index_to_i32(oneofs.len());
                self.path.extend(&[oneof_tag, oneof_index]);
                oneofs.push(OneofDescriptorProto {
                    name: Some(oneof_name),
                    options: None,
                });
                self.pop_path(2);

                (Some(true), Some(oneof_index))
            } else {
                (Some(true), None)
            }
        } else {
            (None, None)
        };

        let name;
        let r#type;
        let type_name;
        let label;
        match ast.kind {
            ast::FieldKind::Normal {
                ty: ast::Ty::Named(ty),
                ..
            } => {
                name = ast.name.value;
                label = self.generate_field_label(ast.label, ast.span.clone(), scope);
                r#type = None;
                type_name = Some(ty.to_string());

                self.add_comments(ast.span, ast.comments);
                self.add_span_for(&[tag::field::TYPE_NAME], ty.span());
            }
            ast::FieldKind::Normal { ty, ty_span } => {
                name = ast.name.value;
                label = self.generate_field_label(ast.label, ast.span.clone(), scope);
                r#type = ty.proto_ty();
                type_name = None;

                self.add_comments(ast.span, ast.comments);
                self.add_span_for(&[tag::field::TYPE], ty_span);
            }
            ast::FieldKind::Group { ty_span, body } => {
                name = ast.name.value.to_ascii_lowercase();
                label = self.generate_field_label(ast.label, ast.span.clone(), scope);
                r#type = Some(field_descriptor_proto::Type::Group);
                type_name = Some(ast.name.value);

                if self.syntax != ast::Syntax::Proto2 {
                    self.errors.push(CheckError::Proto3GroupField {
                        span: ast.span.clone(),
                    });
                }

                self.add_span(ast.span.clone());
                self.add_span_for(&[tag::field::TYPE], ty_span);
                self.add_span_for(&[tag::field::TYPE_NAME], ast.name.span.clone());

                self.pop_path(2);
                self.path
                    .extend(&[message_tag, index_to_i32(messages.len())]);
                self.add_comments(ast.span, ast.comments);
                self.add_span_for(&[tag::message::NAME], ast.name.span);
                messages.push(DescriptorProto {
                    name: type_name.clone(),
                    ..self.generate_message_body_descriptor(body)
                });
                self.pop_path(2);
                self.path.extend(&[field_tag, index_to_i32(field_index)]);
            }
            ast::FieldKind::Map {
                ty_span,
                key_ty,
                key_ty_span,
                value_ty,
                ..
            } => {
                name = ast.name.value;
                label = Some(field_descriptor_proto::Label::Repeated);
                r#type = Some(field_descriptor_proto::Type::Message);
                type_name = Some(to_pascal_case(&name) + "Entry");

                match scope {
                    FieldScope::Oneof => {
                        self.errors.push(CheckError::InvalidOneofFieldKind {
                            kind: "map",
                            span: ast.span.clone(),
                        });
                    }
                    FieldScope::Extend => {
                        self.errors.push(CheckError::InvalidExtendFieldKind {
                            kind: "map",
                            span: ast.span.clone(),
                        });
                    }
                    FieldScope::Message => {
                        if let Some((_, span)) = ast.label {
                            self.errors.push(CheckError::MapFieldWithLabel { span });
                        }
                    }
                }

                self.add_comments(ast.span, ast.comments);
                self.add_span_for(&[tag::field::TYPE_NAME], ty_span);

                if !matches!(
                    key_ty,
                    ast::Ty::Double
                        | ast::Ty::Float
                        | ast::Ty::Int32
                        | ast::Ty::Int64
                        | ast::Ty::Uint32
                        | ast::Ty::Uint64
                        | ast::Ty::Sint32
                        | ast::Ty::Sint64
                        | ast::Ty::Fixed32
                        | ast::Ty::Fixed64
                        | ast::Ty::Sfixed32
                        | ast::Ty::Sfixed64
                        | ast::Ty::Bool
                        | ast::Ty::String
                ) {
                    self.errors
                        .push(CheckError::InvalidMapFieldKeyType { span: key_ty_span });
                };

                messages.push(DescriptorProto {
                    name: type_name.clone(),
                    field: vec![
                        FieldDescriptorProto {
                            name: Some("key".to_owned()),
                            json_name: Some("key".to_owned()),
                            label: Some(field_descriptor_proto::Label::Optional as _),
                            number: Some(1),
                            r#type: key_ty.proto_ty().map(|t| t as _),
                            ..Default::default()
                        },
                        FieldDescriptorProto {
                            name: Some("value".to_owned()),
                            json_name: Some("value".to_owned()),
                            label: Some(field_descriptor_proto::Label::Optional as _),
                            number: Some(2),
                            r#type: value_ty.proto_ty().map(|t| t as _),
                            type_name: value_ty.ty_name(),
                            ..Default::default()
                        },
                    ],
                    options: Some({
                        let mut options = OptionSet::new();
                        options
                            .set(options::MESSAGE_MAP_ENTRY, options::Value::Bool(true))
                            .unwrap();
                        options
                    }),
                    ..Default::default()
                });
            }
        }

        let json_name = Some(to_json_name(&name));

        self.path.push(tag::field::OPTIONS);
        let options = self.generate_options_list(ast.options);
        self.pop_path(3);

        FieldDescriptorProto {
            name: Some(name),
            number,
            label: label.map(|l| l as _),
            r#type: r#type.map(|t| t as _),
            type_name,
            extendee: None,
            default_value: None,
            oneof_index,
            json_name,
            options,
            proto3_optional,
        }
    }

    fn generate_field_label(
        &mut self,
        label: Option<(ast::FieldLabel, Span)>,
        field_span: Span,
        scope: FieldScope,
    ) -> Option<field_descriptor_proto::Label> {
        match (scope, label) {
            (FieldScope::Extend, Some((ast::FieldLabel::Required, span))) => {
                self.errors.push(CheckError::RequiredExtendField { span });
                None
            }
            (FieldScope::Oneof, Some((_, span))) => {
                self.errors.push(CheckError::OneofFieldWithLabel { span });
                None
            }
            (FieldScope::Message | FieldScope::Extend, None)
                if self.syntax == ast::Syntax::Proto2 =>
            {
                self.errors
                    .push(CheckError::Proto2FieldMissingLabel { span: field_span });
                None
            }
            (_, Some((ast::FieldLabel::Required, span))) if self.syntax == ast::Syntax::Proto3 => {
                self.errors.push(CheckError::Proto3RequiredField { span });
                None
            }
            (_, Some((ast::FieldLabel::Required, span))) => {
                self.add_span_for(&[tag::field::LABEL], span);
                Some(field_descriptor_proto::Label::Required)
            }
            (_, Some((ast::FieldLabel::Repeated, span))) => {
                self.add_span_for(&[tag::field::LABEL], span);
                Some(field_descriptor_proto::Label::Repeated)
            }
            (_, Some((ast::FieldLabel::Optional, span))) => {
                self.add_span_for(&[tag::field::LABEL], span);
                Some(field_descriptor_proto::Label::Optional)
            }
            (_, None) => Some(field_descriptor_proto::Label::Optional),
        }
    }

    fn generate_message_number(&mut self, ast: ast::Int) -> Option<i32> {
        match ast.as_i32() {
            Some(number @ 1..=MAX_MESSAGE_FIELD_NUMBER) => Some(number),
            _ => {
                self.errors
                    .push(CheckError::InvalidMessageNumber { span: ast.span });
                None
            }
        }
    }

    fn generate_message_reserved_range(
        &mut self,
        range: ast::ReservedRange,
    ) -> descriptor_proto::ReservedRange {
        let start = self.generate_message_number(range.start);
        let end = match range.end {
            ast::ReservedRangeEnd::None => start,
            ast::ReservedRangeEnd::Int(value) => self.generate_message_number(value),
            ast::ReservedRangeEnd::Max(_) => Some(MAX_MESSAGE_FIELD_NUMBER),
        };

        descriptor_proto::ReservedRange {
            start,
            end: end.map(|n| n + 1),
        }
    }

    fn generate_message_extension_range(
        &mut self,
        range: ast::ReservedRange,
        options: Option<ast::OptionList>,
    ) -> descriptor_proto::ExtensionRange {
        self.add_span(range.span());
        self.add_span_for(&[tag::message::extension_range::START], range.start_span());
        self.add_span_for(&[tag::message::extension_range::END], range.end_span());

        self.path.push(tag::message::extension_range::OPTIONS);
        let options = self.generate_options_list(options);
        self.path.pop();

        let start = self.generate_message_number(range.start);
        let end = match range.end {
            ast::ReservedRangeEnd::None => start,
            ast::ReservedRangeEnd::Int(value) => self.generate_message_number(value),
            ast::ReservedRangeEnd::Max(_) => Some(MAX_MESSAGE_FIELD_NUMBER),
        };

        descriptor_proto::ExtensionRange {
            start,
            end: end.map(|n| n + 1),
            options,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn generate_oneof_descriptor(
        &mut self,
        oneof: ast::Oneof,
        oneof_index: usize,
        oneof_tag: i32,
        message_tag: i32,
        messages: &mut Vec<DescriptorProto>,
        field_tag: i32,
        fields: &mut Vec<FieldDescriptorProto>,
    ) -> OneofDescriptorProto {
        self.path.extend(&[oneof_tag, index_to_i32(oneof_index)]);
        self.add_span(oneof.span.clone());
        self.add_span_for(&[tag::oneof::NAME], oneof.name.span);

        self.path.push(tag::oneof::OPTIONS);
        let options = self.generate_options(oneof.options);
        self.path.pop();
        self.pop_path(2);

        for field_ast in oneof.fields {
            fields.push(FieldDescriptorProto {
                oneof_index: Some(index_to_i32(oneof_index)),
                ..self.generate_field_descriptor(
                    field_ast,
                    fields.len(),
                    field_tag,
                    message_tag,
                    messages,
                    None,
                    &mut Vec::new(),
                    FieldScope::Oneof,
                )
            });
        }

        OneofDescriptorProto {
            name: Some(oneof.name.value),
            options,
        }
    }

    fn generate_extend_descriptors(
        &mut self,
        ast: ast::Extend,
        extension_tag: i32,
        extensions: &mut Vec<FieldDescriptorProto>,
        message_tag: i32,
        messages: &mut Vec<DescriptorProto>,
    ) {
        self.path.push(extension_tag);
        self.add_comments(ast.span, ast.comments);
        self.path.pop();

        for field_ast in ast.fields {
            self.path
                .extend(&[extension_tag, index_to_i32(extensions.len())]);
            self.add_span_for(&[tag::field::EXTENDEE], ast.extendee.span());
            self.pop_path(2);

            extensions.push(FieldDescriptorProto {
                extendee: Some(ast.extendee.to_string()),
                ..self.generate_field_descriptor(
                    field_ast,
                    extensions.len(),
                    extension_tag,
                    message_tag,
                    messages,
                    None,
                    &mut Vec::new(),
                    FieldScope::Extend,
                )
            });
        }
    }

    fn generate_enum_descriptor(&mut self, ast: ast::Enum) -> EnumDescriptorProto {
        self.add_comments(ast.span, ast.comments);
        self.add_span_for(&[tag::enum_::NAME], ast.name.span);

        let name = Some(ast.name.value);
        let mut value = Vec::new();
        let mut reserved_range = Vec::new();
        let mut reserved_name = Vec::new();

        for value_ast in ast.values {
            self.path
                .extend(&[tag::enum_::VALUE, index_to_i32(value.len())]);
            value.push(self.generate_enum_value_descriptor(value_ast));
            self.pop_path(2);
        }

        for reserved in ast.reserved {
            match reserved.kind {
                ast::ReservedKind::Ranges(ranges) => {
                    self.path.push(tag::enum_::RESERVED_RANGE);
                    self.add_comments(reserved.span, reserved.comments);
                    for range in ranges {
                        self.path.push(index_to_i32(reserved_range.len()));
                        reserved_range.push(self.generate_enum_reserved_range(range));
                        self.path.pop();
                    }
                    self.path.pop();
                }
                ast::ReservedKind::Names(names) => {
                    self.path.push(tag::enum_::RESERVED_NAME);
                    self.add_comments(reserved.span, reserved.comments);
                    for name in names {
                        self.path.push(index_to_i32(reserved_name.len()));
                        reserved_name.push(name.value);
                        self.path.pop();
                    }
                    self.path.pop();
                }
            }
        }

        self.path.push(tag::enum_::OPTIONS);
        let options = self.generate_options(ast.options);
        self.path.pop();

        EnumDescriptorProto {
            name,
            value,
            options,
            reserved_range,
            reserved_name,
        }
    }

    fn generate_enum_value_descriptor(&mut self, ast: ast::EnumValue) -> EnumValueDescriptorProto {
        self.add_comments(ast.span, ast.comments);
        self.add_span_for(&[tag::enum_value::NAME], ast.name.span);
        let name = Some(ast.name.value);

        self.add_span_for(&[tag::enum_value::NUMBER], ast.number.span.clone());
        let number = self.generate_enum_number(ast.number);

        self.path.push(tag::enum_value::OPTIONS);
        let options = self.generate_options_list(ast.options);
        self.path.pop();

        EnumValueDescriptorProto {
            name,
            number,
            options,
        }
    }

    fn generate_enum_number(&mut self, ast: ast::Int) -> Option<i32> {
        match ast.as_i32() {
            Some(number) => Some(number),
            None => {
                self.errors
                    .push(CheckError::InvalidEnumNumber { span: ast.span });
                None
            }
        }
    }

    fn generate_enum_reserved_range(
        &mut self,
        range: ast::ReservedRange,
    ) -> enum_descriptor_proto::EnumReservedRange {
        let start = self.generate_enum_number(range.start);
        let end = match range.end {
            ast::ReservedRangeEnd::None => start,
            ast::ReservedRangeEnd::Int(value) => self.generate_enum_number(value),
            ast::ReservedRangeEnd::Max(_) => Some(i32::MAX),
        };

        enum_descriptor_proto::EnumReservedRange { start, end }
    }

    fn generate_service_descriptor(&mut self, service: ast::Service) -> ServiceDescriptorProto {
        self.add_comments(service.span, service.comments);
        self.add_span_for(&[tag::service::NAME], service.name.span);
        let name = Some(service.name.value);
        let mut method = Vec::new();

        self.path.push(tag::service::METHOD);
        for method_ast in service.methods {
            self.path.push(index_to_i32(method.len()));
            method.push(self.generate_method_descriptor(method_ast));
            self.path.pop();
        }

        self.path.push(tag::service::OPTIONS);
        let options = self.generate_options(service.options);
        self.path.pop();

        ServiceDescriptorProto {
            name,
            method,
            options,
        }
    }

    fn generate_method_descriptor(&mut self, ast: ast::Method) -> MethodDescriptorProto {
        self.add_comments(ast.span, ast.comments);
        self.add_span_for(&[tag::method::NAME], ast.name.span);
        let name = Some(ast.name.value);

        self.add_span_for(&[tag::method::INPUT_TYPE], ast.input_ty.span());
        let input_type = ast.input_ty.to_string();

        self.add_span_for(&[tag::method::OUTPUT_TYPE], ast.output_ty.span());
        let output_type = ast.output_ty.to_string();

        let client_streaming = ast.client_streaming.is_some();
        if let Some(span) = ast.client_streaming {
            self.add_span_for(&[tag::method::CLIENT_STREAMING], span);
        }
        let server_streaming = ast.server_streaming.is_some();
        if let Some(span) = ast.server_streaming {
            self.add_span_for(&[tag::method::SERVER_STREAMING], span);
        }

        self.path.push(tag::method::OPTIONS);
        let options = self.generate_options(ast.options);
        self.path.pop();

        MethodDescriptorProto {
            name,
            input_type: Some(input_type),
            output_type: Some(output_type),
            options,
            client_streaming: Some(client_streaming),
            server_streaming: Some(server_streaming),
        }
    }

    fn generate_options(&mut self, ast: Vec<ast::Option>) -> Option<OptionSet> {
        let mut options = Vec::new();

        for option_ast in ast {
            self.add_span(option_ast.span.clone());
            self.add_comments_for(
                &[tag::UNINTERPRETED_OPTION, index_to_i32(options.len())],
                option_ast.span,
                option_ast.comments,
            );
            options.push(self.generate_option(option_ast.body));
        }

        if options.is_empty() {
            None
        } else {
            Some(OptionSet::uninterpreted(options))
        }
    }

    fn generate_options_list(&mut self, ast: Option<ast::OptionList>) -> Option<OptionSet> {
        let mut options = Vec::new();

        if let Some(ast) = ast {
            self.add_span(ast.span);

            for option_ast in ast.options {
                self.add_span_for(
                    &[tag::UNINTERPRETED_OPTION, index_to_i32(options.len())],
                    option_ast.span(),
                );
                options.push(self.generate_option(option_ast));
            }
        }

        if options.is_empty() {
            None
        } else {
            Some(OptionSet::uninterpreted(options))
        }
    }

    fn generate_option(&mut self, ast: ast::OptionBody) -> UninterpretedOption {
        let mut name = Vec::new();
        for part in ast.name {
            match part {
                ast::OptionNamePart::Ident(ident) => name.push(uninterpreted_option::NamePart {
                    name_part: ident.value,
                    is_extension: false,
                }),
                ast::OptionNamePart::Extension(extension, _) => {
                    name.push(uninterpreted_option::NamePart {
                        name_part: extension.to_string(),
                        is_extension: true,
                    })
                }
            }
        }

        match ast.value {
            ast::OptionValue::Ident(ident) => UninterpretedOption {
                name,
                identifier_value: Some(ident.value),
                ..Default::default()
            },
            ast::OptionValue::Int(int) => {
                if int.negative {
                    let negative_int_value = int.as_i64();
                    if negative_int_value.is_none() {
                        self.errors.push(CheckError::IntegerValueOutOfRange {
                            expected: "a 64-bit integer".to_owned(),
                            actual: int.to_string(),
                            min: i64::MIN.to_string(),
                            max: u64::MAX.to_string(),
                            span: int.span,
                        })
                    }

                    UninterpretedOption {
                        name,
                        negative_int_value,
                        ..Default::default()
                    }
                } else {
                    UninterpretedOption {
                        name,
                        positive_int_value: Some(int.value),
                        ..Default::default()
                    }
                }
            }
            ast::OptionValue::Float(float) => UninterpretedOption {
                name,
                double_value: Some(float.value),
                ..Default::default()
            },
            ast::OptionValue::String(string) => UninterpretedOption {
                name,
                string_value: Some(string.value),
                ..Default::default()
            },
            ast::OptionValue::Aggregate(message, _) => UninterpretedOption {
                name,
                aggregate_value: Some(message.to_string()),
                ..Default::default()
            },
        }
    }

    fn add_span(&mut self, span: Span) {
        let span = self.lines.resolve_span(span);
        self.locations.push(Location {
            path: self.path.clone(),
            span,
            ..Default::default()
        });
    }

    fn add_comments(&mut self, span: Span, comments: ast::Comments) {
        let span = self.lines.resolve_span(span);
        self.locations.push(Location {
            path: self.path.clone(),
            span,
            leading_comments: comments.leading_comment,
            trailing_comments: comments.trailing_comment,
            leading_detached_comments: comments.leading_detached_comments,
        });
    }

    fn add_span_for(&mut self, path_items: &[i32], span: Span) {
        self.path.extend_from_slice(path_items);
        self.add_span(span);
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
}
