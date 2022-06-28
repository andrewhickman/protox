use std::convert::TryFrom;

use logos::Span;
use miette::Diagnostic;
use prost_types::{
    descriptor_proto::{ExtensionRange, ReservedRange},
    field, field_descriptor_proto, DescriptorProto, EnumDescriptorProto, FieldDescriptorProto,
    FieldOptions, FileDescriptorProto, FileOptions, MessageOptions, OneofDescriptorProto,
    ServiceDescriptorProto, SourceCodeInfo,
};
use thiserror::Error;

use crate::{
    ast,
    case::{to_camel_case, to_pascal_case},
    files::FileMap,
    index_to_i32,
    lines::LineResolver,
    MAX_MESSAGE_FIELD_NUMBER,
};

#[cfg(test)]
mod tests;

#[derive(Error, Debug, Diagnostic, PartialEq)]
pub(crate) enum CheckError {
    #[error("message numbers must be between 1 and {}", MAX_MESSAGE_FIELD_NUMBER)]
    InvalidMessageNumber {
        #[label("defined here")]
        span: Span,
    },
    #[error("{kind} fields may not have default values")]
    InvalidDefault {
        kind: &'static str,
        #[label("defined here")]
        span: Span,
    },
    #[error("{kind} fields are not allowed in extensions")]
    InvalidExtendFieldKind {
        kind: &'static str,
        #[label("defined here")]
        span: Span,
    },
    #[error("extensions fields may not be required")]
    RequiredExtendField{
        #[label("defined here")]
        span: Span,
    },
}

struct Context {
    syntax: ast::Syntax,
    errors: Vec<CheckError>,
}

impl ast::File {
    pub fn to_file_descriptor(
        &self,
        name: Option<&str>,
        source_code: Option<&str>,
        file_map: Option<&FileMap>,
    ) -> Result<FileDescriptorProto, Vec<CheckError>> {
        let mut ctx = Context {
            syntax: self.syntax,
            errors: vec![],
        };

        let name = name.map(ToOwned::to_owned);

        let package = self.package.as_ref().map(|p| p.name.to_string());

        let dependency = self.imports.iter().map(|i| i.value.value.clone()).collect();
        let public_dependency = self
            .imports
            .iter()
            .enumerate()
            .filter(|(_, i)| i.kind == Some(ast::ImportKind::Public))
            .map(|(index, _)| index_to_i32(index))
            .collect();
        let weak_dependency = self
            .imports
            .iter()
            .enumerate()
            .filter(|(_, i)| i.kind == Some(ast::ImportKind::Weak))
            .map(|(index, _)| index_to_i32(index))
            .collect();

        let message_type = self
            .messages
            .iter()
            .map(|m| m.to_message_descriptor(&mut ctx))
            .collect();
        let enum_type = self.enums.iter().map(|e| e.to_enum_descriptor()).collect();
        let service = self
            .services
            .iter()
            .map(|s| s.to_service_descriptor())
            .collect();
        let extension = self
            .extends
            .iter()
            .flat_map(|e| e.to_field_descriptors(&mut ctx))
            .collect();

        let options = if self.options.is_empty() {
            None
        } else {
            Some(ast::Option::to_file_options(&self.options))
        };

        let source_code_info = source_code.map(|c| {
            let lines = LineResolver::new(c);
            self.get_source_code_info(&lines)
        });

        let syntax = Some(self.syntax.to_string());

        if ctx.errors.is_empty() {
            Ok(FileDescriptorProto {
                name,
                package,
                dependency,
                public_dependency,
                weak_dependency,
                message_type,
                enum_type,
                service,
                extension,
                options,
                source_code_info,
                syntax,
            })
        } else {
            Err(ctx.errors)
        }
    }

    fn get_source_code_info(&self, _lines: &LineResolver) -> SourceCodeInfo {
        todo!()
    }
}

impl ast::Message {
    fn to_message_descriptor(&self, ctx: &mut Context) -> DescriptorProto {
        let name = Some(self.name.value.clone());

        DescriptorProto {
            name,
            ..self.body.to_message_descriptor(ctx)
        }
    }
}

impl ast::MessageBody {
    fn to_message_descriptor(&self, ctx: &mut Context) -> DescriptorProto {
        let mut generated_nested_messages = Vec::new();
        let field: Vec<_> = self
            .fields
            .iter()
            .map(|e| e.to_field_descriptor(ctx, &mut generated_nested_messages))
            .collect();
        let extension = self
            .extends
            .iter()
            .flat_map(|e| e.to_field_descriptors(ctx))
            .collect();

        let mut nested_type: Vec<_> = self
            .messages
            .iter()
            .map(|m| m.to_message_descriptor(ctx))
            .collect();
        nested_type.extend(generated_nested_messages);

        let enum_type = self.enums.iter().map(|e| e.to_enum_descriptor()).collect();

        let extension_range = self
            .extensions
            .iter()
            .map(|e| e.to_extension_range())
            .collect();

        let oneof_decl = self
            .oneofs
            .iter()
            .map(|o| o.to_oneof_descriptor())
            .collect();

        let options = if self.options.is_empty() {
            None
        } else {
            Some(ast::Option::to_message_options(&self.options))
        };

        let reserved_range = self
            .reserved
            .iter()
            .flat_map(|r| r.ranges().map(|r| r.to_message_reserved_range()))
            .collect();
        let reserved_name = self
            .reserved
            .iter()
            .flat_map(|r| r.names().map(|i| i.value.to_owned()))
            .collect::<Vec<_>>();

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
}

impl ast::MessageField {
    fn to_field_descriptor(
        &self,
        ctx: &mut Context,
        messages: &mut Vec<DescriptorProto>,
    ) -> FieldDescriptorProto {
        match self {
            ast::MessageField::Field(field) => field.to_field_descriptor(ctx),
            ast::MessageField::Group(group) => group.to_field_descriptor(ctx, messages),
            ast::MessageField::Map(map) => map.to_field_descriptor(ctx, messages),
        }
    }
}

impl ast::Field {
    fn to_field_descriptor(&self, ctx: &mut Context) -> FieldDescriptorProto {
        let name = Some(self.name.value.clone());
        let number = self.number.to_field_number(ctx);
        let label = Some(
            self.label
                .unwrap_or(ast::FieldLabel::Optional)
                .to_field_label() as i32,
        );
        let (ty, type_name) = self.ty.to_type(ctx);

        let (default_value, options) = if self.options.is_empty() {
            (None, None)
        } else {
            let (default_value, options) = ast::OptionBody::to_field_options(&self.options);
            (default_value, Some(options))
        };

        if default_value.is_some() && ty == Some(field_descriptor_proto::Type::Message) {
            ctx.errors.push(CheckError::InvalidDefault {
                kind: "message",
                span: self.span.clone(),
            })
        }

        let json_name = Some(to_camel_case(&self.name.value));

        let proto3_optional =
            if ctx.syntax == ast::Syntax::Proto3 && self.label == Some(ast::FieldLabel::Optional) {
                Some(true)
            } else {
                None
            };

        FieldDescriptorProto {
            name,
            number,
            label,
            r#type: ty.map(|t| t as i32),
            type_name,
            extendee: None,
            default_value,
            oneof_index: None,
            json_name,
            options,
            proto3_optional,
        }
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
}

impl ast::FieldLabel {
    fn to_field_label(self) -> field_descriptor_proto::Label {
        match self {
            ast::FieldLabel::Optional => field_descriptor_proto::Label::Optional,
            ast::FieldLabel::Required => field_descriptor_proto::Label::Required,
            ast::FieldLabel::Repeated => field_descriptor_proto::Label::Repeated,
        }
    }
}

impl ast::KeyTy {
    fn to_type(&self) -> field_descriptor_proto::Type {
        match self {
            ast::KeyTy::Int32 => field_descriptor_proto::Type::Int32,
            ast::KeyTy::Int64 => field_descriptor_proto::Type::Int64,
            ast::KeyTy::Uint32 => field_descriptor_proto::Type::Uint32,
            ast::KeyTy::Uint64 => field_descriptor_proto::Type::Uint64,
            ast::KeyTy::Sint32 => field_descriptor_proto::Type::Sint32,
            ast::KeyTy::Sint64 => field_descriptor_proto::Type::Sint64,
            ast::KeyTy::Fixed32 => field_descriptor_proto::Type::Fixed32,
            ast::KeyTy::Fixed64 => field_descriptor_proto::Type::Fixed64,
            ast::KeyTy::Sfixed32 => field_descriptor_proto::Type::Sfixed32,
            ast::KeyTy::Sfixed64 => field_descriptor_proto::Type::Sfixed64,
            ast::KeyTy::Bool => field_descriptor_proto::Type::Bool,
            ast::KeyTy::String => field_descriptor_proto::Type::String,
        }
    }
}

impl ast::Ty {
    fn to_type(&self, ctx: &mut Context) -> (Option<field_descriptor_proto::Type>, Option<String>) {
        match self {
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
            ast::Ty::Named(_) => todo!(), // lookup either message or group or enum (or none if no include context),
        }
    }
}

impl ast::Map {
    fn to_field_descriptor(
        &self,
        ctx: &mut Context,
        messages: &mut Vec<DescriptorProto>,
    ) -> FieldDescriptorProto {
        let name = Some(self.name.value.clone());
        let number = self.number.to_field_number(ctx);

        let generated_message = self.generate_message_descriptor(ctx);
        let r#type = Some(field_descriptor_proto::Type::Message as i32);
        let type_name = Some(make_name(ctx.scope_name(), generated_message.name()));
        messages.push(generated_message);

        let (default_value, options) = if self.options.is_empty() {
            (None, None)
        } else {
            let (default_value, options) = ast::OptionBody::to_field_options(&self.options);
            (default_value, Some(options))
        };

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
            label: None,
            r#type,
            type_name,
            extendee: None,
            default_value: None,
            oneof_index: None,
            json_name,
            options,
            proto3_optional: None,
        }
    }

    fn generate_message_descriptor(&self, ctx: &mut Context) -> DescriptorProto {
        let name = Some(to_pascal_case(&self.name.value) + "Entry");

        let (ty, type_name) = self.ty.to_type(ctx);

        let key_field = FieldDescriptorProto {
            name: Some("key".to_owned()),
            number: Some(1),
            label: Some(field_descriptor_proto::Label::Optional as i32),
            r#type: Some(self.key_ty.to_type() as i32),
            json_name: Some("key".to_owned()),
            ..Default::default()
        };
        let value_field = FieldDescriptorProto {
            name: Some("value".to_owned()),
            number: Some(2),
            label: Some(field_descriptor_proto::Label::Optional as i32),
            r#type: ty.map(|t| t as i32),
            type_name,
            json_name: Some("key".to_owned()),
            ..Default::default()
        };

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

        let number = self.number.to_field_number(ctx);

        let generated_message = DescriptorProto {
            name: message_name,
            ..self.body.to_message_descriptor(ctx)
        };

        let r#type = Some(field_descriptor_proto::Type::Group as i32);
        let type_name = Some(make_name(ctx.scope_name(), generated_message.name()));
        messages.push(generated_message);

        let (default_value, options) = if self.options.is_empty() {
            (None, None)
        } else {
            let (default_value, options) = ast::OptionBody::to_field_options(&self.options);
            (default_value, Some(options))
        };

        if default_value.is_some() {
            ctx.errors.push(CheckError::InvalidDefault {
                kind: "group",
                span: self.span.clone(),
            });
        }

        let json_name = Some(to_camel_case(&self.name.value));

        FieldDescriptorProto {
            name: field_name,
            number,
            label: None,
            r#type,
            type_name,
            extendee: None,
            default_value: None,
            oneof_index: None,
            json_name,
            options,
            proto3_optional: None,
        }
    }
}

impl ast::Extend {
    fn to_field_descriptors(&self, ctx: &mut Context) -> Vec<FieldDescriptorProto> {
        let extendee = ctx.resolve_type_name(&self.extendee);
        self.fields.iter().filter_map(|field| match field {
            ast::MessageField::Field(field) => {
                if field.label == Some(ast::FieldLabel::Required) {
                    ctx.errors.push(CheckError::RequiredExtendField { span: field.span.clone() });
                }

                Some(FieldDescriptorProto {
                    extendee: Some(extendee.clone()),
                    ..field.to_field_descriptor(ctx)
                })
            },
            ast::MessageField::Group(field) => {
                ctx.errors.push(CheckError::InvalidExtendFieldKind {
                    kind: "group",
                    span: field.span.clone(),
                });
                None
            }
            ast::MessageField::Map(field) => {
                ctx.errors.push(CheckError::InvalidExtendFieldKind {
                    kind: "map",
                    span: field.span.clone(),
                });
                None
            }
        })
        .collect()
    }
}

impl ast::Oneof {
    fn to_oneof_descriptor(&self) -> OneofDescriptorProto {
        todo!()
    }
}

impl ast::Reserved {
    fn ranges(&self) -> impl Iterator<Item = &ast::ReservedRange> {
        match &self.kind {
            ast::ReservedKind::Ranges(ranges) => ranges.iter(),
            _ => [].iter(),
        }
    }

    fn names(&self) -> impl Iterator<Item = &ast::Ident> {
        match &self.kind {
            ast::ReservedKind::Names(names) => names.iter(),
            _ => [].iter(),
        }
    }
}

impl ast::Extensions {
    fn to_extension_range(&self) -> ExtensionRange {
        todo!()
    }
}

impl ast::ReservedRange {
    fn to_message_reserved_range(&self) -> ReservedRange {
        let end = match &self.end {
            // TODO check
            ast::ReservedRangeEnd::None => i32::try_from(self.start.value + 1).unwrap(),
            // TODO check
            ast::ReservedRangeEnd::Int(value) => i32::try_from(value.value).unwrap(),
            ast::ReservedRangeEnd::Max => MAX_MESSAGE_FIELD_NUMBER + 1,
        };

        ReservedRange {
            // TODO check
            start: Some(i32::try_from(self.start.value).unwrap()),
            end: Some(end),
        }
    }

    fn to_enum_reserved_range(&self) -> ReservedRange {
        let end = match &self.end {
            // TODO check
            ast::ReservedRangeEnd::None => i32::try_from(self.start.value).unwrap(),
            // TODO check
            ast::ReservedRangeEnd::Int(value) => i32::try_from(value.value).unwrap(),
            ast::ReservedRangeEnd::Max => i32::MAX,
        };

        ReservedRange {
            // TODO check
            start: Some(i32::try_from(self.start.value).unwrap()),
            end: Some(end),
        }
    }
}

impl ast::Enum {
    fn to_enum_descriptor(&self) -> EnumDescriptorProto {
        todo!()
    }
}

impl ast::Service {
    fn to_service_descriptor(&self) -> ServiceDescriptorProto {
        todo!()
    }
}

impl ast::Option {
    fn to_file_options(this: &[Self]) -> FileOptions {
        todo!()
    }

    fn to_message_options(this: &[Self]) -> MessageOptions {
        todo!()
    }
}

impl ast::OptionBody {
    fn to_field_options(this: &[Self]) -> (Option<String>, FieldOptions) {
        todo!()
    }
}

impl Context {
    // resolve top-level scope

    // push scope

    fn resolve_type_name(&self, name: &ast::TypeName) -> String {
        // TODO resolve
        // return name unchanged if no imports?
        name.to_string()
    }

    fn scope_name(&self) -> &str {
        todo!()
    }
}

fn make_name(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        name.to_owned()
    } else {
        format!("{}.{}", namespace, name)
    }
}
