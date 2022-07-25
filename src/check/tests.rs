use std::{collections::HashMap, env};

use insta::{assert_json_snapshot, assert_yaml_snapshot};
use prost_reflect::{DynamicMessage, ReflectMessage};
use prost_types::{FileDescriptorProto, FileDescriptorSet};

use super::CheckError::*;
use super::*;
use crate::{
    check::names::NameLocation, error::ErrorKind, file::File, file::FileResolver, Compiler, Error,
};

struct TestFileResolver {
    files: HashMap<String, String>,
}

impl FileResolver for TestFileResolver {
    fn open_file(&self, name: &str) -> Result<File, Error> {
        File::from_source(self.files[name].as_str())
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
        vec![DuplicateName(DuplicateNameError {
            name: "Foo".to_owned(),
            first: NameLocation::Import("dep1.proto".to_owned()),
            second: NameLocation::Import("dep2.proto".to_owned()),
        })]
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
        vec![DuplicateName(DuplicateNameError {
            name: "Foo".to_owned(),
            first: NameLocation::Import("dep.proto".to_owned()),
            second: NameLocation::Root(28..31),
        })]
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
        vec![DuplicateName(DuplicateNameError {
            name: "foo".to_owned(),
            first: NameLocation::Import("dep.proto".to_owned()),
            second: NameLocation::Root(28..31),
        })]
    );
    assert_eq!(
        check_with_imports(vec![
            ("dep.proto", "message foo {}"),
            ("root.proto", r#"import "dep.proto"; package foo;"#),
        ])
        .unwrap_err(),
        vec![DuplicateName(DuplicateNameError {
            name: "foo".to_owned(),
            first: NameLocation::Import("dep.proto".to_owned()),
            second: NameLocation::Root(20..32),
        })]
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
            first: Some(SourceSpan::from(60..67)),
            second_name: "foobar".to_owned(),
            second: Some(SourceSpan::from(104..110)),
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
            first: Some(SourceSpan::from(60..63)),
            second_name: "FOO".to_owned(),
            second: Some(SourceSpan::from(100..103)),
        }]
    );
    assert_yaml_snapshot!(check_ok(
        "syntax = 'proto2';

        message Foo {\
            optional int32 foo = 1;
            optional int32 FOO = 2;
        }"
    ));
}

#[test]
fn name_conflict() {
    assert_eq!(
        check_err("message Foo {} message Foo {}"),
        vec![DuplicateName(DuplicateNameError {
            name: "Foo".to_owned(),
            first: NameLocation::Root(8..11),
            second: NameLocation::Root(23..26),
        })]
    );
    assert_eq!(
        check_err("message Foo {} enum Foo {}"),
        vec![DuplicateName(DuplicateNameError {
            name: "Foo".to_owned(),
            first: NameLocation::Root(8..11),
            second: NameLocation::Root(20..23),
        })]
    );
    assert_eq!(
        check_err("message Foo {} service Foo {}"),
        vec![DuplicateName(DuplicateNameError {
            name: "Foo".to_owned(),
            first: NameLocation::Root(8..11),
            second: NameLocation::Root(23..26),
        })]
    );
    assert_eq!(
        check_err("message Foo {} enum Bar { Foo = 1; }"),
        vec![DuplicateName(DuplicateNameError {
            name: "Foo".to_owned(),
            first: NameLocation::Root(8..11),
            second: NameLocation::Root(26..29),
        })]
    );
}

#[test]
fn invalid_message_number() {
    assert_eq!(
        check_err("message Foo { optional int32 i = -5; }"),
        vec![InvalidMessageNumber {
            span: Some(SourceSpan::from(33..35))
        }]
    );
    assert_eq!(
        check_err("message Foo { optional int32 i = 0; }"),
        vec![InvalidMessageNumber {
            span: Some(SourceSpan::from(33..34))
        }]
    );
    assert_eq!(
        check_err("message Foo { optional int32 i = 536870912; }"),
        vec![InvalidMessageNumber {
            span: Some(SourceSpan::from(33..42))
        }]
    );
    assert_eq!(
        check_err("message Foo { optional int32 i = 19000; }"),
        vec![ReservedMessageNumber {
            span: Some(SourceSpan::from(33..38))
        }]
    );
    assert_eq!(
        check_err("message Foo { optional int32 i = 19999; }"),
        vec![ReservedMessageNumber {
            span: Some(SourceSpan::from(33..38))
        }]
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
        vec![DuplicateName(DuplicateNameError {
            name: "Foo.BazEntry".to_owned(),
            first: NameLocation::Unknown,
            second: NameLocation::Root(63..71),
        })]
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
        vec![DuplicateName(DuplicateNameError {
            name: "Foo.Baz".to_owned(),
            first: NameLocation::Root(28..31),
            second: NameLocation::Root(61..64),
        })],
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
        vec![DuplicateName(DuplicateNameError {
            name: "Foo._val".to_owned(),
            first: NameLocation::Unknown,
            second: NameLocation::Root(113..117),
        })],
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
            span: Some(SourceSpan::from(170..175)),
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
            span: Some(SourceSpan::from(189..194)),
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
            span: Some(SourceSpan::from(125..133)),
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

                message Foo {
                    .foo.FooBar foobar = 1;
                }"#
            ),
        ])
        .unwrap_err(),
        vec![TypeNameNotFound {
            name: ".foo.FooBar".to_owned(),
            span: Some(SourceSpan::from(124..135)),
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

                message Foo {
                    .FooBar foobar = 1;
                }"#
            ),
        ])
        .unwrap_err(),
        vec![TypeNameNotFound {
            name: ".FooBar".to_owned(),
            span: Some(SourceSpan::from(124..131)),
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
        vec![DuplicateName(DuplicateNameError {
            name: "Message".to_owned(),
            first: NameLocation::Root(8..15),
            second: NameLocation::Root(39..46),
        })],
    );
    assert_eq!(
        check_err(
            "\
            message Message {}
            enum Message {
                ZERO = 1;
            }"
        ),
        vec![DuplicateName(DuplicateNameError {
            name: "Message".to_owned(),
            first: NameLocation::Root(8..15),
            second: NameLocation::Root(36..43),
        })],
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
        vec![DuplicateName(DuplicateNameError {
            name: "Message.foo".to_owned(),
            first: NameLocation::Root(49..52),
            second: NameLocation::Root(80..83),
        })],
    );
    assert_eq!(
        check_with_imports(vec![
            ("dep.proto", "package foo;"),
            ("root.proto", "import 'dep.proto'; message foo {}"),
        ])
        .unwrap_err(),
        vec![DuplicateName(DuplicateNameError {
            name: "foo".to_owned(),
            first: NameLocation::Import("dep.proto".to_owned()),
            second: NameLocation::Root(28..31),
        })],
    );
    assert_eq!(
        check_with_imports(vec![
            ("dep.proto", "package foo.bar;"),
            ("root.proto", "import 'dep.proto'; message foo {}"),
        ])
        .unwrap_err(),
        vec![DuplicateName(DuplicateNameError {
            name: "foo".to_owned(),
            first: NameLocation::Import("dep.proto".to_owned()),
            second: NameLocation::Root(28..31),
        })],
    );
}

#[test]
fn proto3_default_value() {
    assert_eq!(
        check_err(
            r#"
            syntax = 'proto3';

            message Message {
                optional int32 foo = 1 [default = -0];
            }"#
        ),
        vec![Proto3DefaultValue {
            span: Some(SourceSpan::from(103..115))
        }],
    );
}

#[test]
fn field_default_value() {
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional Message foo = 1 [default = ""];
            }"#
        ),
        vec![InvalidDefault {
            kind: "message",
            span: Some(SourceSpan::from(83..85)),
        }],
    );
    assert_eq!(
        check_err(
            r#"
            message Message {
                map<uint32, sfixed64> foo = 1 [default = ""];
            }"#
        ),
        vec![InvalidDefault {
            kind: "map",
            span: Some(SourceSpan::from(78..90)),
        }],
    );
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional group Foo = 1 [default = ""] {};
            }"#
        ),
        vec![InvalidDefault {
            kind: "group",
            span: Some(SourceSpan::from(71..83)),
        }],
    );
    assert_eq!(
        check_err(
            r#"
            message Message {
                repeated int32 foo = 1 [default = 1];
            }"#
        ),
        vec![InvalidDefault {
            kind: "repeated",
            span: Some(SourceSpan::from(71..82)),
        }],
    );
    assert_yaml_snapshot!(check_ok(
        r#"
            message Message {
                optional float default_float_exp = 23 [ default = 9e6];
                optional double default_double_exp = 24 [ default = 9e22];
            }
        "#
    ));
}

#[test]
fn enum_field_invalid_default() {
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional Foo foo = 1 [default = ONE];
            }

            enum Foo {
                ZERO = 0;
            }"#
        ),
        vec![InvalidEnumValue {
            value_name: "ONE".to_owned(),
            enum_name: "Foo".to_owned(),
            span: Some(SourceSpan::from(79..82)),
            help: Some("possible values are 'ZERO'".to_owned())
        }],
    );
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional Foo foo = 1 [default = ONE];
            }

            enum Foo {
                ZERO = 0;
                TWO = 2;
            }

            enum Bar {
                NONE = 0;
                ONE = 1;
            }"#
        ),
        vec![InvalidEnumValue {
            value_name: "ONE".to_owned(),
            enum_name: "Foo".to_owned(),
            span: Some(SourceSpan::from(79..82)),
            help: Some("possible values are 'TWO' and 'ZERO'".to_owned()),
        }],
    );
    assert_eq!(
        check_with_imports(vec![
            (
                "dep.proto",
                "
                package foo;
                enum Foo { ZERO = 1; }"
            ),
            (
                "root.proto",
                r#"
                import "dep.proto";

                message Bar {
                    optional foo.Foo foo = 1 [default = ONE];
                }"#
            )
        ])
        .unwrap_err(),
        vec![InvalidEnumValue {
            value_name: "ONE".to_owned(),
            enum_name: "foo.Foo".to_owned(),
            span: Some(SourceSpan::from(124..127)),
            help: Some("possible values are 'ZERO'".to_owned()),
        }],
    );
    assert_eq!(
        check_err(
            "
            package foo;

            message Message {
                optional Foo foo = 1 [default = ONE];
            }

            enum Foo {
                ZERO = 0;
            }"
        ),
        vec![InvalidEnumValue {
            value_name: "ONE".to_owned(),
            enum_name: "foo.Foo".to_owned(),
            span: Some(SourceSpan::from(105..108)),
            help: Some("possible values are 'ZERO'".to_owned()),
        }],
    );
    assert_yaml_snapshot!(check_ok(
        "
        package foo;

        message Message {
            optional Foo foo = 1 [default = ZERO];
        }

        enum Foo {
            ZERO = 0;
        }"
    ));
    assert_eq!(
        check_err(
            "
            package foo;

            message Message {
                optional Foo foo = 1 [default = ONE];

                enum Foo {
                    ZERO = 0;
                }
            }"
        ),
        vec![InvalidEnumValue {
            value_name: "ONE".to_owned(),
            enum_name: "foo.Message.Foo".to_owned(),
            span: Some(SourceSpan::from(105..108)),
            help: Some("possible values are 'ZERO'".to_owned()),
        }],
    );
    assert_yaml_snapshot!(check_ok(
        "
        package foo;

        message Message {
            optional Foo foo = 1 [default = ZERO];

            enum Foo {
                ZERO = 0;
            }
        }"
    ));
    assert_eq!(
        check_err(
            "
            package foo;

            message Message {
                optional Parent.Foo foo = 1 [default = ONE];
            }

            message Parent {
                enum Foo {
                    ZERO = 0;
                }
            }"
        ),
        vec![InvalidEnumValue {
            value_name: "ONE".to_owned(),
            enum_name: "foo.Parent.Foo".to_owned(),
            span: Some(SourceSpan::from(112..115)),
            help: Some("possible values are 'ZERO'".to_owned()),
        }],
    );
    assert_yaml_snapshot!(check_ok(
        "
        package foo;

        message Message {
            optional Parent.Foo foo = 1 [default = ZERO];
        }

        message Parent {
            enum Foo {
                ZERO = 0;
            }
        }"
    ));
}

#[test]
fn field_default_invalid_type() {
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional int32 foo = 1 [default = "foo"];
            }"#
        ),
        vec![ValueInvalidType {
            expected: "an integer".to_owned(),
            actual: "foo".to_owned(),
            span: Some(SourceSpan::from(81..86)),
        }],
    );
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional uint32 foo = 1 [default = -100];
            }"#
        ),
        vec![IntegerValueOutOfRange {
            expected: "an unsigned 32-bit integer".to_owned(),
            actual: "-100".to_owned(),
            min: "0".to_owned(),
            max: "4294967295".to_owned(),
            span: Some(SourceSpan::from(82..86)),
        }],
    );
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional int32 foo = 1 [default = 2147483648];
            }"#
        ),
        vec![IntegerValueOutOfRange {
            expected: "a signed 32-bit integer".to_owned(),
            actual: "2147483648".to_owned(),
            min: "-2147483648".to_owned(),
            max: "2147483647".to_owned(),
            span: Some(SourceSpan::from(81..91)),
        }],
    );
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional Foo foo = 1 [default = 1];
            }

            enum Foo {
                ZERO = 0;
            }"#
        ),
        vec![ValueInvalidType {
            expected: "an enum value identifier".to_owned(),
            actual: "1".to_owned(),
            span: Some(SourceSpan::from(79..80)),
        }],
    );
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional Foo foo = 1 [default = "ZERO"];
            }

            enum Foo {
                ZERO = 0;
            }"#
        ),
        vec![ValueInvalidType {
            expected: "an enum value identifier".to_owned(),
            actual: "\"ZERO\"".to_owned(),
            span: Some(SourceSpan::from(79..85)),
        }],
    );
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional bool foo = 1 [default = FALSE];
            }

            enum Foo {
                ZERO = 0;
            }"#
        ),
        vec![ValueInvalidType {
            expected: "either 'true' or 'false'".to_owned(),
            actual: "FALSE".to_owned(),
            span: Some(SourceSpan::from(80..85)),
        }],
    );
    assert_eq!(
        check_err(
            r#"
            message Message {
                optional string foo = 1 [default = '\xFF'];
            }"#
        ),
        vec![StringValueInvalidUtf8 {
            span: Some(SourceSpan::from(82..88))
        }],
    );
}

#[test]
fn negative_ident_outside_default() {
    assert_eq!(
        check_err("option opt = -foo;"),
        vec![NegativeIdentOutsideDefault {
            span: Some(SourceSpan::from(13..17))
        }],
    );
}

#[test]
fn message_field_json_name() {
    assert_eq!(
        check_err(
            r#"message Message {
            optional int32 field = 1 [json_name = "\xFF"];
        }"#
        ),
        vec![StringValueInvalidUtf8 {
            span: Some(SourceSpan::from(68..74))
        }],
    );
    assert_json_snapshot!(check_ok(
        r#"message Message {
        optional int32 field = 1 [json_name = '$FIELD'];
    }"#
    ));
}

#[test]
fn map_field_with_label() {
    assert_eq!(
        check_err(
            r#"message Message {
            optional map<int32, string> field = 1;
        }"#
        ),
        vec![MapFieldWithLabel {
            span: Some(SourceSpan::from(30..38))
        }],
    );
    assert_eq!(
        check_err(
            r#"
            syntax = 'proto3';

            message Message {
                required map<int32, string> field = 1;
            }"#
        ),
        vec![MapFieldWithLabel {
            span: Some(SourceSpan::from(79..87))
        }],
    );
}

#[test]
fn map_field_invalid_type() {
    assert_eq!(
        check_err(
            r#"message Message {
            map<Message, sfixed32> field = 1;
        }"#
        ),
        vec![InvalidMapFieldKeyType {
            span: Some(SourceSpan::from(34..41))
        }],
    );
    assert_eq!(
        check_err(
            r#"message Message {
            map<.Message, fixed32> field = 1;
        }"#
        ),
        vec![InvalidMapFieldKeyType {
            span: Some(SourceSpan::from(34..42))
        }],
    );
    assert_eq!(
        check_err(
            r#"message Message {
            map<.Message, bool> field = 1;
        }"#
        ),
        vec![InvalidMapFieldKeyType {
            span: Some(SourceSpan::from(34..42))
        }],
    );
    assert_eq!(
        check_err(
            r#"message Message {
            map<float, string> field = 1;
        }"#
        ),
        vec![InvalidMapFieldKeyType {
            span: Some(SourceSpan::from(34..39))
        }],
    );
    assert_eq!(
        check_err(
            r#"message Message {
            map<double, int64> field = 1;
        }"#
        ),
        vec![InvalidMapFieldKeyType {
            span: Some(SourceSpan::from(34..40))
        }],
    );
    assert_eq!(
        check_err(
            r#"message Message {
            map<Enum, int64> field = 1;

            enum Enum {
                ZERO = 0;
            }
        }"#
        ),
        vec![InvalidMapFieldKeyType {
            span: Some(SourceSpan::from(34..38))
        }],
    );
    assert_json_snapshot!(check_ok(
        r#"message Message {
        map<int64, float> int64 = 1;
        map<uint32, double> uint32 = 2;
        map<uint64, .Message> uint64 = 3;
        map<sint32, Message> sint32 = 4;
        map<sint64, bytes> sint64 = 5;
        map<fixed32, int64> fixed32 = 6;
        map<fixed64, uint32> fixed64 = 7;
        map<sfixed32, sint32> sfixed32 = 8;
        map<sfixed64, sint64> sfixed64 = 9;
        map<bool, fixed32> bool = 10;
        map<string, fixed64> string = 11;
    }"#
    ))
}

#[test]
fn message_field_duplicate_number() {
    assert_eq!(
        check_err(
            r#"message Message {
                optional int32 foo = 1;
                optional int32 bar = 1;
            }"#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::Field {
                name: "foo".to_owned(),
                number: 1
            },
            first_span: Some(SourceSpan::from(55..56)),
            second: resolve::NumberKind::Field {
                name: "bar".to_owned(),
                number: 1
            },
            second_span: Some(SourceSpan::from(95..96)),
        })],
    );
    assert_eq!(
        check_err(
            r#"message Message {
                message Nested {
                    optional int32 foo = 1;
                    optional int32 bar = 1;
                }
            }"#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::Field {
                name: "foo".to_owned(),
                number: 1
            },
            first_span: Some(SourceSpan::from(92..93)),
            second: resolve::NumberKind::Field {
                name: "bar".to_owned(),
                number: 1
            },
            second_span: Some(SourceSpan::from(136..137)),
        })],
    );
}

#[test]
fn message_reserved_range_extrema() {
    assert_eq!(
        check_err(
            r#"message Message {
                reserved 0 to 1;
            }"#
        ),
        vec![InvalidMessageNumber {
            span: Some(SourceSpan::from(43..44))
        }],
    );
    assert_eq!(
        check_err(
            r#"message Message {
                reserved 1 to 536870912;
            }"#
        ),
        vec![InvalidMessageNumber {
            span: Some(SourceSpan::from(48..57))
        }],
    );
    assert_yaml_snapshot!(check_ok(
        r#"message Message {
            reserved 1 to 536870911;
        }"#
    ));
}

#[test]
fn message_reserved_range_invalid() {
    assert_eq!(
        check_err(
            r#"message Message {
                reserved 5 to 1;
            }"#
        ),
        vec![InvalidRange {
            span: Some(SourceSpan::from(43..49))
        }],
    );
    assert_eq!(
        check_err(
            r#"message Message {
                reserved 2 to 1;
            }"#
        ),
        vec![InvalidRange {
            span: Some(SourceSpan::from(43..49))
        }],
    );
    assert_yaml_snapshot!(check_ok(
        r#"message Message {
            reserved 1 to 1;
        }"#
    ));
}

#[test]
fn message_reserved_range_overlap() {
    assert_eq!(
        check_err(
            r#"message Message {
                reserved 1;
                reserved 1;
            }"#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 1, end: 1 },
            first_span: Some(SourceSpan::from(43..44)),
            second: resolve::NumberKind::ReservedRange { start: 1, end: 1 },
            second_span: Some(SourceSpan::from(71..72)),
        })],
    );
    assert_eq!(
        check_err(
            r#"message Message {
                reserved 1 to 3;
                reserved 2 to 4;
            }"#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 1, end: 3 },
            first_span: Some(SourceSpan::from(43..49)),
            second: resolve::NumberKind::ReservedRange { start: 2, end: 4 },
            second_span: Some(SourceSpan::from(76..82)),
        })],
    );
    assert_eq!(
        check_err(
            r#"message Message {
                reserved 1 to 3;
                extensions 3 to max;
            }"#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 1, end: 3 },
            first_span: Some(SourceSpan::from(43..49)),
            second: resolve::NumberKind::ExtensionRange {
                start: 3,
                end: 536870911,
            },
            second_span: Some(SourceSpan::from(78..86)),
        })],
    );
    assert_yaml_snapshot!(check_ok(
        r#"message Message {
            reserved 1;
            extensions 2 to 3;
            reserved 4 to max;
        }"#
    ));
}

#[test]
fn message_reserved_range_overlap_with_field() {
    assert_eq!(
        check_err(
            r#"message Message {
                optional int32 field = 2;
                reserved 2;
            }"#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 2, end: 2 },
            first_span: Some(SourceSpan::from(85..86)),
            second: resolve::NumberKind::Field {
                name: "field".to_owned(),
                number: 2,
            },
            second_span: Some(SourceSpan::from(57..58)),
        })],
    );
    assert_eq!(
        check_err(
            r#"message Message {
                optional int32 field = 2;
                extensions 1 to 5;
            }"#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ExtensionRange { start: 1, end: 5 },
            first_span: Some(SourceSpan::from(87..93)),
            second: resolve::NumberKind::Field {
                name: "field".to_owned(),
                number: 2,
            },
            second_span: Some(SourceSpan::from(57..58)),
        })],
    );
}

#[test]
fn extend_required_field() {
    assert_eq!(
        check_err(
            r#"
            message Message {
                extensions 1;
            }

            extend Message {
                required int32 foo = 1;
            }
            "#
        ),
        vec![RequiredExtendField {
            span: Some(SourceSpan::from(121..129)),
        }],
    );
}

#[test]
fn extend_map_field() {
    assert_eq!(
        check_err(
            r#"
            message Message {
                extensions 1;
            }

            extend Message {
                map<int32, string> foo = 1;
            }
            "#
        ),
        vec![InvalidExtendFieldKind {
            kind: "map",
            span: Some(SourceSpan::from(121..148)),
        }],
    );
}

#[test]
fn extend_group_field() {
    assert_yaml_snapshot!(check_ok(
        r#"
        message Message {
            extensions 1;
        }

        extend Message {
            repeated group Foo = 1 {
                required int32 bar = 1;
            };
        }
    "#
    ));
}

#[test]
fn extend_field_number_not_in_extensions() {
    assert_eq!(
        check_err(
            r#"
            message Message {
                extensions 2 to 5;
            }

            extend Message {
                optional int32 a = 1;
                repeated int32 b = 6;
            }
            "#
        ),
        vec![
            InvalidExtensionNumber {
                number: 1,
                message_name: "Message".to_owned(),
                help: Some("available extension numbers are 2 to 5".to_owned()),
                span: Some(SourceSpan::from(145..146)),
            },
            InvalidExtensionNumber {
                number: 6,
                message_name: "Message".to_owned(),
                help: Some("available extension numbers are 2 to 5".to_owned()),
                span: Some(SourceSpan::from(183..184)),
            }
        ],
    );
    assert_eq!(
        check_err(
            r#"
            message Message {
                extensions 2 to 5;

                extend Message {
                    optional int32 a = 1;
                    repeated int32 b = 6;
                }
            }
            "#
        ),
        vec![
            InvalidExtensionNumber {
                number: 1,
                message_name: "Message".to_owned(),
                help: Some("available extension numbers are 2 to 5".to_owned()),
                span: Some(SourceSpan::from(139..140)),
            },
            InvalidExtensionNumber {
                number: 6,
                message_name: "Message".to_owned(),
                help: Some("available extension numbers are 2 to 5".to_owned()),
                span: Some(SourceSpan::from(181..182)),
            }
        ],
    );
}

#[test]
#[ignore]
fn extend_duplicate_field_number() {
    // check same extend block
    // different extend block in scope
    // extend block in different scope (e.g. file vs message)
    // defined in imported file
    // defined in file not directly imported
    todo!()
}

#[test]
#[ignore]
fn extend_non_options_type_proto3() {
    todo!()
}

#[test]
fn proto3_group_field() {
    assert_eq!(
        check_err(
            r#"
            syntax = 'proto3';

            message Message {
                optional group Foo = 1 {};
            }
            "#
        ),
        vec![Proto3GroupField {
            span: Some(SourceSpan::from(79..104)),
        }],
    );
}

#[test]
fn proto3_required_field() {
    assert_eq!(
        check_err(
            r#"
            syntax = 'proto3';

            message Message {
                required int32 foo = 1;
            }
            "#
        ),
        vec![Proto3RequiredField {
            span: Some(SourceSpan::from(79..87)),
        }],
    );
}

#[test]
fn proto2_field_missing_label() {
    assert_eq!(
        check_err(
            r#"
            syntax = 'proto2';

            message Message {
                int32 foo = 1;
            }
            "#
        ),
        vec![Proto2FieldMissingLabel {
            span: Some(SourceSpan::from(79..93)),
        }],
    );
}

#[test]
fn oneof_field_with_label() {
    assert_eq!(
        check_err(
            r#"
            syntax = 'proto3';

            message Message {
                oneof foo {
                    optional int32 bar = 1;
                }
            }
            "#
        ),
        vec![OneofFieldWithLabel {
            span: Some(SourceSpan::from(111..119)),
        }],
    );
}

#[test]
fn oneof_map_field() {
    assert_eq!(
        check_err(
            r#"
            syntax = 'proto3';

            message Message {
                oneof foo {
                    map<int32, bytes> bar = 1;
                }
            }
            "#
        ),
        vec![InvalidOneofFieldKind {
            kind: "map",
            span: Some(SourceSpan::from(111..137)),
        }],
    );
}

#[test]
fn oneof_group_field() {
    assert_yaml_snapshot!(check_ok(
        r#"
        message Message {
            oneof oneof {
                group Group = 1 {
                    repeated float bar = 1;
                }
            }
        }
        "#
    ))
}

#[test]
fn empty_oneof() {
    assert_eq!(
        check_err(
            r#"
            syntax = 'proto3';

            message Message {
                oneof foo {}
            }
            "#
        ),
        vec![EmptyOneof {
            span: Some(SourceSpan::from(79..91)),
        }],
    );
}

#[test]
fn enum_value_extrema() {
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Extreme {
                ZERO = 0;
                MIN = -2147483649;
                MAX = 2147483648;
            }
            "#
        ),
        vec![
            InvalidEnumNumber {
                span: Some(SourceSpan::from(108..119)),
            },
            InvalidEnumNumber {
                span: Some(SourceSpan::from(143..153)),
            }
        ],
    );
    assert_yaml_snapshot!(check_ok(
        r#"
        syntax = "proto3";

        enum Extreme {
            ZERO = 0;
            MIN = -2147483648;
            MAX = 2147483647;
        }
        "#
    ));
}

#[test]
fn enum_reserved_range_extrema() {
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Extreme {
                ZERO = 0;

                reserved -2147483649 to -1, 1 to 2147483648;
            }
            "#
        ),
        vec![
            InvalidEnumNumber {
                span: Some(SourceSpan::from(112..123)),
            },
            InvalidEnumNumber {
                span: Some(SourceSpan::from(136..146)),
            }
        ],
    );
    assert_yaml_snapshot!(check_ok(
        r#"
        syntax = "proto3";

        enum Extreme {
            ZERO = 0;
            reserved -2147483648 to -1, 1 to 2147483647;
        }
        "#
    ));
}

#[test]
fn enum_reserved_range_invalid() {
    assert_eq!(
        check_err(
            r#"enum Enum {
                reserved 1 to -1;
            }"#
        ),
        vec![InvalidRange {
            span: Some(SourceSpan::from(37..44)),
        },],
    );
    assert_yaml_snapshot!(check_ok(
        r#"enum Enum {
            reserved 1 to 1;
        }"#
    ));
}

#[test]
fn enum_reserved_range_overlap() {
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Enum {
                ZERO = 0;

                reserved 3, 3;
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 3, end: 3 },
            first_span: Some(SourceSpan::from(109..110)),
            second: resolve::NumberKind::ReservedRange { start: 3, end: 3 },
            second_span: Some(SourceSpan::from(112..113)),
        })],
    );
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Enum {
                ZERO = 0;

                reserved 1 to 5, 4;
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 1, end: 5 },
            first_span: Some(SourceSpan::from(109..115)),
            second: resolve::NumberKind::ReservedRange { start: 4, end: 4 },
            second_span: Some(SourceSpan::from(117..118)),
        })],
    );
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Enum {
                ZERO = 0;

                reserved 3, 2 to max;
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 3, end: 3 },
            first_span: Some(SourceSpan::from(109..110)),
            second: resolve::NumberKind::ReservedRange {
                start: 2,
                end: 2147483647
            },
            second_span: Some(SourceSpan::from(112..120)),
        })],
    );
}

#[test]
fn enum_reserved_range_overlap_with_value() {
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Enum {
                ZERO = 0;

                reserved -5 to 5;
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: -5, end: 5 },
            first_span: Some(SourceSpan::from(109..116)),
            second: resolve::NumberKind::EnumValue {
                name: "ZERO".to_owned(),
                number: 0
            },
            second_span: Some(SourceSpan::from(80..81)),
        })],
    );
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Enum {
                ZERO = 0;
                FIVE = 5;

                reserved 2 to max;
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange {
                start: 2,
                end: 2147483647
            },
            first_span: Some(SourceSpan::from(135..143)),
            second: resolve::NumberKind::EnumValue {
                name: "FIVE".to_owned(),
                number: 5,
            },
            second_span: Some(SourceSpan::from(106..107)),
        })],
    );
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Enum {
                ZERO = 0;
                FIVE = 5;

                reserved 5;
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::ReservedRange { start: 5, end: 5 },
            first_span: Some(SourceSpan::from(135..136)),
            second: resolve::NumberKind::EnumValue {
                name: "FIVE".to_owned(),
                number: 5,
            },
            second_span: Some(SourceSpan::from(106..107)),
        })],
    );
}

#[test]
fn enum_duplicate_number() {
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            enum Enum {
                ZERO = 0;
                ZERO2 = 0;
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::EnumValue {
                name: "ZERO".to_owned(),
                number: 0
            },
            first_span: Some(SourceSpan::from(80..81)),
            second: resolve::NumberKind::EnumValue {
                name: "ZERO2".to_owned(),
                number: 0
            },
            second_span: Some(SourceSpan::from(107..108)),
        })],
    );
    assert_eq!(
        check_err(
            r#"
            syntax = "proto3";

            message Message {
                enum Enum {
                    option allow_alias = false;

                    ZERO = 0;
                    ZERO2 = 0;
                }
            }
            "#
        ),
        vec![DuplicateNumber(DuplicateNumberError {
            first: resolve::NumberKind::EnumValue {
                name: "ZERO".to_owned(),
                number: 0
            },
            first_span: Some(SourceSpan::from(167..168)),
            second: resolve::NumberKind::EnumValue {
                name: "ZERO2".to_owned(),
                number: 0
            },
            second_span: Some(SourceSpan::from(198..199)),
        })],
    );
    assert_yaml_snapshot!(check_ok(
        r#"
        enum Enum {
            option allow_alias = true;

            ZERO = 0;
            ZERO2 = 0;
        }
        "#
    ));
}

#[test]
#[ignore]
fn proto2_enum_in_proto3_message() {
    todo!()
}

#[test]
#[ignore]
fn proto3_enum_default() {
    todo!()
}

#[test]
#[ignore]
fn option_unknown_field() {
    todo!()
}

#[test]
#[ignore]
fn option_unknown_extension() {
    todo!()
}

#[test]
fn option_already_set() {
    assert_eq!(
        check_err(
            r#"
            syntax = 'proto3';

            message Message {
                optional int32 foo = 1 [deprecated = true, deprecated = false];
            }"#
        ),
        vec![OptionAlreadySet {
            name: "deprecated".to_owned(),
            first: Some(SourceSpan::from(103..120)),
            second: Some(SourceSpan::from(122..140))
        }],
    );
}

#[test]
#[ignore]
fn option_ignore() {
    todo!()
}

#[test]
fn option_map_entry_set_explicitly() {
    assert_yaml_snapshot!(check_ok("message Foo { option map_entry = true; }"));
}

#[test]
#[ignore]
fn public_import() {
    todo!()
}

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

/*

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    foo: 1
};
option (a).foo = 2;
option (a).bar = 2;

extend google.protobuf.FileOptions {
    optional Foo a = 1001;
}

message Foo {
    repeated int32 foo = 1;
    optional int32 bar = 2;
}

*/

/*


foo.proto:8:8: Option field "(a)" is a repeated message. Repeated message options must be initialized using an aggregate value.

syntax = "proto2";

import "google/protobuf/descriptor.proto";

option (a) = {
    foo: 1
};
option (a).foo = 2;

extend google.protobuf.FileOptions {
    repeated Foo a = 1001;
}

message Foo {
    repeated int32 foo = 1;
    optional int32 bar = 2;
}


*/

/*

syntax = "proto3";

package google.protobuf;

message FileOptions {
    optional string java_outer_classname = 1;
}

option java_outer_classname = "ClassName";

*/
