use prost_types::FileDescriptorProto;

use super::*;
use crate::parse::parse;

#[track_caller]
fn check(source: &str) -> Result<FileDescriptorProto, Vec<CheckError>> {
    parse(source).unwrap().to_file_descriptor(None, None, None)
}

#[test]
fn invalid_message_number() {
    assert_eq!(
        check("message Foo { optional int32 i = -5; }"),
        Err(vec![CheckError::InvalidMessageNumber { span: 33..35 }])
    );
    assert_eq!(
        check("message Foo { optional int32 i = 0; }"),
        Err(vec![CheckError::InvalidMessageNumber { span: 33..34 }])
    );
    assert_eq!(
        check("message Foo { optional int32 i = 536870912; }"),
        Err(vec![CheckError::InvalidMessageNumber { span: 33..42 }])
    );
    assert_eq!(
        check("message Foo { optional int32 i = 1; }"),
        Ok(FileDescriptorProto {
            message_type: vec![DescriptorProto {
                name: Some("Foo".to_owned()),
                field: vec![FieldDescriptorProto {
                    name: Some("i".to_owned()),
                    number: Some(1),
                    label: Some(field_descriptor_proto::Label::Optional as _),
                    r#type: Some(field_descriptor_proto::Type::Int32 as _),
                    json_name: Some("i".to_owned()),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        })
    );
    assert_eq!(
        check("message Foo { optional int32 i = 536870911; }"),
        Ok(FileDescriptorProto {
            message_type: vec![DescriptorProto {
                name: Some("Foo".to_owned()),
                field: vec![FieldDescriptorProto {
                    name: Some("i".to_owned()),
                    number: Some(536870911),
                    label: Some(field_descriptor_proto::Label::Optional as _),
                    r#type: Some(field_descriptor_proto::Type::Int32 as _),
                    json_name: Some("i".to_owned()),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        })
    );
}

#[test]
fn generate_map_entry_message() {

    // conflict with other type name
}

#[test]
fn generate_group_message() {

    // conflict with other type name
}

#[test]
fn generated_message_ordering() {
    assert_eq!(
        check(
            "
            extend Bar { optional group Baz = 1 {} }

            message Bar {
                extensions 1;

                map<int32, string> x = 3;

                oneof foo {
                    group Quz = 5 {}
                }

                message Nest {}
            }"
        ),
        Ok(FileDescriptorProto {
            name: todo!(),
            package: todo!(),
            dependency: todo!(),
            public_dependency: todo!(),
            weak_dependency: todo!(),
            message_type: todo!(),
            enum_type: todo!(),
            service: todo!(),
            extension: todo!(),
            options: todo!(),
            source_code_info: todo!(),
            syntax: todo!()
        })
    );
}

#[test]
fn generate_synthetic_oneof() {

    // conflict with other oneof name
}

#[test]
fn invalid_service_type() {
    // use enum/service/oneof etc
}

#[test]
fn name_resolution() {
    // local vs global scope
}

#[test]
fn name_collision() {
    // message vs message vs service etc
    // field vs submessage
    // message vs package
}

#[test]
fn message_field_default_value() {
    // bytes/string etc
}

#[test]
fn message_field_json_name() {}

#[test]
fn map_field_with_label() {}

#[test]
fn map_field_with_default() {}

#[test]
fn message_field_with_default() {}

#[test]
fn message_field_duplicate_number() {}

#[test]
fn message_reserved_range_extrema() {}

#[test]
fn message_reserved_range_overlap() {}

#[test]
fn message_reserved_range_overlap_with_field() {}

#[test]
fn group_field_with_default() {}

#[test]
fn extend_required_field() {}

#[test]
fn extend_map_field() {}

#[test]
fn extend_group_field() {
    // allow
}

#[test]
fn extend_field_number_not_in_extensions() {}

#[test]
fn extend_duplicate_field_number() {}

#[test]
fn extend_oneof_field() {}

#[test]
fn extend_non_options_type_proto3() {}

#[test]
fn repeated_field_default_value() {}

#[test]
fn proto3_group_field() {}

#[test]
fn proto3_required_field() {}

#[test]
fn proto2_field_missing_label() {}

#[test]
fn oneof_field_with_label() {}

#[test]
fn oneof_map_field() {}

#[test]
fn oneof_group_field() {
    // allow
}

#[test]
fn oneof_oneof_field() {}

#[test]
fn empty_oneof() {}

#[test]
fn enum_value_extrema() {}

#[test]
fn enum_reserved_range_extrema() {}

#[test]
fn enum_reserved_range_overlap_with_value() {}

#[test]
fn enum_duplicate_number() {}
