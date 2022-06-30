use insta::assert_json_snapshot;
use prost_reflect::{DynamicMessage, ReflectMessage};
use prost_types::FileDescriptorProto;

use super::*;
use crate::parse::parse;

#[track_caller]
fn check(source: &str) -> Result<FileDescriptorProto, Vec<CheckError>> {
    parse(source)
        .unwrap()
        .to_file_descriptor(None, None, None)
        .map(|(file, _)| file)
}

#[track_caller]
fn check_ok(source: &str) -> DynamicMessage {
    check(source).unwrap().transcode_to_dynamic()
}

#[track_caller]
fn check_err(source: &str) -> Vec<CheckError> {
    check(source).unwrap_err()
}

#[test]
fn name_conflict_in_imported_files() {}

#[test]
fn invalid_message_number() {
    assert_eq!(
        check_err("message Foo { optional int32 i = -5; }"),
        vec![CheckError::InvalidMessageNumber { span: 33..35 }]
    );
    assert_eq!(
        check_err("message Foo { optional int32 i = 0; }"),
        vec![CheckError::InvalidMessageNumber { span: 33..34 }]
    );
    assert_eq!(
        check_err("message Foo { optional int32 i = 536870912; }"),
        vec![CheckError::InvalidMessageNumber { span: 33..42 }]
    );
    assert_json_snapshot!(check_ok("message Foo { optional int32 i = 1; }"));
    assert_json_snapshot!(check_ok("message Foo { optional int32 i = 536870911; }"));
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
    assert_json_snapshot!(check_ok(
        "extend Bar { optional group Baz = 1 {} }

            message Bar {
                extensions 1;

                map<int32, string> x = 5;

                oneof foo {
                    group Quz = 3 {}
                }

                message Nest {}
            }"
    ));
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
