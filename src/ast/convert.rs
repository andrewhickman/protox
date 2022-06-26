use std::convert::TryFrom;

use prost_types::{
    descriptor_proto::{ExtensionRange, ReservedRange},
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorProto, FileOptions,
    MessageOptions, OneofDescriptorProto, ServiceDescriptorProto, SourceCodeInfo,
};

use crate::{ast, MAX_MESSAGE_FIELD_NUMBER};

impl ast::File {
    pub fn to_file_descriptor(&self, source_code: Option<&str>) -> FileDescriptorProto {
        let package = self.package.as_ref().map(|p| p.name.to_string());

        let dependency = self.imports.iter().map(|i| i.value.value.clone()).collect();
        let public_dependency = self
            .imports
            .iter()
            .enumerate()
            .filter(|(_, i)| i.kind == Some(ast::ImportKind::Public))
            .map(|(index, _)| i32::try_from(index).unwrap())
            .collect();
        let weak_dependency = self
            .imports
            .iter()
            .enumerate()
            .filter(|(_, i)| i.kind == Some(ast::ImportKind::Weak))
            .map(|(index, _)| i32::try_from(index).unwrap())
            .collect();

        let message_type = self
            .messages
            .iter()
            .map(|m| m.to_message_descriptor())
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
            source_code_info,
            syntax,
        }
    }

    fn get_source_code_info(&self, _lines: &LineResolver) -> SourceCodeInfo {
        todo!()
    }
}

impl ast::Message {
    fn to_message_descriptor(&self) -> DescriptorProto {
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
            .map(|m| m.to_message_descriptor())
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

        let mut reserved_range = vec![];
        let mut reserved_name = vec![];
        for reserved in &self.body.reserved {
            match &reserved.kind {
                ast::ReservedKind::Ranges(ranges) => {
                    reserved_range.extend(ranges.iter().map(|r| r.to_message_reserved_range()))
                }
                ast::ReservedKind::Names(names) => {
                    reserved_name.extend(names.iter().map(|n| n.to_string()))
                }
            }
        }

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
        todo!()
        // FieldDescriptorProto {
        //     name: (),
        //     number: (),
        //     label: (),
        //     r#type: (),
        //     type_name: (),
        //     extendee: (),
        //     default_value: (),
        //     oneof_index: (),
        //     json_name: (),
        //     options: (),
        //     proto3_optional: (),
        // }
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

impl ast::ReservedRange {
    fn to_message_reserved_range(&self) -> ReservedRange {
        let end = match &self.end {
            ast::ReservedRangeEnd::None => i32::try_from(self.start.value + 1).unwrap(),
            ast::ReservedRangeEnd::Int(value) => i32::try_from(value.value).unwrap(),
            ast::ReservedRangeEnd::Max => MAX_MESSAGE_FIELD_NUMBER + 1,
        };

        ReservedRange {
            start: Some(i32::try_from(self.start.value).unwrap()),
            end: Some(end),
        }
    }

    fn to_enum_reserved_range(&self) -> ReservedRange {
        let end = match &self.end {
            ast::ReservedRangeEnd::None => i32::try_from(self.start.value).unwrap(),
            ast::ReservedRangeEnd::Int(value) => i32::try_from(value.value).unwrap(),
            ast::ReservedRangeEnd::Max => i32::MAX,
        };

        ReservedRange {
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

struct LineResolver {
    lines: Vec<usize>,
}

impl LineResolver {
    fn new(source_code: &str) -> Self {
        let lines = source_code
            .match_indices('\n')
            .map(|(index, _)| index + 1)
            .collect();
        LineResolver { lines }
    }

    fn resolve(&self, offset: usize) -> (usize, usize) {
        match self.lines.binary_search(&offset) {
            Ok(index) => (index + 1, 0),
            Err(0) => (0, offset),
            Err(index) => (index, offset - self.lines[index - 1]),
        }
    }
}

#[test]
fn resolve_line_number() {
    let resolver = LineResolver::new("hello\nworld\nfoo");

    dbg!(&resolver.lines);

    assert_eq!(resolver.resolve(0), (0, 0));
    assert_eq!(resolver.resolve(4), (0, 4));
    assert_eq!(resolver.resolve(5), (0, 5));
    assert_eq!(resolver.resolve(6), (1, 0));
    assert_eq!(resolver.resolve(7), (1, 1));
    assert_eq!(resolver.resolve(10), (1, 4));
    assert_eq!(resolver.resolve(11), (1, 5));
    assert_eq!(resolver.resolve(12), (2, 0));
    assert_eq!(resolver.resolve(13), (2, 1));
    assert_eq!(resolver.resolve(14), (2, 2));
    assert_eq!(resolver.resolve(15), (2, 3));
}
