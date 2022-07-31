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
