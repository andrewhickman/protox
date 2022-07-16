use std::{collections::HashMap, env};

use insta::assert_yaml_snapshot;
use prost_reflect::{DynamicMessage, ReflectMessage};
use prost_types::{FileDescriptorProto, FileDescriptorSet};

use super::CheckError::*;
use super::*;
use crate::{error::ErrorKind, file::File, file::FileResolver, Compiler, Error};

struct TestFileResolver {
    files: HashMap<String, String>,
}

impl FileResolver for TestFileResolver {
    fn open_file(&self, name: &str) -> Result<File, Error> {
        Ok(File {
            path: None,
            content: self.files[name].clone(),
        })
    }
}

#[track_caller]
fn check(source: &str) -> Result<FileDescriptorProto, Vec<CheckError>> {
    Ok(check_with_imports(vec![("root.proto", source)])?
        .file
        .into_iter()
        .last()
        .unwrap())
}

#[track_caller]
fn check_with_imports(files: Vec<(&str, &str)>) -> Result<FileDescriptorSet, Vec<CheckError>> {
    let root = files.last().unwrap().0.to_owned();
    let resolver = TestFileResolver {
        files: files
            .into_iter()
            .map(|(n, s)| (n.to_owned(), s.to_owned()))
            .collect(),
    };

    let mut compiler = Compiler::with_file_resolver(resolver);
    compiler.include_imports(true);
    compiler.include_source_info(true);
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
    assert_yaml_snapshot!(check_with_imports(vec![
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
    assert_eq!(
        check_err("message Foo { optional int32 i = 19000; }"),
        vec![ReservedMessageNumber { span: 33..38 }]
    );
    assert_eq!(
        check_err("message Foo { optional int32 i = 19999; }"),
        vec![ReservedMessageNumber { span: 33..38 }]
    );
    assert_yaml_snapshot!(check_ok("message Foo { optional int32 i = 1; }"));
    assert_yaml_snapshot!(check_ok("message Foo { optional int32 i = 536870911; }"));
    assert_yaml_snapshot!(check_ok("message Foo { optional int32 i = 18999; }"));
    assert_yaml_snapshot!(check_ok("message Foo { optional int32 i = 20000; }"));
}

#[test]
fn generate_map_entry_message_name_conflict() {
    assert_eq!(
        check_err(
            "message Foo {\
                map<uint32, bytes> baz = 1;

                enum BazEntry {
                    ZERO = 0;
                }
            }"
        ),
        vec![DuplicateNameInFile {
            name: "Foo.BazEntry".to_owned(),
            first: 32..35,
            second: 63..71,
        }]
    );
}

#[test]
fn generate_group_message_name_conflict() {
    assert_eq!(
        check_err(
            "\
            message Foo {\
                optional group Baz = 1 {}

                enum Baz {
                    ZERO = 0;
                }
            }"
        ),
        vec![DuplicateNameInFile {
            name: "Foo.Baz".to_owned(),
            first: 28..31,
            second: 61..64,
        }],
    );
}

#[test]
fn generate_synthetic_oneof_name_conflict() {
    assert_eq!(
        check_err(
            "\
            syntax = 'proto3';

            message Foo {
                optional fixed64 val = 1;

                message _val {}
            }"
        ),
        vec![DuplicateNameInFile {
            name: "Foo._val".to_owned(),
            first: 79..82,
            second: 113..117,
        }],
    );
}

#[test]
fn invalid_service_type() {
    // use enum/service/oneof etc
    assert_eq!(
        check_err(
            "\
            syntax = 'proto3';

            enum Enum {
                ZERO = 0;
            }
            message Message {}

            service Service {
                rpc rpc(.Enum) returns (.Message);
            }"
        ),
        vec![InvalidMethodTypeName {
            name: ".Enum".to_owned(),
            kind: "input",
            span: 170..175,
        }],
    );
    assert_eq!(
        check_err(
            "\
            syntax = 'proto3';

            enum Enum {
                ZERO = 0;
            }
            message Message {}

            service Service {
                rpc rpc(.Message) returns (.Enum);
            }"
        ),
        vec![InvalidMethodTypeName {
            name: ".Enum".to_owned(),
            kind: "output",
            span: 189..194,
        }],
    );
    assert_eq!(
        check_err(
            "\
            syntax = 'proto3';

            message Message {}

            service Service {
                rpc rpc(.Message) returns (.Service);
            }"
        ),
        vec![InvalidMethodTypeName {
            name: ".Service".to_owned(),
            kind: "output",
            span: 125..133,
        }],
    );
}

#[test]
fn name_resolution() {
    assert_eq!(
        check_with_imports(vec![
            ("dep.proto", "package foo.bar; message FooBar {}"),
            (
                "root.proto",
                r#"
                syntax = 'proto3';

                import "dep.proto";

                message foo {
                    .foo.FooBar foobar = 1;
                }"#
            ),
        ])
        .unwrap_err(),
        vec![TypeNameNotFound {
            name: "foo.FooBar".to_owned(),
            span: 124..135,
        }]
    );
    assert_eq!(
        check_with_imports(vec![
            ("dep.proto", "package foo.bar; message FooBar {}"),
            (
                "root.proto",
                r#"
                syntax = 'proto3';

                import "dep.proto";

                message foo {
                    .FooBar foobar = 1;
                }"#
            ),
        ])
        .unwrap_err(),
        vec![TypeNameNotFound {
            name: "FooBar".to_owned(),
            span: 124..131,
        }]
    );
}

#[test]
fn name_collision() {
    assert_eq!(
        check_err(
            "\
            message Message {}
            message Message {}
            "
        ),
        vec![DuplicateNameInFile {
            name: "Message".to_owned(),
            first: 8..15,
            second: 39..46,
        }],
    );
    assert_eq!(
        check_err(
            "\
            message Message {}
            enum Message {
                ZERO = 1;
            }"
        ),
        vec![DuplicateNameInFile {
            name: "Message".to_owned(),
            first: 8..15,
            second: 36..43,
        }],
    );
    assert_eq!(
        check_err(
            "\
            message Message {
                optional int32 foo = 1;

                enum foo {
                    ZERO = 1;
                }
            }"
        ),
        vec![DuplicateNameInFile {
            name: "Message.foo".to_owned(),
            first: 49..52,
            second: 80..83,
        }],
    );
    assert_eq!(
        check_with_imports(vec![
            ("dep.proto", "package foo;"),
            ("root.proto", "import 'dep.proto'; message foo {}"),
        ])
        .unwrap_err(),
        vec![DuplicateNameInFileAndImport {
            name: "foo".to_owned(),
            first_file: "dep.proto".to_owned(),
            second: 28..31,
        }],
    );
}

#[test]
fn message_field_default_value() {
    // bytes/string etc
}

#[test]
fn message_field_json_name() {
    // custom json name with option
}

#[test]
fn map_field_with_label() {}

#[test]
fn map_field_with_default() {}

#[test]
fn map_field_invalid_type() {}

#[test]
fn message_field_with_default() {}

#[test]
fn message_field_duplicate_number() {}

#[test]
fn message_reserved_range_extrema() {}

#[test]
fn message_reserved_range_invalid() {
    // empty
    // end < start
}

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
fn enum_reserved_range_invalid() {
    // empty
    // end < start
}

#[test]
fn enum_reserved_range_overlap_with_value() {}

#[test]
fn enum_duplicate_number() {}

#[test]
fn proto2_enum_in_proto3_message() {}

#[test]
fn proto3_enum_default() {}

/*
syntax = 'proto2';

import 'google/protobuf/descriptor.proto';

message Foo {
    optional int32 a = 1;
    optional int32 b = 2;
}

extend google.protobuf.FileOptions {
    optional Foo foo = 1001;
}

option (foo).a = 1;

option optimize_for = SPEED;

option (foo).b = 1;
*/

/*

message Foo {
    // hello
    optional group A = 1 {}     ;
}

*/

/*

syntax = 'proto2';

message Foo {
    optional int32 a = 1;

    oneof foo {
        int32 c = 2;
    }

    extensions 3, 6 to max;

    reserved 4 to 5;
    reserved "d", "e";

    extend Foo {
        optional sint32 b = 3;
    }

    message Bar {}

    enum Quz {
        ZERO = 0;
    }

    option deprecated = true;
}

*/

/*
import "google/protobuf/descriptor.proto";
extend google.protobuf.OneofOptions {
  optional int32 my_option = 12345;
}

message Hello {
  oneof something {
    int32 bar = 1;

    option (my_option) = 54321;
  }
}
 */

/*

syntax = 'proto2';

import 'google/protobuf/descriptor.proto';

package exttest;

message Message {
    optional int32 a = 1;
    optional Message b = 3;

    extensions 5 to 6;
}

extend Message {
    optional int32 c = 5;
    optional Message d = 6;
}

extend google.protobuf.FileOptions {
    optional Message foo = 50000;
}

option (exttest.foo).(exttest.d).a = 1;

*/

/*

message Foo {
    optional bytes foo = 20000 [default = "\777"];
}

*/

/*
message Foo {
    optional bytes foo = 20000 [default = "\xFF"];
}
*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a).key = 1;

extend google.protobuf.FileOptions {
    optional Foo.BarEntry a = 1001;
}

message Foo {
    map<int32, string> bar = 1;
    /*optional group A = 1 {

    };*/
}

foo.proto:8:14: map_entry should not be set explicitly. Use map<KeyType, ValueType> instead.


*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a).key = 1;

extend google.protobuf.FileOptions {
    optional Foo.A a = 1001;
}

message Foo {
    optional group A = 1 {
        optional int32 key = 1;
    };
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    key: 1, // should fail with numeric keys
};

extend google.protobuf.FileOptions {
    repeated Foo.A a = 1001;
}

message Foo {
    optional group A = 1 {
        optional int32 key = 1;
    };
}

*/

/*

syntax = "proto3";

import "google/protobuf/descriptor.proto";

package demo;

extend google.protobuf.EnumValueOptions {
  optional uint32 len = 50000;
}

enum Foo {
  None = 0 [(len) = 0];
  One = 1 [(len) = 1];
  Two = 2 [(len) = 2];
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    /* block */
    # hash
    key: 1.0
    // line
};

extend google.protobuf.FileOptions {
    repeated Foo.A a = 1001;
}

message Foo {
    optional group A = 1 {
        optional float key = 1;
    };
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    key:
        "hello"
        "gdfg"
};

extend google.protobuf.FileOptions {
    repeated Foo.A a = 1001;
}

message Foo {
    optional group A = 1 {
        optional string key = 1;
    };
}

*/

/*

syntax =
    "proto"
    "2";

import "google/protobuf/descriptor.proto";

option (a) =
    "hello"
    "gdfg"
;

extend google.protobuf.FileOptions {
    repeated string a = 1001;
}

message Foo {
    optional group A = 1 {
        optional string key = 1;
    };
}

*/

/*

syntax = "proto" "2";

import "google/protobuf/descriptor.proto";

option (a) = {
    key : -inf;
};

extend google.protobuf.FileOptions {
    repeated Foo.A a = 1001;
}

message Foo {
    optional group A = 1 {
        optional float key = 1;
    };
}


*/

/*

syntax = "proto2";

import "google/protobuf/any.proto";
import "google/protobuf/descriptor.proto";

option (a) = {
    [type.googleapis.com/Foo] { foo: "bar" }
};

extend google.protobuf.FileOptions {
    repeated google.protobuf.Any a = 1001;
}

message Foo {
    optional string foo = 1;
}

*/

/*
syntax = "proto2";

import "google/protobuf/any.proto";
import "google/protobuf/descriptor.proto";

option (a) = {
    [type.googleapis.com/Foo]: { foo: "bar" }
};

extend google.protobuf.FileOptions {
    repeated google.protobuf.Any a = 1001;
}

message Foo {
    optional string foo = 1;
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    foo: <
    >
};

extend google.protobuf.FileOptions {
    repeated Foo a = 1001;
}

message Foo {
    optional Foo foo = 1;
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    foo <
    >
};

extend google.protobuf.FileOptions {
    repeated Foo a = 1001;
}

message Foo {
    optional Foo foo = 1;
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    foo: [1, 2, 3]
};

extend google.protobuf.FileOptions {
    repeated Foo a = 1001;
}

message Foo {
    repeated int32 foo = 1;
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    foo: 1
    foo: 2
    foo: 3
};

extend google.protobuf.FileOptions {
    repeated Foo a = 1001;
}

message Foo {
    repeated int32 foo = 1;
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    Foo {
    }
};

extend google.protobuf.FileOptions {
    repeated Foo a = 1001;
}

message Foo {
    optional group Foo = 1 {};
}

*/

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    Foo : {
    }
};

extend google.protobuf.FileOptions {
    repeated Foo a = 1001;
}

message Foo {
    optional group Foo = 1 {};
}

*/
