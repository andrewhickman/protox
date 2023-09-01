use super::*;

// use proptest::prelude::*;

#[test]
fn simple_tokens() {
    let source = r#"hell0 052 42 0x2A 5. 0.5 0.42e+2 2e-4 .2e+3 52e3 true
        false "hello \a\b\f\n\r\t\v\?\\\'\" \052 \x2a" 'hello 😀' _foo"#;
    let mut lexer = Token::lexer(source);

    assert_eq!(lexer.next().unwrap(), Ok(Token::Ident("hell0")));
    assert_eq!(lexer.next().unwrap(), Ok(Token::IntLiteral(42)));
    assert_eq!(lexer.next().unwrap(), Ok(Token::IntLiteral(42)));
    assert_eq!(lexer.next().unwrap(), Ok(Token::IntLiteral(42)));
    assert_eq!(lexer.next().unwrap(), Ok(Token::FloatLiteral(EqFloat(5.))));
    assert_eq!(lexer.next().unwrap(), Ok(Token::FloatLiteral(EqFloat(0.5))));
    assert_eq!(
        lexer.next().unwrap(),
        Ok(Token::FloatLiteral(EqFloat(0.42e+2)))
    );
    assert_eq!(
        lexer.next().unwrap(),
        Ok(Token::FloatLiteral(EqFloat(2e-4)))
    );
    assert_eq!(
        lexer.next().unwrap(),
        Ok(Token::FloatLiteral(EqFloat(0.2e+3)))
    );
    assert_eq!(
        lexer.next().unwrap(),
        Ok(Token::FloatLiteral(EqFloat(52e3)))
    );
    assert_eq!(lexer.next().unwrap(), Ok(Token::Ident("true")));
    assert_eq!(lexer.next().unwrap(), Ok(Token::Newline));
    assert_eq!(lexer.next().unwrap(), Ok(Token::Ident("false")));
    assert_eq!(
        lexer.next().unwrap(),
        Ok(Token::StringLiteral(
            b"hello \x07\x08\x0c\n\r\t\x0b?\\'\" * *".as_ref().into()
        ))
    );
    assert_eq!(
        lexer.next().unwrap(),
        Ok(Token::StringLiteral(
            b"hello \xF0\x9F\x98\x80".as_ref().into()
        ))
    );
    assert_eq!(lexer.next().unwrap(), Ok(Token::Ident("_foo")));
    assert_eq!(lexer.next(), None);

    assert_eq!(lexer.extras.errors, vec![]);
}

#[test]
fn integer_overflow() {
    let source = "99999999999999999999999999999999999999 4";
    let mut lexer = Token::lexer(source);

    assert_eq!(lexer.next(), Some(Ok(Token::IntLiteral(0))));
    assert_eq!(lexer.next(), Some(Ok(Token::IntLiteral(4))));
    assert_eq!(lexer.next(), None);

    assert_eq!(
        lexer.extras.errors,
        vec![ParseErrorKind::IntegerOutOfRange {
            span: 0..(source.len() - 2),
        }]
    );
}

#[test]
fn float_suffix() {
    let source = "10f 5.f 0.5f 0.42e+2f 2e-4f .2e+3f";
    let mut lexer = Token::lexer(source);

    assert_eq!(lexer.next().unwrap(), Ok(Token::FloatLiteral(EqFloat(10.))));
    assert_eq!(lexer.next().unwrap(), Ok(Token::FloatLiteral(EqFloat(5.))));
    assert_eq!(lexer.next().unwrap(), Ok(Token::FloatLiteral(EqFloat(0.5))));
    assert_eq!(
        lexer.next().unwrap(),
        Ok(Token::FloatLiteral(EqFloat(0.42e+2)))
    );
    assert_eq!(
        lexer.next().unwrap(),
        Ok(Token::FloatLiteral(EqFloat(2e-4)))
    );
    assert_eq!(
        lexer.next().unwrap(),
        Ok(Token::FloatLiteral(EqFloat(0.2e+3)))
    );
    assert_eq!(lexer.next(), None);

    assert_eq!(
        lexer.extras.errors,
        vec![
            ParseErrorKind::FloatSuffixOutsideTextFormat { span: 0..3 },
            ParseErrorKind::FloatSuffixOutsideTextFormat { span: 4..7 },
            ParseErrorKind::FloatSuffixOutsideTextFormat { span: 8..12 },
            ParseErrorKind::FloatSuffixOutsideTextFormat { span: 13..21 },
            ParseErrorKind::FloatSuffixOutsideTextFormat { span: 22..27 },
            ParseErrorKind::FloatSuffixOutsideTextFormat { span: 28..34 },
        ],
    );

    let mut lexer = Token::lexer(source);
    lexer.extras.text_format_mode = true;

    assert_eq!(lexer.next().unwrap(), Ok(Token::FloatLiteral(EqFloat(10.))));
    assert_eq!(lexer.next().unwrap(), Ok(Token::FloatLiteral(EqFloat(5.))));
    assert_eq!(lexer.next().unwrap(), Ok(Token::FloatLiteral(EqFloat(0.5))));
    assert_eq!(
        lexer.next().unwrap(),
        Ok(Token::FloatLiteral(EqFloat(0.42e+2)))
    );
    assert_eq!(
        lexer.next().unwrap(),
        Ok(Token::FloatLiteral(EqFloat(2e-4)))
    );
    assert_eq!(
        lexer.next().unwrap(),
        Ok(Token::FloatLiteral(EqFloat(0.2e+3)))
    );
    assert_eq!(lexer.next(), None);
    assert_eq!(lexer.extras.errors, vec![]);
}

#[test]
fn invalid_token() {
    let source = "@ foo";
    let mut lexer = Token::lexer(source);

    assert_eq!(lexer.next(), Some(Err(())));
    assert_eq!(lexer.next(), Some(Ok(Token::Ident("foo"))));
    assert_eq!(lexer.next(), None);

    assert_eq!(lexer.extras.errors, vec![]);
}

#[test]
fn invalid_string_char() {
    let source = "\"\x00\" foo";
    let mut lexer = Token::lexer(source);

    assert_eq!(
        lexer.next(),
        Some(Ok(Token::StringLiteral(Default::default())))
    );
    assert_eq!(lexer.next(), Some(Ok(Token::Ident("foo"))));
    assert_eq!(lexer.next(), None);

    assert_eq!(
        lexer.extras.errors,
        vec![ParseErrorKind::InvalidStringCharacters { span: 1..2 }]
    );
}

#[test]
fn unterminated_string() {
    let source = "\"hello \n foo";
    let mut lexer = Token::lexer(source);

    assert_eq!(
        lexer.next(),
        Some(Ok(Token::StringLiteral(b"hello ".as_ref().into())))
    );
    assert_eq!(lexer.next(), Some(Ok(Token::Ident("foo"))));
    assert_eq!(lexer.next(), None);

    assert_eq!(
        lexer.extras.errors,
        vec![ParseErrorKind::UnterminatedString { span: 7..8 }]
    );
}

#[test]
fn invalid_string_escape() {
    let source = r#""\m" foo"#;
    let mut lexer = Token::lexer(source);

    assert_eq!(
        lexer.next(),
        Some(Ok(Token::StringLiteral(b"m".as_ref().into())))
    );
    assert_eq!(lexer.next(), Some(Ok(Token::Ident("foo"))));
    assert_eq!(lexer.next(), None);

    assert_eq!(
        lexer.extras.errors,
        vec![ParseErrorKind::InvalidStringEscape { span: 1..2 }]
    );
}

#[test]
fn string_escape_invalid_utf8() {
    let source = r#""\xFF""#;
    let mut lexer = Token::lexer(source);

    assert_eq!(
        lexer.next(),
        Some(Ok(Token::StringLiteral([0xff].as_ref().into())))
    );
    assert_eq!(lexer.next(), None);
}

#[test]
fn string_unterminated() {
    let source = r#""a"#;
    let mut lexer = Token::lexer(source);

    assert_eq!(
        lexer.next(),
        Some(Ok(Token::StringLiteral(b"a".as_ref().into())))
    );
    assert_eq!(lexer.next(), None);

    assert_eq!(
        lexer.extras.errors,
        vec![ParseErrorKind::UnexpectedEof {
            expected: "string terminator".to_owned()
        }]
    );
}

#[test]
fn merge_string_errors() {
    let source = "\"\\\x00\" foo";
    let mut lexer = Token::lexer(source);

    assert_eq!(
        lexer.next(),
        Some(Ok(Token::StringLiteral(b"".as_ref().into())))
    );
    assert_eq!(lexer.next(), Some(Ok(Token::Ident("foo"))));
    assert_eq!(lexer.next(), None);

    assert_eq!(
        lexer.extras.errors,
        vec![ParseErrorKind::InvalidStringEscape { span: 1..3 }]
    );
}

#[test]
fn merge_string_errors2() {
    let source = "9999999999999999999999 \"\\\x00\"";
    let mut lexer = Token::lexer(source);

    assert_eq!(lexer.next(), Some(Ok(Token::IntLiteral(0))));
    assert_eq!(
        lexer.next(),
        Some(Ok(Token::StringLiteral(b"".as_ref().into())))
    );
    assert_eq!(lexer.next(), None);

    assert_eq!(
        lexer.extras.errors,
        vec![
            ParseErrorKind::IntegerOutOfRange { span: 0..22 },
            ParseErrorKind::InvalidStringEscape { span: 24..26 },
        ]
    );
}

#[test]
fn merge_string_errors3() {
    let source = "\"\\\x00 \\\x00\"";
    let mut lexer = Token::lexer(source);

    assert_eq!(
        lexer.next(),
        Some(Ok(Token::StringLiteral(b" ".as_ref().into())))
    );
    assert_eq!(lexer.next(), None);

    assert_eq!(
        lexer.extras.errors,
        vec![
            ParseErrorKind::InvalidStringEscape { span: 1..3 },
            ParseErrorKind::InvalidStringEscape { span: 4..6 },
        ]
    );
}

#[test]
fn string_unicode_escape() {
    let source = r"'\u0068\u0065\u006c\u006c\u006f\u0020\U0001f600'";
    let mut lexer = Token::lexer(source);

    assert_eq!(
        lexer.next(),
        Some(Ok(Token::StringLiteral(
            b"hello \xF0\x9F\x98\x80".as_ref().into()
        )))
    );
    assert_eq!(lexer.next(), None);

    assert_eq!(lexer.extras.errors, vec![]);
}

#[test]
fn string_invalid_unicode_escape() {
    let mut lexer = Token::lexer(r"'\Uffffffff'");
    lexer.by_ref().for_each(drop);
    assert_eq!(
        lexer.extras.errors,
        vec![ParseErrorKind::InvalidStringEscape { span: 1..11 }]
    );
}

#[test]
fn line_comment() {
    let source = "foo // bar \n quz";
    let mut lexer = Token::lexer(source);

    assert_eq!(lexer.next(), Some(Ok(Token::Ident("foo"))));
    assert_eq!(lexer.next(), Some(Ok(Token::LineComment(" bar \n".into()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Ident("quz"))));
    assert_eq!(lexer.next(), None);

    assert_eq!(lexer.extras.errors, vec![]);
}

#[test]
fn line_comment_normalize_newlines() {
    let source = "// foo\r\n // bar\r\n";
    let mut lexer = Token::lexer(source);

    assert_eq!(lexer.next(), Some(Ok(Token::LineComment(" foo\n".into()))));
    assert_eq!(lexer.next(), Some(Ok(Token::LineComment(" bar\n".into()))));
    assert_eq!(lexer.next(), None);

    assert_eq!(lexer.extras.errors, vec![]);
}

#[test]
fn block_comment() {
    let source = "foo /* bar\n */ quz";
    let mut lexer = Token::lexer(source);

    assert_eq!(lexer.next(), Some(Ok(Token::Ident("foo"))));
    assert_eq!(lexer.next(), Some(Ok(Token::BlockComment(" bar\n".into()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Ident("quz"))));
    assert_eq!(lexer.next(), None);

    assert_eq!(lexer.extras.errors, vec![]);
}

#[test]
fn block_comment_multiline() {
    let source = "/* foo\n * bar\n quz*/";
    let mut lexer = Token::lexer(source);

    assert_eq!(
        lexer.next(),
        Some(Ok(Token::BlockComment(" foo\n bar\nquz".into())))
    );
    assert_eq!(lexer.next(), None);

    assert_eq!(lexer.extras.errors, vec![]);
}

#[test]
fn nested_block_comment() {
    let source = "foo /* /* bar\n */ */ quz";
    let mut lexer = Token::lexer(source);

    for _ in &mut lexer {}

    assert_eq!(
        lexer.extras.errors,
        vec![ParseErrorKind::NestedBlockComment { span: 7..9 }]
    );
}

#[test]
fn nested_block_comment_unterminated() {
    let source = "foo /* /* bar\n */ quz";
    let mut lexer = Token::lexer(source);

    for _ in &mut lexer {}

    assert_eq!(
        lexer.extras.errors,
        vec![ParseErrorKind::NestedBlockComment { span: 7..9 }]
    );
}

#[test]
fn block_comment_unterminated() {
    let source = "foo /* bar\n quz";
    let mut lexer = Token::lexer(source);

    for _ in &mut lexer {}

    assert_eq!(
        lexer.extras.errors,
        vec![ParseErrorKind::UnexpectedEof {
            expected: "comment terminator".to_owned()
        }]
    );
}

#[test]
fn block_comment_trailing_newline() {
    let source = "/* bar */\n";
    let mut lexer = Token::lexer(source);

    assert_eq!(lexer.next(), Some(Ok(Token::BlockComment(" bar ".into()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Newline)));
    assert_eq!(lexer.next(), None);

    assert_eq!(lexer.extras.errors, vec![]);
}

#[test]
fn block_comment_normalize_newlines() {
    let source = "/* foo\r\n bar */";
    let mut lexer = Token::lexer(source);

    assert_eq!(
        lexer.next(),
        Some(Ok(Token::BlockComment(" foo\nbar ".into())))
    );
    assert_eq!(lexer.next(), None);

    assert_eq!(lexer.extras.errors, vec![]);
}

#[test]
fn hash_comment() {
    let source = "# bar";
    let mut lexer = Token::lexer(source);

    assert_eq!(lexer.next(), Some(Ok(Token::LineComment(" bar".into()))));
    assert_eq!(lexer.next(), None);

    assert_eq!(
        lexer.extras.errors,
        vec![ParseErrorKind::HashCommentOutsideTextFormat { span: 0..5 }]
    );

    let mut lexer = Token::lexer(source);
    lexer.extras.text_format_mode = true;

    assert_eq!(lexer.next(), Some(Ok(Token::LineComment(" bar".into()))));
    assert_eq!(lexer.next(), None);

    assert_eq!(lexer.extras.errors, vec![]);
}

#[test]
fn whitespace() {
    assert_eq!(
        Token::lexer("value: -2.0").collect::<Vec<_>>(),
        vec![
            Ok(Token::Ident("value")),
            Ok(Token::Colon),
            Ok(Token::Minus),
            Ok(Token::FloatLiteral(EqFloat(2.0))),
        ]
    );
    assert_eq!(
        Token::lexer("value: - 2.0").collect::<Vec<_>>(),
        vec![
            Ok(Token::Ident("value")),
            Ok(Token::Colon),
            Ok(Token::Minus),
            Ok(Token::FloatLiteral(EqFloat(2.0))),
        ]
    );
    assert_eq!(
        Token::lexer("value: -\n  #comment\n  2.0").collect::<Vec<_>>(),
        vec![
            Ok(Token::Ident("value")),
            Ok(Token::Colon),
            Ok(Token::Minus),
            Ok(Token::Newline),
            Ok(Token::LineComment("comment\n".into())),
            Ok(Token::FloatLiteral(EqFloat(2.0))),
        ]
    );
    assert_eq!(
        Token::lexer("value: 2 . 0").collect::<Vec<_>>(),
        vec![
            Ok(Token::Ident("value")),
            Ok(Token::Colon),
            Ok(Token::IntLiteral(2)),
            Ok(Token::Dot),
            Ok(Token::IntLiteral(0)),
        ]
    );

    assert_eq!(
        Token::lexer("foo: 10 bar: 20").collect::<Vec<_>>(),
        vec![
            Ok(Token::Ident("foo")),
            Ok(Token::Colon),
            Ok(Token::IntLiteral(10)),
            Ok(Token::Ident("bar")),
            Ok(Token::Colon),
            Ok(Token::IntLiteral(20)),
        ]
    );
    assert_eq!(
        Token::lexer("foo: 10,bar: 20").collect::<Vec<_>>(),
        vec![
            Ok(Token::Ident("foo")),
            Ok(Token::Colon),
            Ok(Token::IntLiteral(10)),
            Ok(Token::Comma),
            Ok(Token::Ident("bar")),
            Ok(Token::Colon),
            Ok(Token::IntLiteral(20)),
        ]
    );
    assert_eq!(
        Token::lexer("foo: 10[com.foo.ext]: 20").collect::<Vec<_>>(),
        vec![
            Ok(Token::Ident("foo")),
            Ok(Token::Colon),
            Ok(Token::IntLiteral(10)),
            Ok(Token::LeftBracket),
            Ok(Token::Ident("com")),
            Ok(Token::Dot),
            Ok(Token::Ident("foo")),
            Ok(Token::Dot),
            Ok(Token::Ident("ext")),
            Ok(Token::RightBracket),
            Ok(Token::Colon),
            Ok(Token::IntLiteral(20)),
        ]
    );

    let mut lexer = Token::lexer("foo: 10bar: 20_foo");
    assert_eq!(
        lexer.by_ref().collect::<Vec<_>>(),
        vec![
            Ok(Token::Ident("foo")),
            Ok(Token::Colon),
            Ok(Token::IntLiteral(10)),
            Ok(Token::Ident("bar")),
            Ok(Token::Colon),
            Ok(Token::IntLiteral(20)),
            Ok(Token::Ident("_foo")),
        ]
    );
    assert_eq!(
        lexer.extras.errors,
        vec![
            ParseErrorKind::NoSpaceBetweenIntAndIdent { span: 5..10 },
            ParseErrorKind::NoSpaceBetweenIntAndIdent { span: 12..18 },
        ]
    );
}

// TODO Disabled for now due to logos bug: https://github.com/maciejhirsz/logos/issues/255
// #[test]
// fn prop_regression_1() {
//     let mut lexer = Token::lexer("08¡");

//     assert_eq!(lexer.next(), Some(Token::IntLiteral(0)));
//     assert_eq!(lexer.next(), Some(Token::Error));
//     assert_eq!(lexer.next(), None);
// }

// proptest! {
//     #[test]
//     fn prop_lex_random_string(s in ".{2,256}") {
//         // Should produce at least one 'Error' token.
//         assert_ne!(Token::lexer(&s).count(), 0);
//     }
// }
