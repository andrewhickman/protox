use insta::assert_debug_snapshot;
use prost_types::FileDescriptorProto;

use crate::ParseErrorKind::{self, *};

fn parse(source: &str) -> Result<FileDescriptorProto, Vec<ParseErrorKind>> {
    crate::parse(source).map_err(|mut err| {
        err.related.insert(0, err.kind);
        err.related
    })
}

#[test]
fn parse_field_default() {
    assert_debug_snapshot!(parse(
        "
        message Foo {
            optional int32 enum = 1 [default = 2.4];
        }
    "
    ));
    assert_debug_snapshot!(parse(
        "
        message Foo {
            optional Enum enum = 1 [default = ONE];
        }

        enum Enum {}
    "
    ));
}

#[test]
fn negative_ident_outside_default() {
    assert_debug_snapshot!(parse(
        "
        option opt = -foo;
    "
    ));
}

#[test]
fn map_field_invalid_type() {
    assert_eq!(
        parse(
            r#"message Message {
            map<Message, sfixed32> field = 1;
        }"#
        ),
        Err(vec![InvalidMapFieldKeyType { span: 34..41 }]),
    );
    assert_eq!(
        parse(
            r#"message Message {
            map<.Message, fixed32> field = 1;
        }"#
        ),
        Err(vec![InvalidMapFieldKeyType { span: 34..42 }]),
    );
    assert_eq!(
        parse(
            r#"message Message {
            map<.Message, bool> field = 1;
        }"#
        ),
        Err(vec![InvalidMapFieldKeyType { span: 34..42 }]),
    );
    assert_eq!(
        parse(
            r#"message Message {
            map<float, string> field = 1;
        }"#
        ),
        Err(vec![InvalidMapFieldKeyType { span: 34..39 }]),
    );
    assert_eq!(
        parse(
            r#"message Message {
            map<double, int64> field = 1;
        }"#
        ),
        Err(vec![InvalidMapFieldKeyType { span: 34..40 }]),
    );
    assert_eq!(
        parse(
            r#"message Message {
            map<Enum, int64> field = 1;

            enum Enum {
                ZERO = 0;
            }
        }"#
        ),
        Err(vec![InvalidMapFieldKeyType { span: 34..38 }]),
    );
    assert_debug_snapshot!(parse(
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
    ));
}

#[test]
fn invalid_message_number() {
    assert_eq!(
        parse("message Foo { optional int32 i = -5; }"),
        Err(vec![InvalidMessageNumber { span: 33..35 }])
    );
    assert_eq!(
        parse("message Foo { optional int32 i = 0; }"),
        Err(vec![InvalidMessageNumber { span: 33..34 }])
    );
    assert_eq!(
        parse("message Foo { optional int32 i = 536870912; }"),
        Err(vec![InvalidMessageNumber { span: 33..42 }])
    );
    assert_debug_snapshot!(parse("message Foo { optional int32 i = 1; }"));
    assert_debug_snapshot!(parse("message Foo { optional int32 i = 536870911; }"));
    assert_debug_snapshot!(parse("message Foo { optional int32 i = 18999; }"));
    assert_debug_snapshot!(parse("message Foo { optional int32 i = 20000; }"));
}

#[test]
fn proto3_default_value() {
    assert_eq!(
        parse(
            r#"
            syntax = 'proto3';

            message Message {
                optional int32 foo = 1 [default = -0];
            }"#
        ),
        Err(vec![Proto3DefaultValue { span: 103..115 }]),
    );
}

#[test]
fn map_field_with_label() {
    assert_eq!(
        parse(
            r#"message Message {
            optional map<int32, string> field = 1;
        }"#
        ),
        Err(vec![MapFieldWithLabel { span: 30..38 }]),
    );
    assert_eq!(
        parse(
            r#"
            syntax = 'proto3';

            message Message {
                required map<int32, string> field = 1;
            }"#
        ),
        Err(vec![MapFieldWithLabel { span: 79..87 }]),
    );
}

#[test]
fn message_reserved_range_extrema() {
    assert_eq!(
        parse(
            r#"message Message {
                reserved 0 to 1;
            }"#
        ),
        Err(vec![InvalidMessageNumber { span: 43..44 }]),
    );
    assert_eq!(
        parse(
            r#"message Message {
                reserved 1 to 536870912;
            }"#
        ),
        Err(vec![InvalidMessageNumber { span: 48..57 }]),
    );
    assert_debug_snapshot!(parse(
        r#"message Message {
            reserved 1 to 536870911;
        }"#
    ));
}

#[test]
fn extend_required_field() {
    assert_eq!(
        parse(
            r#"
            message Message {
                extensions 1;
            }

            extend Message {
                required int32 foo = 1;
            }
            "#
        ),
        Err(vec![RequiredExtendField { span: 121..129 }]),
    );
}

#[test]
fn extend_map_field() {
    assert_eq!(
        parse(
            r#"
            message Message {
                extensions 1;
            }

            extend Message {
                map<int32, string> foo = 1;
            }
            "#
        ),
        Err(vec![InvalidExtendFieldKind {
            kind: "map",
            span: 121..148,
        }]),
    );
}

#[test]
fn proto3_group_field() {
    assert_eq!(
        parse(
            r#"
            syntax = 'proto3';

            message Message {
                optional group Foo = 1 {};
            }
            "#
        ),
        Err(vec![Proto3GroupField { span: 79..104 }]),
    );
}

#[test]
fn proto3_required_field() {
    assert_eq!(
        parse(
            r#"
            syntax = 'proto3';

            message Message {
                required int32 foo = 1;
            }
            "#
        ),
        Err(vec![Proto3RequiredField { span: 79..87 }]),
    );
}

#[test]
fn proto2_field_missing_label() {
    assert_eq!(
        parse(
            r#"
            syntax = 'proto2';

            message Message {
                int32 foo = 1;
            }
            "#
        ),
        Err(vec![Proto2FieldMissingLabel { span: 79..93 }]),
    );
}

#[test]
fn oneof_field_with_label() {
    assert_eq!(
        parse(
            r#"
            syntax = 'proto3';

            message Message {
                oneof foo {
                    optional int32 bar = 1;
                }
            }
            "#
        ),
        Err(vec![OneofFieldWithLabel { span: 111..119 }]),
    );
}

#[test]
fn oneof_map_field() {
    assert_eq!(
        parse(
            r#"
            syntax = 'proto3';

            message Message {
                oneof foo {
                    map<int32, bytes> bar = 1;
                }
            }
            "#
        ),
        Err(vec![InvalidOneofFieldKind {
            kind: "map",
            span: 111..137,
        }]),
    );
}

#[test]
fn empty_oneof() {
    assert_eq!(
        parse(
            r#"
            syntax = 'proto3';

            message Message {
                oneof foo {}
            }
            "#
        ),
        Err(vec![EmptyOneof { span: 79..91 }]),
    );
}

#[test]
fn enum_value_extrema() {
    assert_eq!(
        parse(
            r#"
            syntax = "proto3";

            enum Extreme {
                ZERO = 0;
                MIN = -2147483649;
                MAX = 2147483648;
            }
            "#
        ),
        Err(vec![
            InvalidEnumNumber { span: 108..119 },
            InvalidEnumNumber { span: 143..153 }
        ]),
    );
    assert_debug_snapshot!(parse(
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
        parse(
            r#"
            syntax = "proto3";

            enum Extreme {
                ZERO = 0;

                reserved -2147483649 to -1, 1 to 2147483648;
            }
            "#
        ),
        Err(vec![
            InvalidEnumNumber { span: 112..123 },
            InvalidEnumNumber { span: 136..146 }
        ]),
    );
    assert_debug_snapshot!(parse(
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
fn message_field_json_name() {
    assert_eq!(
        parse(
            r#"message Message {
            optional int32 field = 1 [json_name = "\xFF"];
        }"#
        ),
        Err(vec![InvalidUtf8String { span: 68..74 }]),
    );
    assert_debug_snapshot!(parse(
        r#"message Message {
        optional int32 field = 1 [json_name = '$FIELD'];
    }"#
    ));
}

#[test]
fn field_default_value() {
    assert_debug_snapshot!(parse(
        r#"
        message Message {
            optional Message foo = 1 [default = ""];
        }"#
    ));
    assert_eq!(
        parse(
            r#"
            message Message {
                map<uint32, sfixed64> foo = 1 [default = ""];
            }"#
        ),
        Err(vec![InvalidDefault {
            kind: "map",
            span: 78..90,
        }]),
    );
    assert_eq!(
        parse(
            r#"
            message Message {
                optional group Foo = 1 [default = ""] {};
            }"#
        ),
        Err(vec![InvalidDefault {
            kind: "group",
            span: 71..83,
        }]),
    );
    assert_eq!(
        parse(
            r#"
            message Message {
                repeated int32 foo = 1 [default = 1];
            }"#
        ),
        Err(vec![InvalidDefault {
            kind: "repeated",
            span: 71..82,
        }]),
    );
    assert_debug_snapshot!(parse(
        r#"
            message Message {
                optional float default_float_exp = 23 [ default = 9e6];
                optional double default_double_exp = 24 [ default = 9e22];
            }
        "#
    ));
}

#[test]
fn field_default_invalid_type() {
    assert_eq!(
        parse(
            r#"
            message Message {
                optional int32 foo = 1 [default = "foo"];
            }"#
        ),
        Err(vec![ValueInvalidType {
            expected: "an integer".to_owned(),
            actual: "foo".to_owned(),
            span: 81..86,
        }]),
    );
    assert_eq!(
        parse(
            r#"
            message Message {
                optional uint32 foo = 1 [default = -100];
            }"#
        ),
        Err(vec![IntegerValueOutOfRange {
            expected: "an unsigned 32-bit integer".to_owned(),
            actual: "-100".to_owned(),
            min: "0".to_owned(),
            max: "4294967295".to_owned(),
            span: 82..86,
        }]),
    );
    assert_eq!(
        parse(
            r#"
            message Message {
                optional int32 foo = 1 [default = 2147483648];
            }"#
        ),
        Err(vec![IntegerValueOutOfRange {
            expected: "a signed 32-bit integer".to_owned(),
            actual: "2147483648".to_owned(),
            min: "-2147483648".to_owned(),
            max: "2147483647".to_owned(),
            span: 81..91,
        }]),
    );
    assert_debug_snapshot!(parse(
        r#"
        message Message {
            optional Foo foo = 1 [default = 1];
        }

        enum Foo {
            ZERO = 0;
        }"#
    ));
    assert_debug_snapshot!(parse(
        r#"
        message Message {
            optional Foo foo = 1 [default = "ZERO"];
        }

        enum Foo {
            ZERO = 0;
        }"#
    ));
    assert_eq!(
        parse(
            r#"
            message Message {
                optional bool foo = 1 [default = FALSE];
            }"#
        ),
        Err(vec![ValueInvalidType {
            expected: "either 'true' or 'false'".to_owned(),
            actual: "FALSE".to_owned(),
            span: 80..85,
        }]),
    );
    assert_debug_snapshot!(parse(
        r#"
        message Message {
            optional Foo foo = 1 [default = FALSE];
        }

        enum Foo {
            ZERO = 0;
        }"#
    ));
    assert_eq!(
        parse(
            r#"
            message Message {
                optional bool foo = 1 [default = -false];
            }

            enum Foo {
                ZERO = 0;
            }"#
        ),
        Err(vec![ValueInvalidType {
            expected: "either 'true' or 'false'".to_owned(),
            actual: "-false".to_owned(),
            span: 80..86,
        }]),
    );
    assert_eq!(
        parse(
            r#"
            message Message {
                optional string foo = 1 [default = '\xFF'];
            }"#
        ),
        Err(vec![InvalidUtf8String { span: 82..88 }]),
    );
}
