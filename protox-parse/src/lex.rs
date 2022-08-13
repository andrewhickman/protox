use std::{ascii, borrow::Cow, convert::TryInto, fmt, num::IntErrorKind};

use logos::{skip, Lexer, Logos};

use super::ParseErrorKind;

#[derive(Debug, Clone, Logos, PartialEq, Eq)]
#[logos(extras = TokenExtras)]
#[logos(subpattern exponent = r"[eE][+\-]?[0-9]+")]
pub(crate) enum Token<'a> {
    #[regex("[A-Za-z_][A-Za-z0-9_]*")]
    Ident(&'a str),
    #[regex("0", |_| 0)]
    #[regex("0[0-7]+", |lex| int(lex, 8, 1))]
    #[regex("[1-9][0-9]*", |lex| int(lex, 10, 0))]
    #[regex("0[xX][0-9A-Fa-f]+", |lex| int(lex, 16, 2))]
    IntLiteral(u64),
    #[regex("0[fF]", float)]
    #[regex("[1-9][0-9]*[fF]", float)]
    #[regex(r#"[0-9]+\.[0-9]*(?&exponent)?[fF]?"#, float)]
    #[regex(r#"[0-9]+(?&exponent)[fF]?"#, float)]
    #[regex(r#"\.[0-9]+(?&exponent)?[fF]?"#, float)]
    FloatLiteral(EqFloat),
    #[regex(r#"'|""#, string)]
    StringLiteral(Cow<'a, [u8]>),
    #[token(".")]
    Dot,
    #[token("-")]
    Minus,
    #[token("+")]
    Plus,
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("<")]
    LeftAngleBracket,
    #[token(">")]
    RightAngleBracket,
    #[token(",")]
    Comma,
    #[token("=")]
    Equals,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[regex(r#"(//|#)[^\n]*\n?"#, line_comment)]
    LineComment(Cow<'a, str>),
    #[token(r#"/*"#, block_comment)]
    BlockComment(Cow<'a, str>),
    #[token("\n")]
    Newline,
    #[error]
    #[regex(r"[\t\v\f\r ]+", skip)]
    Error,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct EqFloat(pub f64);

impl EqFloat {
    fn new(f: f64) -> Self {
        debug_assert!(!f.is_nan());
        EqFloat(f)
    }
}

impl Eq for EqFloat {}

impl<'a> Token<'a> {
    pub const SYNTAX: Token<'static> = Token::Ident("syntax");
    pub const PACKAGE: Token<'static> = Token::Ident("package");
    pub const IMPORT: Token<'static> = Token::Ident("import");
    pub const WEAK: Token<'static> = Token::Ident("weak");
    pub const PUBLIC: Token<'static> = Token::Ident("public");
    pub const ENUM: Token<'static> = Token::Ident("enum");
    pub const OPTION: Token<'static> = Token::Ident("option");
    pub const SERVICE: Token<'static> = Token::Ident("service");
    pub const RPC: Token<'static> = Token::Ident("rpc");
    pub const STREAM: Token<'static> = Token::Ident("stream");
    pub const RETURNS: Token<'static> = Token::Ident("returns");
    pub const EXTEND: Token<'static> = Token::Ident("extend");
    pub const MESSAGE: Token<'static> = Token::Ident("message");
    pub const OPTIONAL: Token<'static> = Token::Ident("optional");
    pub const REQUIRED: Token<'static> = Token::Ident("required");
    pub const REPEATED: Token<'static> = Token::Ident("repeated");
    pub const MAP: Token<'static> = Token::Ident("map");
    pub const ONEOF: Token<'static> = Token::Ident("oneof");
    pub const GROUP: Token<'static> = Token::Ident("group");
    pub const DOUBLE: Token<'static> = Token::Ident("double");
    pub const FLOAT: Token<'static> = Token::Ident("float");
    pub const INT32: Token<'static> = Token::Ident("int32");
    pub const INT64: Token<'static> = Token::Ident("int64");
    pub const UINT32: Token<'static> = Token::Ident("uint32");
    pub const UINT64: Token<'static> = Token::Ident("uint64");
    pub const SINT32: Token<'static> = Token::Ident("sint32");
    pub const SINT64: Token<'static> = Token::Ident("sint64");
    pub const FIXED32: Token<'static> = Token::Ident("fixed32");
    pub const FIXED64: Token<'static> = Token::Ident("fixed64");
    pub const SFIXED32: Token<'static> = Token::Ident("sfixed32");
    pub const SFIXED64: Token<'static> = Token::Ident("sfixed64");
    pub const BOOL: Token<'static> = Token::Ident("bool");
    pub const STRING: Token<'static> = Token::Ident("string");
    pub const BYTES: Token<'static> = Token::Ident("bytes");
    pub const RESERVED: Token<'static> = Token::Ident("reserved");
    pub const EXTENSIONS: Token<'static> = Token::Ident("extensions");
    pub const TO: Token<'static> = Token::Ident("to");
    pub const MAX: Token<'static> = Token::Ident("max");
}

impl<'a> fmt::Display for Token<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Ident(value) => write!(f, "{}", value),
            Token::IntLiteral(value) => write!(f, "{}", value),
            Token::FloatLiteral(value) => {
                if value.0.fract() == 0.0 {
                    write!(f, "{:.1}", value.0)
                } else {
                    write!(f, "{}", value.0)
                }
            }
            Token::StringLiteral(bytes) => {
                write!(f, "\"")?;
                for &ch in bytes.as_ref() {
                    write!(f, "{}", ascii::escape_default(ch))?;
                }
                write!(f, "\"")?;
                Ok(())
            }
            Token::Dot => write!(f, "."),
            Token::Minus => write!(f, "-"),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::LeftBracket => write!(f, "["),
            Token::RightBracket => write!(f, "]"),
            Token::LeftAngleBracket => write!(f, "<"),
            Token::RightAngleBracket => write!(f, ">"),
            Token::Comma => write!(f, ","),
            Token::Plus => write!(f, "+"),
            Token::Equals => write!(f, "="),
            Token::Colon => write!(f, ":"),
            Token::Semicolon => write!(f, ";"),
            Token::LineComment(value) => writeln!(f, "//{}", value),
            Token::BlockComment(value) => write!(f, "/*{}*/", value),
            Token::Newline => writeln!(f),
            Token::Error => write!(f, "<ERROR>"),
        }
    }
}

#[derive(Default)]
pub(crate) struct TokenExtras {
    pub errors: Vec<ParseErrorKind>,
    pub text_format_mode: bool,
}

fn int<'a>(lex: &mut Lexer<'a, Token<'a>>, radix: u32, prefix_len: usize) -> Result<u64, ()> {
    debug_assert!(lex.slice().len() > prefix_len);
    let span = lex.span().start + prefix_len..lex.span().end;

    if matches!(lex.remainder().chars().next(), Some(ch) if ch.is_ascii_alphabetic() || ch == '_') {
        let mut end = span.end + 1;
        while end < lex.source().len() && lex.source().as_bytes()[end].is_ascii_alphabetic() {
            end += 1;
        }
        lex.extras
            .errors
            .push(ParseErrorKind::NoSpaceBetweenIntAndIdent {
                span: span.start..end,
            })
    }

    match u64::from_str_radix(&lex.source()[span.clone()], radix) {
        Ok(value) => Ok(value),
        Err(err) => {
            debug_assert_eq!(err.kind(), &IntErrorKind::PosOverflow);
            lex.extras
                .errors
                .push(ParseErrorKind::IntegerOutOfRange { span });
            Ok(Default::default())
        }
    }
}

fn float<'a>(lex: &mut Lexer<'a, Token<'a>>) -> EqFloat {
    let start = lex.span().start;
    let last = lex.span().end - 1;
    let s = match lex.source().as_bytes()[last] {
        b'f' | b'F' => {
            if !lex.extras.text_format_mode {
                lex.extras
                    .errors
                    .push(ParseErrorKind::FloatSuffixOutsideTextFormat { span: lex.span() });
            }
            &lex.source()[start..last]
        }
        _ => lex.slice(),
    };

    EqFloat::new(s.parse().expect("failed to parse float"))
}

fn string<'a>(lex: &mut Lexer<'a, Token<'a>>) -> Cow<'a, [u8]> {
    #[derive(Logos)]
    #[logos(subpattern hex = r"[0-9A-Fa-f]")]
    enum Component<'a> {
        #[regex(r#"[^\x00\n\\'"]+"#)]
        Unescaped(&'a str),
        #[regex(r#"['"]"#, terminator)]
        Terminator(u8),
        #[regex(r#"\\[xX](?&hex)(?&hex)?"#, hex_escape)]
        #[regex(r#"\\[0-7][0-7]?[0-7]?"#, oct_escape)]
        #[regex(r#"\\[abfnrtv?\\'"]"#, char_escape)]
        Byte(u8),
        #[regex(r#"\\u(?&hex)(?&hex)(?&hex)(?&hex)"#, unicode_escape)]
        #[regex(
            r#"\\U(?&hex)(?&hex)(?&hex)(?&hex)(?&hex)(?&hex)(?&hex)(?&hex)"#,
            unicode_escape
        )]
        Char(char),
        #[error]
        Error,
    }

    fn terminator<'a>(lex: &mut Lexer<'a, Component<'a>>) -> u8 {
        debug_assert_eq!(lex.slice().len(), 1);
        lex.slice().bytes().next().unwrap()
    }

    fn hex_escape<'a>(lex: &mut Lexer<'a, Component<'a>>) -> u8 {
        u32::from_str_radix(&lex.slice()[2..], 16)
            .expect("expected valid hex escape")
            .try_into()
            .expect("two-digit hex escape should be valid byte")
    }

    fn oct_escape<'a>(lex: &mut Lexer<'a, Component<'a>>) -> Result<u8, ()> {
        u32::from_str_radix(&lex.slice()[1..], 8)
            .expect("expected valid oct escape")
            .try_into()
            .map_err(drop)
    }

    fn char_escape<'a>(lex: &mut Lexer<'a, Component<'a>>) -> u8 {
        match lex.slice().as_bytes()[1] {
            b'a' => b'\x07',
            b'b' => b'\x08',
            b'f' => b'\x0c',
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'v' => b'\x0b',
            b'?' => b'?',
            b'\\' => b'\\',
            b'\'' => b'\'',
            b'"' => b'"',
            _ => panic!("failed to parse char escape"),
        }
    }

    fn unicode_escape<'a>(lex: &mut Lexer<'a, Component<'a>>) -> Option<char> {
        let value = u32::from_str_radix(&lex.slice()[2..], 16).expect("expected valid hex escape");
        char::from_u32(value)
    }

    let mut result: Option<Cow<'a, [u8]>> = None;

    let mut char_lexer = Component::lexer(lex.remainder());
    let terminator = lex.slice().as_bytes()[0];

    loop {
        match char_lexer.next() {
            Some(Component::Unescaped(s)) => cow_push_bytes(&mut result, s.as_bytes()),
            Some(Component::Terminator(t)) if t == terminator => {
                break;
            }
            Some(Component::Terminator(ch) | Component::Byte(ch)) => {
                result.get_or_insert_with(Cow::default).to_mut().push(ch)
            }
            Some(Component::Char(ch)) => {
                let mut buf = [0; 4];
                let ch = ch.encode_utf8(&mut buf);
                result
                    .get_or_insert_with(Cow::default)
                    .to_mut()
                    .extend_from_slice(ch.as_bytes())
            }
            Some(Component::Error) => {
                let start = lex.span().end + char_lexer.span().start;
                let end = lex.span().end + char_lexer.span().end;

                if char_lexer.slice().contains('\n') {
                    lex.extras
                        .errors
                        .push(ParseErrorKind::UnterminatedString { span: start..end });
                    break;
                } else if let Some(err) = lex.extras.errors.last_mut() {
                    match err {
                        ParseErrorKind::InvalidStringCharacters { span: err_span }
                        | ParseErrorKind::InvalidStringEscape { span: err_span } => {
                            // If the last character was invalid, extend the span of its error
                            // instead of adding a new error.
                            if err_span.end == start {
                                *err_span = err_span.start..end;
                                continue;
                            }
                        }
                        _ => (),
                    }
                } else if char_lexer.slice().starts_with('\\') {
                    lex.extras
                        .errors
                        .push(ParseErrorKind::InvalidStringEscape { span: start..end });
                    continue;
                } else {
                    lex.extras
                        .errors
                        .push(ParseErrorKind::InvalidStringCharacters { span: start..end });
                    continue;
                }
            }
            None => {
                lex.extras.errors.push(ParseErrorKind::UnexpectedEof {
                    expected: "string terminator".to_owned(),
                });
                break;
            }
        }
    }

    lex.bump(char_lexer.span().end);
    result.unwrap_or_default()
}

fn line_comment<'a>(lex: &mut Lexer<'a, Token<'a>>) -> Cow<'a, str> {
    if !lex.extras.text_format_mode && lex.slice().starts_with('#') {
        lex.extras
            .errors
            .push(ParseErrorKind::HashCommentOutsideTextFormat { span: lex.span() });
    }

    let content = lex
        .slice()
        .strip_prefix("//")
        .or_else(|| lex.slice().strip_prefix('#'))
        .expect("invalid line comment");
    normalize_newlines(content.into())
}

fn block_comment<'a>(lex: &mut Lexer<'a, Token<'a>>) -> Cow<'a, str> {
    #[derive(Logos)]
    enum Component {
        #[regex(r#"\*/[\t\v\f\r ]*"#)]
        EndComment,
        #[token("/*")]
        StartComment,
        #[token("\n")]
        Newline,
        #[error]
        Text,
    }

    let mut comment_lexer = Component::lexer(lex.remainder());
    let mut result: Option<Cow<'a, str>> = None;

    let mut depth = 1u32;
    let mut last_end = None;
    let len = loop {
        match comment_lexer.next() {
            Some(Component::EndComment) => {
                depth -= 1;
                if depth == 0 {
                    break comment_lexer.span().end;
                } else {
                    last_end = Some(comment_lexer.span().end);
                }
            }
            Some(Component::StartComment) => {
                let start = lex.span().end + comment_lexer.span().start;
                let end = lex.span().end + comment_lexer.span().end;
                lex.extras
                    .errors
                    .push(ParseErrorKind::NestedBlockComment { span: start..end });
                depth += 1;
            }
            Some(Component::Newline) => {
                cow_push_str(&mut result, "\n");
                let stripped = comment_lexer.remainder().trim_start();
                comment_lexer.bump(comment_lexer.remainder().len() - stripped.len());
                if stripped.starts_with('*') && !stripped.starts_with("*/") {
                    comment_lexer.bump(1);
                }
            }
            Some(Component::Text) => cow_push_str(&mut result, comment_lexer.slice()),
            None => {
                if let Some(last_end) = last_end {
                    // This must be a nested block comment
                    break last_end;
                } else {
                    lex.extras.errors.push(ParseErrorKind::UnexpectedEof {
                        expected: "comment terminator".to_owned(),
                    });
                    break lex.remainder().len();
                }
            }
        }
    };

    lex.bump(len);
    normalize_newlines(result.unwrap_or_default())
}

fn cow_push_str<'a>(cow: &mut Option<Cow<'a, str>>, s: &'a str) {
    if s.is_empty() {
        return;
    }

    match cow {
        Some(cow) => cow.to_mut().push_str(s),
        None => *cow = Some(Cow::Borrowed(s)),
    }
}

fn cow_push_bytes<'a>(cow: &mut Option<Cow<'a, [u8]>>, s: &'a [u8]) {
    if s.is_empty() {
        return;
    }

    match cow {
        Some(cow) => cow.to_mut().extend_from_slice(s),
        None => *cow = Some(Cow::Borrowed(s)),
    }
}

fn normalize_newlines(s: Cow<str>) -> Cow<str> {
    if s.contains("\r\n") {
        Cow::Owned(s.replace("\r\n", "\n"))
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // use proptest::prelude::*;

    #[test]
    fn simple_tokens() {
        let source = r#"hell0 052 42 0x2A 5. 0.5 0.42e+2 2e-4 .2e+3 52e3 true
            false "hello \a\b\f\n\r\t\v\?\\\'\" \052 \x2a" 'hello 😀' _foo"#;
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next().unwrap(), Token::Ident("hell0"));
        assert_eq!(lexer.next().unwrap(), Token::IntLiteral(42));
        assert_eq!(lexer.next().unwrap(), Token::IntLiteral(42));
        assert_eq!(lexer.next().unwrap(), Token::IntLiteral(42));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(5.)));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(0.5)));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(0.42e+2)));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(2e-4)));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(0.2e+3)));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(52e3)));
        assert_eq!(lexer.next().unwrap(), Token::Ident("true"));
        assert_eq!(lexer.next().unwrap(), Token::Newline);
        assert_eq!(lexer.next().unwrap(), Token::Ident("false"));
        assert_eq!(
            lexer.next().unwrap(),
            Token::StringLiteral(b"hello \x07\x08\x0c\n\r\t\x0b?\\'\" * *".as_ref().into())
        );
        assert_eq!(
            lexer.next().unwrap(),
            Token::StringLiteral(b"hello \xF0\x9F\x98\x80".as_ref().into())
        );
        assert_eq!(lexer.next().unwrap(), Token::Ident("_foo"));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn integer_overflow() {
        let source = "99999999999999999999999999999999999999 4";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::IntLiteral(0)));
        assert_eq!(lexer.next(), Some(Token::IntLiteral(4)));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
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

        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(10.)));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(5.)));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(0.5)));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(0.42e+2)));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(2e-4)));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(0.2e+3)));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
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

        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(10.)));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(5.)));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(0.5)));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(0.42e+2)));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(2e-4)));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(EqFloat(0.2e+3)));
        assert_eq!(lexer.next(), None);
        assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn invalid_token() {
        let source = "@ foo";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Error));
        assert_eq!(lexer.next(), Some(Token::Ident("foo")));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn invalid_string_char() {
        let source = "\"\x00\" foo";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::StringLiteral(Default::default())));
        assert_eq!(lexer.next(), Some(Token::Ident("foo")));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
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
            Some(Token::StringLiteral(b"hello ".as_ref().into()))
        );
        assert_eq!(lexer.next(), Some(Token::Ident("foo")));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
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
            Some(Token::StringLiteral(b"m".as_ref().into()))
        );
        assert_eq!(lexer.next(), Some(Token::Ident("foo")));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
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
            Some(Token::StringLiteral([0xff].as_ref().into()))
        );
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn merge_string_errors() {
        let source = "\"\\\x00\" foo";
        let mut lexer = Token::lexer(source);

        assert_eq!(
            lexer.next(),
            Some(Token::StringLiteral(b"".as_ref().into()))
        );
        assert_eq!(lexer.next(), Some(Token::Ident("foo")));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseErrorKind::InvalidStringEscape { span: 1..3 }]
        );
    }

    #[test]
    fn string_unicode_escape() {
        let source = r#"'\u0068\u0065\u006c\u006c\u006f\u0020\U0001f600'"#;
        let mut lexer = Token::lexer(source);

        assert_eq!(
            lexer.next(),
            Some(Token::StringLiteral(
                b"hello \xF0\x9F\x98\x80".as_ref().into()
            ))
        );
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn string_invalid_unicode_escape() {
        let mut lexer = Token::lexer(r#"'\Uffffffff'"#);
        lexer.by_ref().for_each(drop);
        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseErrorKind::InvalidStringEscape { span: 1..11 }]
        );
    }

    #[test]
    fn line_comment() {
        let source = "foo // bar \n quz";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Ident("foo")));
        assert_eq!(lexer.next(), Some(Token::LineComment(" bar \n".into())));
        assert_eq!(lexer.next(), Some(Token::Ident("quz")));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn block_comment() {
        let source = "foo /* bar\n */ quz";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Ident("foo")));
        assert_eq!(lexer.next(), Some(Token::BlockComment(" bar\n".into())));
        assert_eq!(lexer.next(), Some(Token::Ident("quz")));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn block_comment_multiline() {
        let source = "/* foo\n * bar\n quz*/";
        let mut lexer = Token::lexer(source);

        assert_eq!(
            lexer.next(),
            Some(Token::BlockComment(" foo\n bar\nquz".into()))
        );
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn nested_block_comment() {
        let source = "foo /* /* bar\n */ */ quz";
        let mut lexer = Token::lexer(source);

        for _ in &mut lexer {}

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseErrorKind::NestedBlockComment { span: 7..9 }]
        );
    }

    #[test]
    fn nested_block_comment_unterminated() {
        let source = "foo /* /* bar\n */ quz";
        let mut lexer = Token::lexer(source);

        for _ in &mut lexer {}

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseErrorKind::NestedBlockComment { span: 7..9 }]
        );
    }

    #[test]
    fn block_comment_unterminated() {
        let source = "foo /* bar\n quz";
        let mut lexer = Token::lexer(source);

        for _ in &mut lexer {}

        debug_assert_eq!(
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

        assert_eq!(lexer.next(), Some(Token::BlockComment(" bar ".into())));
        assert_eq!(lexer.next(), Some(Token::Newline));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn hash_comment() {
        let source = "# bar";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::LineComment(" bar".into())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseErrorKind::HashCommentOutsideTextFormat { span: 0..5 }]
        );

        let mut lexer = Token::lexer(source);
        lexer.extras.text_format_mode = true;

        assert_eq!(lexer.next(), Some(Token::LineComment(" bar".into())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn whitespace() {
        assert_eq!(
            Token::lexer("value: -2.0").collect::<Vec<_>>(),
            vec![
                Token::Ident("value"),
                Token::Colon,
                Token::Minus,
                Token::FloatLiteral(EqFloat(2.0)),
            ]
        );
        assert_eq!(
            Token::lexer("value: - 2.0").collect::<Vec<_>>(),
            vec![
                Token::Ident("value"),
                Token::Colon,
                Token::Minus,
                Token::FloatLiteral(EqFloat(2.0)),
            ]
        );
        assert_eq!(
            Token::lexer("value: -\n  #comment\n  2.0").collect::<Vec<_>>(),
            vec![
                Token::Ident("value"),
                Token::Colon,
                Token::Minus,
                Token::Newline,
                Token::LineComment("comment\n".into()),
                Token::FloatLiteral(EqFloat(2.0)),
            ]
        );
        assert_eq!(
            Token::lexer("value: 2 . 0").collect::<Vec<_>>(),
            vec![
                Token::Ident("value"),
                Token::Colon,
                Token::IntLiteral(2),
                Token::Dot,
                Token::IntLiteral(0),
            ]
        );

        assert_eq!(
            Token::lexer("foo: 10 bar: 20").collect::<Vec<_>>(),
            vec![
                Token::Ident("foo"),
                Token::Colon,
                Token::IntLiteral(10),
                Token::Ident("bar"),
                Token::Colon,
                Token::IntLiteral(20),
            ]
        );
        assert_eq!(
            Token::lexer("foo: 10,bar: 20").collect::<Vec<_>>(),
            vec![
                Token::Ident("foo"),
                Token::Colon,
                Token::IntLiteral(10),
                Token::Comma,
                Token::Ident("bar"),
                Token::Colon,
                Token::IntLiteral(20),
            ]
        );
        assert_eq!(
            Token::lexer("foo: 10[com.foo.ext]: 20").collect::<Vec<_>>(),
            vec![
                Token::Ident("foo"),
                Token::Colon,
                Token::IntLiteral(10),
                Token::LeftBracket,
                Token::Ident("com"),
                Token::Dot,
                Token::Ident("foo"),
                Token::Dot,
                Token::Ident("ext"),
                Token::RightBracket,
                Token::Colon,
                Token::IntLiteral(20),
            ]
        );

        let mut lexer = Token::lexer("foo: 10bar: 20_foo");
        assert_eq!(
            lexer.by_ref().collect::<Vec<_>>(),
            vec![
                Token::Ident("foo"),
                Token::Colon,
                Token::IntLiteral(10),
                Token::Ident("bar"),
                Token::Colon,
                Token::IntLiteral(20),
                Token::Ident("_foo"),
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
}
