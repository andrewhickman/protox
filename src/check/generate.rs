use logos::Span;

use crate::{
    ast,
    case::{to_json_name, to_pascal_case},
    index_to_i32,
    lines::LineResolver,
    options::{OptionSet, self},
    tag,
    types::{descriptor_proto, field_descriptor_proto, FileDescriptorProto, OneofDescriptorProto},
    types::{
        source_code_info::Location, DescriptorProto, EnumDescriptorProto, FieldDescriptorProto,
        ServiceDescriptorProto, SourceCodeInfo,
    },
};

use super::CheckError;

pub(crate) fn generate(
    ast: ast::File,
    lines: &LineResolver,
) -> Result<FileDescriptorProto, Vec<CheckError>> {
    let mut ctx = Context {
        errors: vec![],
        path: vec![],
        locations: vec![],
        lines,
    };

    let file = ctx.generate_file_descriptor(ast);

    if ctx.errors.is_empty() {
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
    errors: Vec<CheckError>,
    path: Vec<i32>,
    locations: Vec<Location>,
    lines: &'a LineResolver,
}

impl<'a> Context<'a> {
    fn generate_file_descriptor(&mut self, ast: ast::File) -> FileDescriptorProto {
        self.add_location(ast.span);

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
                    self.add_location_for(
                        &[
                            tag::file::PUBLIC_DEPENDENCY,
                            index_to_i32(public_dependency.len()),
                        ],
                        import.span,
                    );
                    public_dependency.push(index);
                }
                Some((ast::ImportKind::Weak, _)) => {
                    self.add_location_for(
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
        self.add_location_for(&[tag::message::NAME], ast.name.span);

        DescriptorProto {
            name,
            ..self.generate_message_body_descriptor(ast.body)
        }
    }

    fn generate_message_body_descriptor(&mut self, mut ast: ast::MessageBody) -> DescriptorProto {
        let mut field = Vec::new();
        let mut extension = Vec::new();
        let mut nested_type = Vec::new();
        let mut enum_type = Vec::new();
        let mut oneof_decl = Vec::new();
        let mut extension_range = Vec::new();
        let mut reserved_range = Vec::new();
        let mut reserved_name = Vec::new();

        // Real oneofs must be ordered before any synthetic oneofs generated by fields
        self.path.extend(&[tag::message::ONEOF_DECL, 0]);
        for oneof in ast.drain_oneofs() {
            self.replace_path(&[index_to_i32(oneof_decl.len())]);
            oneof_decl.push(self.generate_oneof_descriptor(oneof));
        }
        self.pop_path(2);

        for item in ast.items {
            match item {
                ast::MessageItem::Field(field_ast) => {
                    self.generate_field_descriptor(
                        field_ast,
                        tag::message::FIELD,
                        &mut field,
                        tag::message::NESTED_TYPE,
                        &mut nested_type,
                        tag::message::ONEOF_DECL,
                        &mut oneof_decl,
                    );
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
                ast::MessageItem::Oneof(_) => unreachable!(),
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

        self.path.push(tag::file::OPTIONS);
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

    fn generate_field_descriptor(
        &self,
        mut ast: ast::Field,
        field_tag: i32,
        fields: &mut Vec<FieldDescriptorProto>,
        message_tag: i32,
        messages: &mut Vec<DescriptorProto>,
        oneof_tag: i32,
        oneofs: &mut Vec<OneofDescriptorProto>,
    ) {
        let number = self.generate_message_number(ast.number);

        let name;
        let r#type;
        let type_name;
        let proto3_optional;
        let label: Option<field_descriptor_proto::Label> = todo!();
        match ast.kind {
            ast::FieldKind::Normal {
                ty: ast::Ty::Named(ty),
                ty_span,
            } => {
                name = ast.name.value;
                r#type = None;
                type_name = Some(ty.to_string());

                self.add_comments(ast.span, ast.comments);
                self.add_location_for(&[tag::field::NAME], ast.name.span);
                self.add_location_for(&[tag::field::TYPE_NAME], ty.span());
            }
            ast::FieldKind::Normal { ty, ty_span } => {
                name = ast.name.value;
                r#type = ty.proto_ty();
                type_name = None;

                self.add_comments(ast.span, ast.comments);
                self.add_location_for(&[tag::field::NAME], ast.name.span);
                self.add_location_for(&[tag::field::TYPE], ty_span);
            }
            ast::FieldKind::Group { ty_span, body } => {
                name = ast.name.value.to_ascii_lowercase();
                r#type = Some(field_descriptor_proto::Type::Group);
                type_name = Some(ast.name.value);

                self.add_location_for(&[tag::field::TYPE], ty_span);
                self.add_location_for(&[tag::field::TYPE_NAME], ast.name.span);

                self.path.extend(&[message_tag, index_to_i32(messages.len())]);
                self.add_comments(ast.span, ast.comments);
                self.add_location_for(&[tag::message::NAME], ast.name.span);
                messages.push(DescriptorProto {
                    name: type_name.clone(),
                    ..self.generate_message_body_descriptor(body)
                });
                self.pop_path(2);
            }
            ast::FieldKind::Map {
                ty_span,
                key_ty,
                key_ty_span,
                value_ty,
                value_ty_span,
            } => {
                name = ast.name.value;
                r#type = Some(field_descriptor_proto::Type::Message);
                type_name = Some(to_pascal_case(&name) + "Entry");

                self.add_comments(ast.span, ast.comments);
                self.add_location_for(&[tag::field::NAME], ast.name.span);
                self.add_location_for(&[tag::field::TYPE_NAME], ty_span);

                // TODO check key type

                messages.push(DescriptorProto {
                    name: type_name.clone(),
                    field: vec![
                        FieldDescriptorProto {
                            name: Some("key".to_owned()),
                            json_name: Some("key".to_owned()),
                            number: Some(1),
                            r#type: key_ty.proto_ty().map(|t| t as i32),
                            ..Default::default()
                        },
                        FieldDescriptorProto {
                            name: Some("value".to_owned()),
                            json_name: Some("value".to_owned()),
                            number: Some(2),
                            r#type: value_ty.proto_ty().map(|t| t as i32),
                            type_name: value_ty.ty_name(),
                            ..Default::default()
                        },
                    ],
                    options: Some({
                        let mut options = OptionSet::new();
                        options.set(options::MESSAGE_MAP_ENTRY, options::Value::Bool(true));
                        options
                    }),
                    ..Default::default()
                });
            }
        }

        let json_name = Some(to_json_name(&name));

        let default_value = match ast.take_default_value() {
            Some(default) => {
                self.add_location_for(&[tag::field::DEFAULT_VALUE], default.value.span());
                Some(default.value.to_string())
            }
            None => None,
        };

        self.path.push(tag::file::OPTIONS);
        let options = self.generate_options_list(ast.options);
        self.path.pop();

        if proto3_optional == Some(true) {
            self.path.extend(&[oneof_tag, index_to_i32(oneofs.len())]);
            oneofs.push(OneofDescriptorProto {
                name: Some(ast.synthetic_oneof_name()),
                options: None,
            });
            self.pop_path(2);
        }

        self.path.extend(&[field_tag, index_to_i32(fields.len())]);
        fields.push(FieldDescriptorProto {
            name: Some(name),
            number,
            label: label.map(|l| l as i32),
            r#type: r#type.map(|t| t as i32),
            type_name,
            extendee: None,
            default_value,
            oneof_index: None,
            json_name,
            options,
            proto3_optional,
        });
        self.pop_path(2);
    }

    fn generate_message_number(&self, number: ast::Int) -> Option<i32> {
        todo!()
    }

    fn generate_message_reserved_range(
        &self,
        range: ast::ReservedRange,
    ) -> descriptor_proto::ReservedRange {
        todo!()
    }

    fn generate_message_extension_range(
        &self,
        range: ast::ReservedRange,
        options: Option<ast::OptionList>,
    ) -> descriptor_proto::ExtensionRange {
        todo!()
    }

    fn generate_oneof_descriptor(&mut self, oneof: ast::Oneof) -> OneofDescriptorProto {
        todo!()
    }

    fn generate_extend_descriptors(
        &mut self,
        ast: ast::Extend,
        extension_tag: i32,
        extensions: &mut Vec<FieldDescriptorProto>,
        message_tag: i32,
        messages: &mut Vec<DescriptorProto>,
    ) {
        for field_ast in ast.fields {
            let mut oneofs = Vec::new();
            self.generate_field_descriptor(field_ast, extension_tag, extensions, message_tag, messages, 0, &mut oneofs);
            debug_assert!(oneofs.is_empty());
        }
    }

    fn generate_enum_descriptor(&mut self, enum_: ast::Enum) -> EnumDescriptorProto {
        todo!()
    }

    fn generate_service_descriptor(&mut self, service: ast::Service) -> ServiceDescriptorProto {
        todo!()
    }

    fn generate_options(&mut self, options: Vec<ast::Option>) -> Option<OptionSet> {
        todo!()
    }

    fn generate_options_list(&mut self, options: Option<ast::OptionList>) -> Option<OptionSet> {
        todo!()
    }

    fn add_location(&mut self, span: Span) {
        // TODO maintain sort order by path_itemd
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
}
