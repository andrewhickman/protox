use std::convert::TryFrom;

use miette::Diagnostic;
use prost_types::{
    descriptor_proto::{ExtensionRange, ReservedRange},
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorProto, FileOptions,
    MessageOptions, OneofDescriptorProto, ServiceDescriptorProto, SourceCodeInfo,
};

use crate::{ast, files::FileMap, lines::LineResolver, Error, MAX_MESSAGE_FIELD_NUMBER};

#[derive(Error, Debug, Diagnostic, PartialEq)]
pub(crate) enum CheckError {}

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
            // TODO check
            .map(|(index, _)| i32::try_from(index).unwrap())
            .collect();
        let weak_dependency = self
            .imports
            .iter()
            .enumerate()
            .filter(|(_, i)| i.kind == Some(ast::ImportKind::Weak))
            // TODO check
            .map(|(index, _)| i32::try_from(index).unwrap())
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
            .map(|e| e.to_field_descriptor())
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

        let field: Vec<_> = self
            .body
            .fields
            .iter()
            .map(|e| e.to_field_descriptor())
            .collect();
        let extension = self
            .body
            .extends
            .iter()
            .map(|e| e.to_field_descriptor())
            .collect();

        let nested_type = self
            .body
            .messages
            .iter()
            .map(|m| m.to_message_descriptor(ctx))
            .collect();
        let enum_type = self
            .body
            .enums
            .iter()
            .map(|e| e.to_enum_descriptor())
            .collect();

        let extension_range = self
            .body
            .extensions
            .iter()
            .map(|e| e.to_extension_range())
            .collect();

        let oneof_decl = self
            .body
            .oneofs
            .iter()
            .map(|o| o.to_oneof_descriptor())
            .collect();

        let options = if self.body.options.is_empty() {
            None
        } else {
            Some(ast::Option::to_message_options(&self.body.options))
        };

        let reserved_range = self
            .body
            .reserved
            .iter()
            .flat_map(|r| r.ranges().map(|r| r.to_message_reserved_range()))
            .collect();
        let reserved_name = self
            .body
            .reserved
            .iter()
            .flat_map(|r| r.names().map(|i| i.value.to_owned()))
            .collect::<Vec<_>>();

        DescriptorProto {
            name,
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
    fn to_field_descriptor(&self) -> FieldDescriptorProto {
        match self {
            ast::MessageField::Field(field) => field.to_field_descriptor(),
            ast::MessageField::Group(group) => group.to_field_descriptor(),
            ast::MessageField::Map(map) => map.to_field_descriptor(),
        }
    }
}

impl ast::Field {
    fn to_field_descriptor(&self) -> FieldDescriptorProto {
        todo!()
    }
}

impl ast::Map {
    fn to_field_descriptor(&self) -> FieldDescriptorProto {
        todo!()
    }

    fn generated_message_name(&self) -> String {
        todo!()
    }
}

impl ast::Group {
    fn to_field_descriptor(&self) -> FieldDescriptorProto {
        todo!()
    }

    fn generated_message_name(&self) -> String {
        todo!()
    }
}

impl ast::Extend {
    fn to_field_descriptor(&self) -> FieldDescriptorProto {
        todo!()
    }
}

impl ast::Extensions {
    fn to_extension_range(&self) -> ExtensionRange {
        todo!()
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
