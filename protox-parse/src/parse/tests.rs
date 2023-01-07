use insta::assert_debug_snapshot;

use super::*;

macro_rules! case {
    ($method:ident($source:expr)) => {{
        let mut parser = Parser::new($source);
        let result = parser.$method();
        assert_debug_snapshot!(if parser.lexer.extras.errors.is_empty() {
            Ok(result.unwrap())
        } else {
            Err(parser.lexer.extras.errors)
        });
    }};
}

#[test]
pub fn parse_option() {
    case!(parse_option("option foo = 5;"));
    case!(parse_option(
        "//detached\n\n /*leading*/\noption foo = 5;//trailing"
    ));
    case!(parse_option("option (foo.bar) = \"hello\";"));
    case!(parse_option("option (foo).bar = true;"));
    case!(parse_option("option foo.(bar.baz).qux = ident;"));
    case!(parse_option("option ;"));
    case!(parse_option("option foo ("));
    case!(parse_option("option foo.]"));
    case!(parse_option("option foo = ="));
    case!(parse_option("option foo = 1 )"));
    case!(parse_option("option foo = {};"));
    case!(parse_option("option (ext).foo = { foo: 5 };"));
    case!(parse_option("option quz.(bar) = { foo: [blah] };"));
    case!(parse_option(
        "option baz = { foo: [<x:3>, <y{};z<a:-foo>,>] };"
    ));
    case!(parse_option("option foo = 1f;"));
    case!(parse_option("option optimize_for = message;"));
    case!(parse_option("option message = 1;"));
    case!(parse_option("option ext.(service.rpc) = 1;"));
    case!(parse_option("option foo = {"));
    case!(parse_option("option foo = { x:1"));
    case!(parse_option("option optimize_for = google.protobuf.SPEED;"));
    case!(parse_option("option ext.(.foo.bar) = 42;"));
    case!(parse_option("option foo = -'a';"));
    case!(parse_option("option foo = { } }"));
    case!(parse_option("option foo = {"));
}

#[test]
fn parse_text_format_message() {
    case!(parse_text_format_message("foo: 10"));
    case!(parse_text_format_message("foo: 10f"));
    case!(parse_text_format_message("foo: 1.0f"));
    case!(parse_text_format_message("foo: 'bar' \"baz\" \n\t 'quz'"));
    case!(parse_text_format_message(
        r#"s: "first" 'second'
            "third"
        joined: "first""second"'third''fourth'"#
    ));
    case!(parse_text_format_message("message: { foo: \"bar\" }"));
    case!(parse_text_format_message("[ext.scalar]: 10"));
    case!(parse_text_format_message("[ext.message]: { foo: \"bar\" }"));
    case!(parse_text_format_message(
        "any: { [type.googleapis.com/foo.bar]: { foo: \"bar\" } }"
    ));
    case!(parse_text_format_message("foo: enum"));
    case!(parse_text_format_message("foo: [enum]"));
    case!(parse_text_format_message("foo: -enum"));
    case!(parse_text_format_message("foo: [-enum]"));
    case!(parse_text_format_message(
        "pot < kind: TULIP name: \"Bob\" legs: 0 >"
    ));
    case!(parse_text_format_message(
        r#"escapes: '\a\b\f\n\r\t\v\?\\\'\"\1\11\111\xa\xAA\u1111\U00101111'"#
    ));
    case!(parse_text_format_message(r#"value: -'string'"#));
    case!(parse_text_format_message(r#"value: -"#));
    case!(parse_text_format_message(r#"value"#));
    case!(parse_text_format_message(r#"value: {"#));
    case!(parse_text_format_message(r#"value: {} foo: 10f"#));
    case!(parse_text_format_message(r#"value: <"#));
    case!(parse_text_format_message(r#"value 'foo'"#));
    case!(parse_text_format_message(
        "value: 1 /* block */ value: 2 // line\n value: 3"
    ));
}

#[test]
fn parse_enum() {
    case!(parse_enum("enum Foo {}"));
    case!(parse_enum("enum Foo { ; ; }"));
    case!(parse_enum(
        "/*detached*//*leading*/\nenum Foo {\n//trailing\n\n; ; }"
    ));
    case!(parse_enum("enum Foo { BAR = 1; }"));
    case!(parse_enum("enum Foo { option bar = 'quz' ; VAL = -1; }"));
    case!(parse_enum("enum Foo { BAR = 0 [opt = 0.5]; }"));
    case!(parse_enum("enum Foo { BAR = 0; reserved -1 to max; }"));
    case!(parse_enum("enum 3"));
    case!(parse_enum("enum Foo 0.1"));
    case!(parse_enum("enum Foo {]"));
    case!(parse_enum("enum Foo { BAR ."));
    case!(parse_enum("enum Foo { BAR = foo"));
    case!(parse_enum("enum Foo { message = 1; }"));
    case!(parse_enum("enum \"Foo\""));
    case!(parse_enum("enum Foo { BAR = 0 [(an.ext).opt = 0.5]; }"));
    case!(parse_enum("enum Foo { BAR = 0 }"));
}

#[test]
fn parse_service() {
    case!(parse_service("service Foo {}"));
    case!(parse_service("service Foo { ; ; }"));
    case!(parse_service(
        "//detached\n\n//leading\nservice Foo {\n/* nottrailing */; ; }"
    ));
    case!(parse_service("service service { }"));
    case!(parse_service("service Foo { rpc bar(A) returns (B.C); }"));
    case!(parse_service(
        "service Foo { rpc bar(stream .A.B) returns (stream .C); }"
    ));
    case!(parse_service(
        "service Foo { rpc bar(A) returns (B.C) { } }"
    ));
    case!(parse_service(
        "service Foo { rpc bar(A) returns (B.C) { ; ; } }"
    ));
    case!(parse_service(
        "service Foo { rpc bar(A) returns (B.C) { option opt = -1; } }"
    ));
    case!(parse_service("service ;"));
    case!(parse_service("service Foo ("));
    case!(parse_service("service Foo { bar"));
    case!(parse_service("service Foo { rpc ="));
    case!(parse_service("service Foo { rpc bar{"));
    case!(parse_service("service Foo { rpc bar(+"));
    case!(parse_service("service Foo { rpc bar(A("));
    case!(parse_service("service Foo { rpc bar(A) ["));
    case!(parse_service("service Foo { rpc bar(A) returns ="));
    case!(parse_service("service Foo { rpc bar(A) returns ()"));
    case!(parse_service("service Foo { rpc bar(A) returns (stream =)"));
    case!(parse_service("service Foo { rpc bar(A) returns (stream B}"));
    case!(parse_service(
        "service Foo { rpc bar(A) returns (stream B) )"
    ));
    case!(parse_service(
        "service Foo { rpc bar(A) returns (stream B) {rpc"
    ));
    case!(parse_service(
        "service Foo { rpc bar(stream stream) returns (stream stream.stream); }"
    ));
    case!(parse_service(
        "service Foo { rpc bar(stream .stream.rpc) returns (stream .map.enum); }"
    ));
    case!(parse_service(
        "service Foo {
            option foo = bar;
        }"
    ));
}

#[test]
pub fn parse_package() {
    case!(parse_package("package foo;"));
    case!(parse_package(
        "//detached\n//detached2\n\n//detached3\n\npackage foo;\n/*trailing*/"
    ));
    case!(parse_package("package foo.bar;"));
    case!(parse_package("package ="));
    case!(parse_package("package foo)"));
}

#[test]
pub fn parse_import() {
    case!(parse_import("import 'foo';"));
    case!(parse_import("/*leading*/\nimport 'foo';/*trailing*/\n"));
    case!(parse_import("import weak \"foo\";"));
    case!(parse_import("import public 'f\\x6fo';"));
    case!(parse_import("import ;"));
    case!(parse_import("import public ;"));
    case!(parse_import("import 'foo' message"));
    case!(parse_import("import 'foo\\\\bar';"));
    case!(parse_import("import 'foo//bar';"));
    case!(parse_import("import 'foo/./bar';"));
    case!(parse_import("import 'foo/../bar';"));
    case!(parse_import("import '\\xFF';"));
    case!(parse_import("import 'foo' \"bar\";"));
}

#[test]
pub fn parse_extension() {
    case!(parse_extend("extend Foo { }"));
    case!(parse_extend("/*leading*/extend Foo {\n//trailing\n }"));
    case!(parse_extend("extend Foo { ; ; }"));
    case!(parse_extend("extend Foo.Foo { optional int32 bar = 126; }"));
    case!(parse_extend(
        "extend .Foo { optional int32 bar = 126; repeated string quz = 127; }"
    ));
    case!(parse_extend(
        "extend Foo { repeated group A = 1 { optional string name = 2; } }"
    ));
    case!(parse_extend("extend ] "));
    case!(parse_extend("extend { 'foo' }"));
}

#[test]
pub fn parse_reserved() {
    case!(parse_reserved("//detached\n\nreserved 'foo';//trailing"));
    case!(parse_reserved("reserved 0, 2 to 3, 5 to max;"));
    case!(parse_reserved("reserved -1;"));
    case!(parse_reserved("reserved 'foo', 'bar';"));
    case!(parse_reserved("reserved ;"));
    case!(parse_reserved("reserved '0foo';"));
    case!(parse_reserved("reserved '\\xFF';"));
    case!(parse_reserved("reserved -1f;"));
}

#[test]
pub fn parse_group() {
    case!(parse_field(
        "//leading\noptional group A = 1 {\n/*trailing*/ }"
    ));
    case!(parse_field("optional group A = 1 { }"));
    case!(parse_field("optional group A = 1 { ; ; }"));
    case!(parse_field("optional group A = 1 [deprecated = true] { }"));
    case!(parse_field(
        "optional group A = 1 { optional sint32 foo = 2; }"
    ));
    case!(parse_field("optional group a = 1 { }"));
    case!(parse_field("optional group , { }"));
    case!(parse_field("optional group a ["));
    case!(parse_field("optional group A = {"));
    case!(parse_field("optional group A = 1 ;"));
    case!(parse_field("optional group A = 1 {]"));
    case!(parse_field("optional group A = 1f { };"));
    case!(parse_field("optional 'group' A = 1 { };"));
}

#[test]
pub fn parse_field() {
    case!(parse_field("map<string, Project> projects = 3;"));
    case!(parse_field(
        "/*leading*/map<string, int32> projects = 3;\n/*trailing*/\n"
    ));
    case!(parse_field(
        "map<int32, bool> name = 5 [opt = true, opt2 = 4.5];"
    ));
    case!(parse_field("map<.foo.bar, bool> invalid = -0;"));
    case!(parse_field("map>"));
    case!(parse_field("map<;"));
    case!(parse_field("map<int32("));
    case!(parse_field("map<string, ="));
    case!(parse_field("map<string, .Foo,"));
    case!(parse_field("map<string, Foo> ;"));
    case!(parse_field("map<string, Foo> foo ]"));
    case!(parse_field("map<string, Foo> foo = x"));
    case!(parse_field("map<string, Foo> foo = 1 service"));
    case!(parse_field("map<foo;"));
    case!(parse_field("double double = 1 [default = -nan];"));
    case!(parse_field("optional int32 name = 5 [(ext) = \"foo\"];"));
    case!(parse_field("{"));
}

#[test]
pub fn parse_message() {
    case!(parse_message("message Foo {}"));
    case!(parse_message(
        "//detached\n/*leading*/message Foo {/*trailing*/}"
    ));
    case!(parse_message("message Foo { ; ; }"));
    case!(parse_message(
        "\
        message Foo {\
            message Bar {}\
            enum Quz {}\
            extend Bar {}\
        }"
    ));
    case!(parse_message(
        "\
        message Foo {
            fixed32 a = 1;
            optional map<int32, bool> b = 2;

            optional group C = 3 {
                required float d = 1;
            }

            oneof x {
                string y = 4;
            }
        }"
    ));
    case!(parse_message("message Foo { repeated Bar a = 1; }"));
    case!(parse_message("message Foo { repeated Bar service = 2; }"));
    case!(parse_message(
        "message Foo { extensions 5, 7 to 8, 10 to max [deprecated = false]; }"
    ));
    case!(parse_message(
        "message Foo { repeated map<sint32, fixed64> m = 1; }"
    ));
    case!(parse_message("message Foo { group Baz = 1 {} }"));
    case!(parse_message("message Foo { , }"));
    case!(parse_message(
        "message Foo {
        message Foo {
            optional int32 start = 1;  // trail1
            optional int32 end = 2;    // trail2
        }
    }"
    ));
    case!(parse_message("message Foo { service foo = 1; }"));
    case!(parse_message("message Foo { optional rpc foo = 1; }"));
    case!(parse_message(
        "message Foo { extensions 5, 7 to 8, 10 to max [(ext.ext).ext.(ext) = { a: IDENT }]; }"
    ));
    case!(parse_message("message Foo { extensions 5 }"));
    case!(parse_message("message Foo { extensions 5 to 5 }"));
    case!(parse_message("message Foo { reserved 'a' }"));
    case!(parse_message("message Foo { extensions 5 to }"));
    case!(parse_message("message Foo { optional .a.b, }"));
}

#[test]
pub fn parse_oneof() {
    case!(parse_oneof("oneof Foo {}"));
    case!(parse_oneof("oneof Foo { ; ; }"));
    case!(parse_oneof(
        "/*detached1*///detached2\n\n//leading\noneof Foo {/*trailing*/ ; ; }"
    ));
    case!(parse_oneof("oneof Foo { int32 bar = 1; }"));
    case!(parse_oneof("oneof Foo { optional group Bar = 1 {} }"));
    case!(parse_oneof("oneof Foo { group Baz = -1 {} }"));
    case!(parse_oneof("oneof 10.4"));
    case!(parse_oneof("oneof Foo <"));
    case!(parse_oneof("oneof Foo { ,"));
    case!(parse_oneof("oneof Foo { bytes b = 1 }"));
    case!(parse_oneof("oneof Foo { oneof Bar {} }"));
    case!(parse_oneof("oneof Foo { oneof bar = 1; }"));
    case!(parse_oneof("oneof Foo { required oneof bar = 1; }"));
}

#[test]
pub fn parse_file() {
    case!(parse_file(""));
    case!(parse_file("package protox.lib;"));
    case!(parse_file(
        "\
        package protox.lib;
        package another.one;
    "
    ));
    case!(parse_file(
        "\
        syntax = 'proto2';

        option optimize_for = SPEED;
    "
    ));
    case!(parse_file(
        "\
        syntax = \"proto3\";

        import \"foo.proto\";
    "
    ));
    case!(parse_file(
        "\
        syntax = 'unknown';
    "
    ));
    case!(parse_file(
        "\
        syntax = 'proto2';

        message Foo { , }
        enum Bar { ; }
        option quz 1;
    "
    ));
    case!(parse_file(
        "\
        syntax = 'proto3';

        message Foo {
            // trailing

            // detached

            // leading
            int32 bar = 1;
            // trailing2
        }
    "
    ));
    case!(parse_file("syntax = 'proto3'"));
    case!(parse_file(
        "/* leading detached */\n// leading\n syntax = 'proto3'; /* trailing */"
    ));
    case!(parse_file("option invalid = /"));
    case!(parse_file(
        "
        // code goes brrr
        option optimize_for = SPEED;
    "
    ));
    case!(parse_file("syntax = \"pro\" \n\n 'to3';"));
    case!(parse_file(";"));
    case!(parse_file("syntax = 1;"));
    case!(parse_file("thing"));
    case!(parse_file("message } } } message } } }"));
}
