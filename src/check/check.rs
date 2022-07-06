use std::{fmt::Display, convert::TryFrom};

use prost_types::{
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorProto, FileOptions,
    ServiceDescriptorProto, OneofDescriptorProto, MessageOptions, descriptor_proto::{ReservedRange, ExtensionRange}, enum_descriptor_proto::EnumReservedRange, ExtensionRangeOptions,
};

use crate::{ast::{self, MessageBody}, index_to_i32, s, MAX_MESSAGE_FIELD_NUMBER};

use super::{ir, CheckError, NameMap};

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

        let field = message.fields.iter()
            .map(|field| self.check_field(field))
            .collect();
        let nested_type = message.messages.iter()
            .map(|nested| self.check_message(nested))
            .collect();
        let oneof_decl = message.oneofs.iter()
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
                    ast::ReservedKind::Ranges(ranges) => reserved_range.extend(ranges.iter().map(|range| self.check_message_reserved_range(range))),
                    ast::ReservedKind::Names(names) => reserved_name.extend(names.iter().map(|name| name.value.to_owned())),
                }
            }

            for extension in &body.extensions {
                let extension_options = self.check_extension_range_options(&extension.options);

                extension_range.extend(extension.ranges.iter().map(|e| {
                    ExtensionRange {
                        options: extension_options.clone(),
                        ..self.check_message_extension_range(e)
                    }
                }));
            }

            options = self.check_message_options(&body.options);
        };

        self.exit();
        DescriptorProto { name: s(message.ast.name()), field, nested_type, extension, enum_type, extension_range, oneof_decl, options, reserved_range, reserved_name }
    }

    fn check_field(&mut self, field: &ir::Field) -> FieldDescriptorProto {
        todo!()
    }

    fn check_oneof(&mut self, field: &ir::Oneof) -> OneofDescriptorProto {
        todo!()
    }

    fn check_enum(&mut self, e: &ast::Enum) -> EnumDescriptorProto {
        todo!()
    }

    fn check_extend(&mut self, e: &ast::Extend) -> FieldDescriptorProto {
        todo!()
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

    fn check_extension_range_options(&mut self, options: &[ast::OptionBody]) -> Option<ExtensionRangeOptions> {
        todo!()
    }
}
