use prost_types::FileDescriptorProto;

use super::*;
use crate::parse::parse;

fn check(source: &str) -> Result<FileDescriptorProto, Vec<CheckError>> {
    parse(source).unwrap().to_file_descriptor(None, None, None)
}

#[test]
fn invalid_message_number() {
    assert_eq!(
        check("message Foo { int32 i = -5; }"),
        Err(vec![CheckError::InvalidMessageNumber { span: 0..0 }])
    );
    assert_eq!(
        check("message Foo { int32 i = 0; }"),
        Err(vec![CheckError::InvalidMessageNumber { span: 0..0 }])
    );
    assert_eq!(
        check("message Foo { int32 i = 536870912; }"),
        Err(vec![CheckError::InvalidMessageNumber { span: 0..0 }])
    );
    assert_eq!(
        check("message Foo { int32 i = 536870911; }"),
        Ok(FileDescriptorProto::default())
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
fn group_field_with_default() {}

#[test]
fn extend_required_field() {}

#[test]
fn extend_map_field() {}

#[test]
fn extend_group_field() {}

#[test]
fn repeated_field_default_value() {}
