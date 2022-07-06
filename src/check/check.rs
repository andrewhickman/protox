use std::{convert::TryFrom, fmt::Display};

use logos::Span;
use prost_types::{
    descriptor_proto::{ExtensionRange, ReservedRange},
    enum_descriptor_proto::EnumReservedRange,
    field_descriptor_proto, DescriptorProto, EnumDescriptorProto, ExtensionRangeOptions,
    FieldDescriptorProto, FieldOptions, FileDescriptorProto, FileOptions, MessageOptions,
    OneofDescriptorProto, ServiceDescriptorProto,
};

use crate::{ast, case::to_camel_case, index_to_i32, s, MAX_MESSAGE_FIELD_NUMBER};

use super::{ir, names::DefinitionKind, CheckError, NameMap};

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
    Enum,
    Service { full_name: String },
    Oneof,
    Extend { extendee: String },
    Group,
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
                Scope::Message { full_name, .. }
                | Scope::Service { full_name, .. }
                | Scope::Package { full_name } => return format!("{}.{}", full_name, name),
                _ => continue,
            }
        }

        name.to_string()
    }

    fn resolve_type_name(
        &mut self,
        absolute: bool,
        name: String,
        span: Span,
    ) -> (String, Option<DefinitionKind>) {
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
                        Scope::Message { full_name, .. }
                        | Scope::Service { full_name, .. }
                        | Scope::Package { full_name } => format!(".{}.{}", full_name, name),
                        _ => continue,
                    };

                    if let Some(def) = name_map.get(&full_name) {
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

    fn in_oneof(&self) -> bool {
        matches!(self.scope.last(), Some(Scope::Oneof { .. }))
    }

    fn in_extend(&self) -> bool {
        matches!(self.scope.last(), Some(Scope::Extend { .. }))
    }

    fn check_file(&mut self, file: &ir::File) -> FileDescriptorProto {
        if let Some(package) = &file.ast.package {
            self.enter(Scope::Package {
                full_name: package.name.to_string(),
            });
        }

        let package = file.ast.package.as_ref().map(|p| p.name.to_string());

        let dependency = file
            .ast
            .imports
            .iter()
            .map(|i| i.value.value.clone())
            .collect();
        let public_dependency = file
            .ast
            .imports
            .iter()
            .enumerate()
            .filter(|(_, i)| i.kind == Some(ast::ImportKind::Public))
            .map(|(index, _)| index_to_i32(index))
            .collect();
        let weak_dependency = file
            .ast
            .imports
            .iter()
            .enumerate()
            .filter(|(_, i)| i.kind == Some(ast::ImportKind::Weak))
            .map(|(index, _)| index_to_i32(index))
            .collect();

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
                ast::FileItem::Extend(e) => extension.push(self.check_extend(e)),
                ast::FileItem::Service(s) => service.push(self.check_service(s)),
            }
        }

        let options = self.check_file_options(&file.ast.options);

        let syntax = if self.syntax == ast::Syntax::default() {
            None
        } else {
            Some(self.syntax.to_string())
        };

        if file.ast.package.is_some() {
            self.exit();
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

        let mut enum_type = Vec::new();
        let mut extension = Vec::new();
        let mut extension_range = Vec::new();
        let mut reserved_range = Vec::new();
        let mut reserved_name = Vec::new();
        let mut options = None;
        if let Some(body) = message.ast.body() {
            for item in &body.items {
                match item {
                    ast::MessageItem::Field(_) | ast::MessageItem::Message(_) => continue,
                    ast::MessageItem::Enum(e) => enum_type.push(self.check_enum(e)),
                    ast::MessageItem::Extend(e) => extension.push(self.check_extend(e)),
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
                let extension_options = self.check_extension_range_options(&extension.options);

                extension_range.extend(extension.ranges.iter().map(|e| ExtensionRange {
                    options: extension_options.clone(),
                    ..self.check_message_extension_range(e)
                }));
            }

            options = self.check_message_options(&body.options);
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
        let oneof_index = field.oneof_index;

        let descriptor = match field.ast {
            ir::FieldSource::Field(ast) => self.check_field(ast),
            ir::FieldSource::Group(group) => self.check_group(group),
            ir::FieldSource::Map(map) => self.check_map(map),
            ir::FieldSource::MapKey(map) => self.check_map_key(map),
            ir::FieldSource::MapValue(map) => self.check_map_value(map),
        };

        FieldDescriptorProto {
            oneof_index: field.oneof_index,
            ..descriptor
        }
    }

    fn check_field(&mut self, field: &ast::Field) -> FieldDescriptorProto {
        let name = s(&field.name.value);
        let number = self.check_field_number(&field.number);
        let label = self.check_field_label(field.label.clone(), field.span.clone());
        let (ty, type_name) = self.check_field_type(&field.ty);
        let (default_value, options) = self.check_field_options(&field.options);

        if default_value.is_some() && ty == Some(field_descriptor_proto::Type::Message) {
            self.errors.push(CheckError::InvalidDefault {
                kind: "message",
                span: field.span.clone(),
            })
        }

        let json_name = Some(to_camel_case(&field.name.value));

        let proto3_optional = if self.syntax == ast::Syntax::Proto3
            && matches!(field.label, Some((ast::FieldLabel::Optional, _)))
        {
            Some(true)
        } else {
            None
        };

        FieldDescriptorProto {
            name,
            number,
            label: label.map(|l| l as i32),
            r#type: ty.map(|t| t as i32),
            type_name,
            default_value,
            json_name,
            options,
            proto3_optional,
            ..Default::default()
        }
    }

    fn check_field_label(
        &mut self,
        label: Option<(ast::FieldLabel, Span)>,
        field_span: Span,
    ) -> Option<field_descriptor_proto::Label> {
        let (label, span) = match label {
            Some((label, span)) => (Some(label), span),
            None => (None, field_span),
        };

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

    fn check_group(&mut self, field: &ast::Group) -> FieldDescriptorProto {
        todo!()
    }

    fn check_map(&mut self, field: &ast::Map) -> FieldDescriptorProto {
        todo!()
    }

    fn check_map_key(&mut self, field: &ast::Map) -> FieldDescriptorProto {
        let ty = self.check_map_key_type(&field.key_ty);

        FieldDescriptorProto {
            name: s("key"),
            number: Some(1),
            label: Some(field_descriptor_proto::Label::Optional as i32),
            r#type: ty.map(|t| t as i32),
            json_name: s("key"),
            ..Default::default()
        }
    }

    fn check_map_value(&mut self, field: &ast::Map) -> FieldDescriptorProto {
        let (ty, type_name) = self.check_field_type(&field.key_ty);

        FieldDescriptorProto {
            name: s("value"),
            number: Some(2),
            label: Some(field_descriptor_proto::Label::Optional as i32),
            r#type: ty.map(|t| t as i32),
            type_name,
            json_name: s("value"),
            ..Default::default()
        }
    }

    fn check_map_key_type(&mut self, ty: &ast::Ty) -> Option<field_descriptor_proto::Type> {
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
                    .push(CheckError::InvalidMapFieldKeyType { span: todo!() });
                None
            }
        }
    }

    fn check_field_type(
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
            ast::Ty::Named(type_name) => match self.resolve_type_name(
                type_name.leading_dot.is_some(),
                type_name.to_string(),
                type_name.span(),
            ) {
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

    fn check_oneof(&mut self, oneof: &ir::Oneof) -> OneofDescriptorProto {
        match &oneof.ast {
            ir::OneofSource::Oneof(oneof) => {
                for field in &oneof.fields {
                    if matches!(
                        field,
                        ast::MessageField::Oneof(_) | ast::MessageField::Map(_)
                    ) {
                        self.errors.push(CheckError::InvalidOneofFieldKind {
                            kind: field.kind_name(),
                            span: field.span(),
                        });
                    }
                }
            }
            ir::OneofSource::Field(_) => todo!(),
        }

        todo!()
    }

    fn check_enum(&mut self, e: &ast::Enum) -> EnumDescriptorProto {
        todo!()
    }

    fn check_extend(&mut self, e: &ast::Extend) -> FieldDescriptorProto {
        todo!()
        // } else if ctx.in_extend()
        //     && matches!(
        //         self,
        //         ast::MessageField::Oneof(_) | ast::MessageField::Map(_)
        //     )
        // {
        //     ctx.errors.push(CheckError::InvalidExtendFieldKind {
        //         kind: self.kind_name(),
        //         span: self.span(),
        //     });
        //     return;
        // }
    }

    fn check_service(&mut self, s: &ast::Service) -> ServiceDescriptorProto {
        todo!()
    }

    fn check_message_reserved_range(&mut self, range: &ast::ReservedRange) -> ReservedRange {
        let start = self.check_field_number(&range.start);
        let end = match &range.end {
            ast::ReservedRangeEnd::None => start.map(|n| n + 1),
            ast::ReservedRangeEnd::Int(value) => self.check_field_number(value),
            ast::ReservedRangeEnd::Max => Some(MAX_MESSAGE_FIELD_NUMBER + 1),
        };

        ReservedRange { start, end }
    }

    fn check_message_extension_range(&mut self, range: &ast::ReservedRange) -> ExtensionRange {
        let start = self.check_field_number(&range.start);
        let end = match &range.end {
            ast::ReservedRangeEnd::None => start.map(|n| n + 1),
            ast::ReservedRangeEnd::Int(value) => self.check_field_number(value),
            ast::ReservedRangeEnd::Max => Some(MAX_MESSAGE_FIELD_NUMBER + 1),
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
            ast::ReservedRangeEnd::Max => Some(i32::MAX),
        };

        EnumReservedRange { start, end }
    }

    fn check_field_number(&mut self, int: &ast::Int) -> Option<i32> {
        match (int.negative, i32::try_from(int.value)) {
            (false, Ok(number @ 1..=MAX_MESSAGE_FIELD_NUMBER)) => Some(number),
            _ => {
                self.errors.push(CheckError::InvalidMessageNumber {
                    span: int.span.clone(),
                });
                None
            }
        }
    }

    fn check_enum_number(&mut self, int: &ast::Int) -> Option<i32> {
        let as_i32 = if int.negative {
            int.value.checked_neg().and_then(|n| i32::try_from(n).ok())
        } else {
            i32::try_from(int.value).ok()
        };

        if as_i32.is_none() {
            self.errors.push(CheckError::InvalidEnumNumber {
                span: int.span.clone(),
            });
        }

        as_i32
    }

    fn check_file_options(&mut self, options: &[ast::Option]) -> Option<FileOptions> {
        todo!()
    }

    fn check_message_options(&mut self, options: &[ast::Option]) -> Option<MessageOptions> {
        todo!()
    }

    fn check_field_options(
        &mut self,
        options: &[ast::OptionBody],
    ) -> (Option<String>, Option<FieldOptions>) {
        todo!()
    }

    fn check_extension_range_options(
        &mut self,
        options: &[ast::OptionBody],
    ) -> Option<ExtensionRangeOptions> {
        todo!()
    }
}
