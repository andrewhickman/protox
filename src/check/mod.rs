use std::convert::TryFrom;

use logos::Span;
use miette::Diagnostic;
use prost_types::{
    descriptor_proto::{ExtensionRange, ReservedRange},
    field_descriptor_proto, DescriptorProto, EnumDescriptorProto, ExtensionRangeOptions,
    FieldDescriptorProto, FieldOptions, FileDescriptorProto, FileOptions, MessageOptions,
    OneofDescriptorProto, OneofOptions, ServiceDescriptorProto, SourceCodeInfo,
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
    #[error("enum numbers must be between {} and {}", i32::MIN, i32::MAX)]
    InvalidEnumNumber {
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
    #[error("extension fields may not be required")]
    RequiredExtendField {
        #[label("defined here")]
        span: Span,
    },
    #[error("map fields cannot have labels")]
    MapFieldWithLabel {
        #[label("defined here")]
        span: Span,
    },
    #[error("fields must have a label with proto2 syntax (expected one of 'optional', 'repeated' or 'required')")]
    Proto2FieldMissingLabel {
        #[label("field defined here")]
        span: Span,
    },
    #[error("groups are not allowed in proto3 syntax")]
    Proto3GroupField {
        #[label("defined here")]
        span: Span,
    },
    #[error("required fields are not allowed in proto3 syntax")]
    Proto3RequiredField {
        #[label("defined here")]
        span: Span,
    },
    #[error("{kind} fields are not allowed in a oneof")]
    InvalidOneofFieldKind {
        kind: &'static str,
        #[label("defined here")]
        span: Span,
    },
    #[error("oneof fields cannot have labels")]
    OneofFieldWithLabel {
        #[label("defined here")]
        span: Span,
    },
}

struct Context {
    syntax: ast::Syntax,
    errors: Vec<CheckError>,
    package: String,
    stack: Vec<Definition>,
}

enum Definition {
    Message { full_name: String },
    Enum { full_name: String },
    Service { full_name: String },
    Oneof { full_name: String, index: i32 },
    Extend { extendee: String },
    Group { full_name: String },
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
            package: self
                .package
                .as_ref()
                .map(|p| p.name.to_string())
                .unwrap_or_default(),
            stack: vec![],
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

        let mut message_type: Vec<_> = self
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
        let mut extension = Vec::new();
        self.extends
            .iter()
            // TODO message ordering is wrong
            .for_each(|e| e.to_field_descriptors(&mut ctx, &mut message_type, &mut extension));

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
        ctx.enter(Definition::Message {
            full_name: make_name(ctx.scope_name(), &self.name.value),
        });

        let name = Some(self.name.value.clone());
        let body = self.body.to_message_descriptor(ctx);

        ctx.exit();
        DescriptorProto { name, ..body }
    }
}

impl ast::MessageBody {
    fn to_message_descriptor(&self, ctx: &mut Context) -> DescriptorProto {
        // TODO ordering of nested messages is wrong
        let mut generated_nested_messages = Vec::new();
        let mut oneof_decl = Vec::new();
        let mut field = Vec::new();

        self.fields.iter().for_each(|e| {
            e.to_field_descriptors(
                ctx,
                &mut generated_nested_messages,
                &mut field,
                &mut oneof_decl,
            )
        });
        let mut extension = Vec::new();
        self.extends.iter().for_each(|e| {
            e.to_field_descriptors(ctx, &mut generated_nested_messages, &mut extension)
        });

        let mut nested_type: Vec<_> = self
            .messages
            .iter()
            .map(|m| m.to_message_descriptor(ctx))
            .collect();
        nested_type.extend(generated_nested_messages);

        let enum_type = self.enums.iter().map(|e| e.to_enum_descriptor()).collect();

        let mut extension_range = Vec::new();
        self.extensions
            .iter()
            .for_each(|e| e.to_extension_ranges(ctx, &mut extension_range));

        let options = if self.options.is_empty() {
            None
        } else {
            Some(ast::Option::to_message_options(&self.options))
        };

        let mut reserved_range = Vec::new();
        for r in &self.reserved {
            for range in r.ranges() {
                reserved_range.push(range.to_message_reserved_range(ctx));
            }
        }
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
    fn to_field_descriptors(
        &self,
        ctx: &mut Context,
        messages: &mut Vec<DescriptorProto>,
        fields: &mut Vec<FieldDescriptorProto>,
        oneofs: &mut Vec<OneofDescriptorProto>,
    ) {
        if ctx.in_oneof()
            && matches!(
                self,
                ast::MessageField::Oneof(_) | ast::MessageField::Map(_)
            )
        {
            ctx.errors.push(CheckError::InvalidOneofFieldKind {
                kind: self.kind_name(),
                span: self.span(),
            });
            return;
        } else if ctx.in_extend()
            && matches!(
                self,
                ast::MessageField::Oneof(_) | ast::MessageField::Map(_)
            )
        {
            ctx.errors.push(CheckError::InvalidExtendFieldKind {
                kind: self.kind_name(),
                span: self.span(),
            });
            return;
        }

        match self {
            ast::MessageField::Field(field) => fields.push(field.to_field_descriptor(ctx)),
            ast::MessageField::Group(group) => {
                fields.push(group.to_field_descriptor(ctx, messages))
            }
            ast::MessageField::Map(map) => fields.push(map.to_field_descriptor(ctx, messages)),
            ast::MessageField::Oneof(oneof) => {
                oneofs.push(oneof.to_oneof_descriptor(ctx, messages, fields, oneofs.len()))
            }
        }
    }

    fn kind_name(&self) -> &'static str {
        match self {
            ast::MessageField::Field(_) => "normal",
            ast::MessageField::Group(_) => "group",
            ast::MessageField::Map(_) => "map",
            ast::MessageField::Oneof(_) => "oneof",
        }
    }

    fn span(&self) -> Span {
        match self {
            ast::MessageField::Field(field) => field.span.clone(),
            ast::MessageField::Group(field) => field.span.clone(),
            ast::MessageField::Map(field) => field.span.clone(),
            ast::MessageField::Oneof(field) => field.span.clone(),
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

        ctx.check_label(self.label, self.span.clone());

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
            extendee: ctx.parent_extendee(),
            default_value,
            oneof_index: ctx.parent_oneof(),
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
            label: None,
            r#type,
            type_name,
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
        ctx.enter(Definition::Group {
            full_name: make_name(ctx.scope_name(), &self.name.value),
        });

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

        let json_name = Some(to_camel_case(&self.name.value));

        ctx.exit();
        FieldDescriptorProto {
            name: field_name,
            number,
            label: None,
            r#type,
            type_name,
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
        let extendee = ctx.resolve_type_name(&self.extendee);
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
            full_name: make_name(ctx.scope_name(), &self.name.value),
            index: index_to_i32(index),
        });

        let name = Some(self.name.value.clone());

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

impl ast::Reserved {
    fn ranges(&self) -> impl Iterator<Item = &'_ ast::ReservedRange> + '_ {
        match &self.kind {
            ast::ReservedKind::Ranges(ranges) => ranges.iter(),
            _ => [].iter(),
        }
    }

    fn names(&self) -> impl Iterator<Item = &ast::Ident> + '_ {
        match &self.kind {
            ast::ReservedKind::Names(names) => names.iter(),
            _ => [].iter(),
        }
    }
}

impl ast::Extensions {
    fn to_extension_ranges(&self, ctx: &mut Context, ranges: &mut Vec<ExtensionRange>) {
        let options = if self.options.is_empty() {
            None
        } else {
            Some(ast::OptionBody::to_extension_range_options(&self.options))
        };

        for range in &self.ranges {
            ranges.push(ExtensionRange {
                options: options.clone(),
                ..range.to_extension_range(ctx)
            });
        }
    }
}

impl ast::ReservedRange {
    fn to_message_reserved_range(&self, ctx: &mut Context) -> ReservedRange {
        let start = self.start.to_field_number(ctx);
        let end = match &self.end {
            ast::ReservedRangeEnd::None => start.map(|n| n + 1),
            ast::ReservedRangeEnd::Int(value) => value.to_field_number(ctx),
            ast::ReservedRangeEnd::Max => Some(MAX_MESSAGE_FIELD_NUMBER + 1),
        };

        ReservedRange { start, end }
    }

    fn to_extension_range(&self, ctx: &mut Context) -> ExtensionRange {
        let start = self.start.to_field_number(ctx);
        let end = match &self.end {
            ast::ReservedRangeEnd::None => start.map(|n| n + 1),
            ast::ReservedRangeEnd::Int(value) => value.to_field_number(ctx),
            ast::ReservedRangeEnd::Max => Some(MAX_MESSAGE_FIELD_NUMBER + 1),
        };

        ExtensionRange {
            start,
            end,
            ..Default::default()
        }
    }

    fn to_enum_reserved_range(&self, ctx: &mut Context) -> ReservedRange {
        let start = self.start.to_enum_number(ctx);
        let end = match &self.end {
            ast::ReservedRangeEnd::None => start,
            ast::ReservedRangeEnd::Int(value) => value.to_enum_number(ctx),
            ast::ReservedRangeEnd::Max => Some(i32::MAX),
        };

        ReservedRange { start, end }
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

    fn to_oneof_options(this: &[Self]) -> OneofOptions {
        todo!()
    }
}

impl ast::OptionBody {
    fn to_field_options(this: &[Self]) -> (Option<String>, FieldOptions) {
        todo!()
    }

    fn to_extension_range_options(this: &[Self]) -> ExtensionRangeOptions {
        todo!()
    }
}

impl Context {
    // resolve top-level scope

    // add name for conflict checking / later resolution

    fn enter(&mut self, def: Definition) {
        self.stack.push(def);
    }

    fn exit(&mut self) {
        self.stack.pop().expect("unbalanced stack");
    }

    fn resolve_type_name(&self, name: &ast::TypeName) -> String {
        // TODO resolve
        // return name unchanged if no imports?
        name.to_string()
    }

    fn scope_name(&self) -> &str {
        for def in self.stack.iter().rev() {
            match def {
                Definition::Message { full_name }
                | Definition::Group { full_name }
                | Definition::Oneof { full_name, .. }
                | Definition::Enum { full_name }
                | Definition::Service { full_name } => return full_name.as_str(),
                Definition::Extend { .. } => continue,
            }
        }

        self.package.as_str()
    }

    fn in_oneof(&self) -> bool {
        matches!(self.stack.last(), Some(Definition::Oneof { .. }))
    }

    fn in_extend(&self) -> bool {
        matches!(self.stack.last(), Some(Definition::Extend { .. }))
    }

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

    fn check_label(&mut self, label: Option<ast::FieldLabel>, span: Span) {
        if self.in_extend() && label == Some(ast::FieldLabel::Required) {
            self.errors.push(CheckError::RequiredExtendField { span });
        } else if self.in_oneof() && label.is_some() {
            self.errors.push(CheckError::OneofFieldWithLabel { span });
        } else if self.syntax == ast::Syntax::Proto2 && label.is_none() && !self.in_oneof() {
            self.errors
                .push(CheckError::Proto2FieldMissingLabel { span });
        } else if self.syntax == ast::Syntax::Proto3 && label == Some(ast::FieldLabel::Required) {
            self.errors.push(CheckError::Proto3RequiredField { span });
        }
    }
}

fn make_name(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        name.to_owned()
    } else {
        format!("{}.{}", namespace, name)
    }
}
