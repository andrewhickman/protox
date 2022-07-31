use insta::assert_debug_snapshot;

use crate::parse;

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
