impl ast::Field {
    fn to_field_descriptor(&self, ctx: &mut Context) -> FieldDescriptorProto {
    }
}

impl ast::Int {
    fn to_field_number(&self, ctx: &mut Context) -> Option<i32> {
        match (self.negative, i32::try_from(self.value)) {
            (false, Ok(number @ 1..=MAX_MESSAGE_FIELD_NUMBER)) => Some(number),
            _ => {
                ctx.errors.push(CheckError::InvalidMessageNumber {
                    span: self.span.clone(),
                });
                None
            }
        }
    }

    fn to_enum_number(&self, ctx: &mut Context) -> Option<i32> {
        let as_i32 = if self.negative {
            self.value.checked_neg().and_then(|n| i32::try_from(n).ok())
        } else {
            i32::try_from(self.value).ok()
        };

        if as_i32.is_none() {
            ctx.errors.push(CheckError::InvalidEnumNumber {
                span: self.span.clone(),
            });
        }

        as_i32
    }
}

impl ast::Map {
    fn to_field_descriptor(
        &self,
        ctx: &mut Context,
        messages: &mut Vec<DescriptorProto>,
    ) -> FieldDescriptorProto {
        let name = s(&self.name.value);
        let number = self.number.to_field_number(ctx);

        let generated_message = self.generate_message_descriptor(ctx);
        let r#type = Some(field_descriptor_proto::Type::Message as i32);
        let (type_name, def) = ctx.resolve_relative_type_name(
            generated_message.name().to_owned(),
            self.name.span.clone(),
        );
        debug_assert_eq!(def, Some(DefinitionKind::Message));
        messages.push(generated_message);

        let (default_value, options) = if self.options.is_empty() {
            (None, None)
        } else {
            let (default_value, options) = ast::OptionBody::to_field_options(&self.options);
            (default_value, Some(options))
        };

        if self.label.is_some() {
            ctx.errors.push(CheckError::MapFieldWithLabel {
                span: self.span.clone(),
            });
        }

        if default_value.is_some() {
            ctx.errors.push(CheckError::InvalidDefault {
                kind: "map",
                span: self.span.clone(),
            });
        }

        let json_name = Some(to_camel_case(&self.name.value));

        FieldDescriptorProto {
            name,
            number,
            label: Some(field_descriptor_proto::Label::Repeated as _),
            r#type,
            type_name: Some(type_name),
            extendee: ctx.parent_extendee(),
            default_value: None,
            oneof_index: ctx.parent_oneof(),
            json_name,
            options,
            proto3_optional: None,
        }
    }

    fn generate_message_descriptor(&self, ctx: &mut Context) -> DescriptorProto {
        let name = Some(to_pascal_case(&self.name.value) + "Entry");

        let (ty, type_name) = self.ty.to_type(ctx);


        DescriptorProto {
            name,
            field: vec![key_field, value_field],
            options: Some(MessageOptions {
                map_entry: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

impl ast::Group {
    fn to_field_descriptor(
        &self,
        ctx: &mut Context,
        messages: &mut Vec<DescriptorProto>,
    ) -> FieldDescriptorProto {
        let field_name = Some(self.name.value.to_ascii_lowercase());
        let message_name = Some(self.name.value.clone());

        let json_name = Some(to_camel_case(&self.name.value));
        let number = self.number.to_field_number(ctx);
        let label = Some(
            self.label
                .unwrap_or(ast::FieldLabel::Optional)
                .to_field_label() as i32,
        );

        let (default_value, options) = if self.options.is_empty() {
            (None, None)
        } else {
            let (default_value, options) = ast::OptionBody::to_field_options(&self.options);
            (default_value, Some(options))
        };

        if ctx.syntax == ast::Syntax::Proto3 {
            ctx.errors.push(CheckError::Proto3GroupField {
                span: self.span.clone(),
            });
        } else {
            ctx.check_label(self.label, self.span.clone());
        }

        if default_value.is_some() {
            ctx.errors.push(CheckError::InvalidDefault {
                kind: "group",
                span: self.span.clone(),
            });
        }

        ctx.enter(Definition::Group);

        let generated_message = DescriptorProto {
            name: message_name,
            ..self.body.to_message_descriptor(ctx)
        };
        ctx.exit();

        let r#type = Some(field_descriptor_proto::Type::Group as i32);
        let (type_name, def) = ctx.resolve_relative_type_name(
            generated_message.name().to_owned(),
            self.name.span.clone(),
        );
        debug_assert_eq!(def, Some(DefinitionKind::Group));
        messages.push(generated_message);

        FieldDescriptorProto {
            name: field_name,
            number,
            label,
            r#type,
            type_name: Some(type_name),
            extendee: ctx.parent_extendee(),
            default_value: None,
            oneof_index: ctx.parent_oneof(),
            json_name,
            options,
            proto3_optional: None,
        }
    }
}

impl ast::Extend {
    fn to_field_descriptors(
        &self,
        ctx: &mut Context,
        messages: &mut Vec<DescriptorProto>,
        fields: &mut Vec<FieldDescriptorProto>,
    ) {
        let (extendee, kind) = ctx.resolve_type_name(&self.extendee);
        if !matches!(
            kind,
            None | Some(DefinitionKind::Message) | Some(DefinitionKind::Group)
        ) {
            ctx.errors.push(CheckError::InvalidExtendeeTypeName {
                name: self.extendee.to_string(),
                span: self.extendee.span(),
            });
        }
        ctx.enter(Definition::Extend { extendee });

        for field in &self.fields {
            let mut oneofs = Vec::new();
            field.to_field_descriptors(ctx, messages, fields, &mut oneofs);
            debug_assert_eq!(oneofs, vec![]);
        }
        ctx.exit();
    }
}

impl ast::Oneof {
    fn to_oneof_descriptor(
        &self,
        ctx: &mut Context,
        messages: &mut Vec<DescriptorProto>,
        fields: &mut Vec<FieldDescriptorProto>,
        index: usize,
    ) -> OneofDescriptorProto {
        ctx.enter(Definition::Oneof {
            index: index_to_i32(index),
        });

        let name = s(&self.name.value);

        for field in &self.fields {
            let mut oneofs = Vec::new();
            field.to_field_descriptors(ctx, messages, fields, &mut oneofs);
            debug_assert_eq!(oneofs, vec![]);
        }

        let options = if self.options.is_empty() {
            None
        } else {
            Some(ast::Option::to_oneof_options(&self.options))
        };

        ctx.exit();
        OneofDescriptorProto { name, options }
    }
}

impl ast::Enum {
    fn to_enum_descriptor(&self, ctx: &mut Context) -> EnumDescriptorProto {
        ctx.enter(Definition::Enum);

        let name = s(&self.name.value);

        let value = self
            .values
            .iter()
            .map(|v| v.to_enum_value_descriptor(ctx))
            .collect();

        let options = if self.options.is_empty() {
            None
        } else {
            Some(ast::Option::to_enum_options(&self.options))
        };

        let mut reserved_range = Vec::new();
        let mut reserved_name = Vec::new();

        for r in &self.reserved {
            match &r.kind {
                ast::ReservedKind::Ranges(ranges) => {
                    reserved_range.extend(ranges.iter().map(|r| r.to_enum_reserved_range(ctx)))
                }
                ast::ReservedKind::Names(names) => {
                    reserved_name.extend(names.iter().map(|n| n.value.clone()))
                }
            }
        }

        ctx.exit();
        EnumDescriptorProto {
            name,
            value,
            options,
            reserved_range,
            reserved_name,
        }
    }
}

impl ast::EnumValue {
    fn to_enum_value_descriptor(&self, ctx: &mut Context) -> EnumValueDescriptorProto {
        let name = s(&self.name.value);

        let number = self.value.to_enum_number(ctx);

        let options = if self.options.is_empty() {
            None
        } else {
            Some(ast::OptionBody::to_enum_value_options(&self.options, ctx))
        };

        EnumValueDescriptorProto {
            name,
            number,
            options,
        }
    }
}

impl ast::Service {
    fn to_service_descriptor(&self, ctx: &mut Context) -> ServiceDescriptorProto {
        let name = s(&self.name);
        let options = if self.options.is_empty() {
            None
        } else {
            Some(ast::Option::to_service_options(&self.options))
        };

        ctx.enter(Definition::Service {
            full_name: ctx.full_name(&self.name.value),
        });

        let method = self
            .methods
            .iter()
            .map(|m| m.to_method_descriptor(ctx))
            .collect();

        ctx.exit();
        ServiceDescriptorProto {
            name,
            method,
            options,
        }
    }
}

impl ast::Method {
    fn to_method_descriptor(&self, ctx: &mut Context) -> MethodDescriptorProto {
        let name = s(&self.name);

        let (input_type, kind) = ctx.resolve_type_name(&self.input_ty);
        if !matches!(
            kind,
            None | Some(DefinitionKind::Message) | Some(DefinitionKind::Group)
        ) {
            ctx.errors.push(CheckError::InvalidMethodTypeName {
                name: self.input_ty.to_string(),
                kind: "input",
                span: self.input_ty.span(),
            })
        }

        let (output_type, kind) = ctx.resolve_type_name(&self.output_ty);
        if !matches!(
            kind,
            None | Some(DefinitionKind::Message) | Some(DefinitionKind::Group)
        ) {
            ctx.errors.push(CheckError::InvalidMethodTypeName {
                name: self.output_ty.to_string(),
                kind: "output",
                span: self.output_ty.span(),
            })
        }

        let options = if self.options.is_empty() {
            None
        } else {
            Some(ast::Option::to_method_options(&self.options))
        };

        let client_streaming = Some(self.is_client_streaming);
        let server_streaming = Some(self.is_server_streaming);

        MethodDescriptorProto {
            name,
            input_type: Some(input_type),
            output_type: Some(output_type),
            options,
            client_streaming,
            server_streaming,
        }
    }
}

impl<'a> Context<'a> {
    fn parent_extendee(&self) -> Option<String> {
        match self.stack.last() {
            Some(Definition::Extend { extendee, .. }) => Some(extendee.clone()),
            _ => None,
        }
    }

    fn parent_oneof(&self) -> Option<i32> {
        match self.stack.last() {
            Some(Definition::Oneof { index, .. }) => Some(*index),
            _ => None,
        }
    }
}
