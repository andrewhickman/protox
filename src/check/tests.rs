use std::collections::HashMap;

use insta::assert_json_snapshot;
use prost_reflect::{DynamicMessage, ReflectMessage};
use prost_types::{FileDescriptorProto, FileDescriptorSet};

use super::CheckError::*;
use super::*;
use crate::{error::ErrorKind, files::File, parse::parse, Compiler, Error, ImportResolver};

struct TestImportResolver {
    files: HashMap<String, String>,
}

impl ImportResolver for TestImportResolver {
    fn open(&self, name: &str) -> Result<File, Error> {
        Ok(File {
            path: None,
            content: self.files[name].clone(),
        })
    }
}

#[track_caller]
fn check(source: &str) -> Result<FileDescriptorProto, Vec<CheckError>> {
    parse(source)
        .unwrap()
        .to_file_descriptor(None, None, None)
        .map(|(file, _)| file)
}

#[track_caller]
fn check_with_imports(files: Vec<(&str, &str)>) -> Result<FileDescriptorSet, Vec<CheckError>> {
    let root = files.last().unwrap().0.to_owned();
    let resolver = TestImportResolver {
        files: files
            .into_iter()
            .map(|(n, s)| (n.to_owned(), s.to_owned()))
            .collect(),
    };

    let mut compiler = Compiler::with_import_resolver(resolver);
    compiler.include_imports(true);
    match compiler.add_file(&root) {
        Ok(_) => Ok(compiler.file_descriptor_set()),
        Err(err) => match err.kind() {
            ErrorKind::CheckErrors { err, errors, .. } => {
                let mut errors = errors.clone();
                errors.insert(0, err.clone());
                Err(errors)
            }
            err => panic!("unexpected error: {}", err),
        },
    }
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
fn name_conflict_in_imported_files() {
    assert_eq!(
        check_with_imports(vec![
            ("dep1.proto", "message Foo {}"),
            ("dep2.proto", "message Foo {}"),
            ("root.proto", r#"import "dep1.proto"; import "dep2.proto";"#),
        ])
        .unwrap_err(),
        vec![DuplicateNameInImports {
            name: "Foo".to_owned(),
            first_file: "dep1.proto".to_owned(),
            second_file: "dep2.proto".to_owned()
        }]
    );
}

#[test]
fn name_conflict_with_import() {
    assert_eq!(
        check_with_imports(vec![
            ("dep.proto", "message Foo {}"),
            ("root.proto", r#"import "dep.proto"; message Foo {}"#),
        ])
        .unwrap_err(),
        vec![DuplicateNameInFileAndImport {
            name: "Foo".to_owned(),
            first_file: "dep.proto".to_owned(),
            second: 28..31,
        }]
    );
}

#[test]
fn name_conflict_package() {
    assert_eq!(
        check_with_imports(vec![
            ("dep.proto", "package foo;"),
            ("root.proto", r#"import "dep.proto"; message foo {}"#),
        ])
        .unwrap_err(),
        vec![DuplicateNameInFileAndImport {
            name: "foo".to_owned(),
            first_file: "dep.proto".to_owned(),
            second: 28..31,
        }]
    );
    assert_eq!(
        check_with_imports(vec![
            ("dep.proto", "message foo {}"),
            ("root.proto", r#"import "dep.proto"; package foo;"#),
        ])
        .unwrap_err(),
        vec![DuplicateNameInFileAndImport {
            name: "foo".to_owned(),
            first_file: "dep.proto".to_owned(),
            second: 28..31,
        }]
    );
    assert_json_snapshot!(check_with_imports(vec![
        ("dep.proto", "package foo;"),
        ("root.proto", r#"import "dep.proto"; package foo;"#),
    ])
    .unwrap()
    .transcode_to_dynamic());
}

#[test]
fn name_conflict_field_camel_case() {
    assert_eq!(
        check_err(
            "syntax = 'proto3';

            message Foo {\
                optional int32 foo_bar = 1;
                optional int32 foobar = 2;
            }"
        ),
        vec![DuplicateCamelCaseFieldName {
            first_name: "foo_bar".to_owned(),
            first: 60..67,
            second_name: "foobar".to_owned(),
            second: 104..110,
        }]
    );
    assert_eq!(
        check_err(
            "syntax = 'proto3';

            message Foo {\
                optional int32 foo = 1;
                optional int32 FOO = 2;
            }"
        ),
        vec![DuplicateCamelCaseFieldName {
            first_name: "foo".to_owned(),
            first: 60..63,
            second_name: "FOO".to_owned(),
            second: 100..103,
        }]
    );
}

#[test]
fn name_conflict() {
    assert_eq!(
        check_err("message Foo {} message Foo {}"),
        vec![DuplicateNameInFile {
            name: "Foo".to_owned(),
            first: 8..11,
            second: 23..26
        }]
    );
    assert_eq!(
        check_err("message Foo {} enum Foo {}"),
        vec![DuplicateNameInFile {
            name: "Foo".to_owned(),
            first: 8..11,
            second: 20..23
        }]
    );
    assert_eq!(
        check_err("message Foo {} service Foo {}"),
        vec![DuplicateNameInFile {
            name: "Foo".to_owned(),
            first: 8..11,
            second: 23..26
        }]
    );
    assert_eq!(
        check_err("message Foo {} enum Bar { Foo = 1; }"),
        vec![DuplicateNameInFile {
            name: "Foo".to_owned(),
            first: 8..11,
            second: 26..29
        }]
    );
}

#[test]
fn invalid_message_number() {
    assert_eq!(
        check_err("message Foo { optional int32 i = -5; }"),
        vec![InvalidMessageNumber { span: 33..35 }]
    );
    assert_eq!(
        check_err("message Foo { optional int32 i = 0; }"),
        vec![InvalidMessageNumber { span: 33..34 }]
    );
    assert_eq!(
        check_err("message Foo { optional int32 i = 536870912; }"),
        vec![InvalidMessageNumber { span: 33..42 }]
    );
    assert_json_snapshot!(check_ok("message Foo { optional int32 i = 1; }"));
    assert_json_snapshot!(check_ok("message Foo { optional int32 i = 536870911; }"));
}

#[test]
fn generate_map_entry_message() {
    assert_json_snapshot!(check_ok(
        "\
        message Foo {
            map<int32, string> bar = 1;
        }"
    ));
}

#[test]
fn generate_map_entry_message_name_conflict() {

    // conflict with other type name
}

#[test]
fn generate_group_message() {

    // conflict with other type name
}

#[test]
fn generated_message_ordering() {
    assert_json_snapshot!(check_ok(
        "
        extend Bar { optional group Baz = 1 {} }

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
    // leading dot
    // package vs no package
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

#[test]
fn proto2_enum_in_proto3_message() {}

#[test]
fn proto3_enum_default() {}
