#[cfg(test)]
mod tests;

use std::{ascii, borrow::Cow, convert::TryInto, fmt, num::IntErrorKind};

use logos::{Lexer, Logos};

use super::error::ParseErrorKind;

#[derive(Debug, Clone, Logos, PartialEq, Eq)]
#[logos(extras = TokenExtras)]
#[logos(skip r"[\t\v\f\r ]+")]
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
    #[token("/")]
    ForwardSlash,
    #[regex(r#"(//|#)[^\n]*\n?"#, line_comment)]
    LineComment(Cow<'a, str>),
    #[token(r#"/*"#, block_comment)]
    BlockComment(Cow<'a, str>),
    #[token("\n")]
    Newline,
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

impl Token<'_> {
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

impl fmt::Display for Token<'_> {
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
            Token::ForwardSlash => write!(f, "/"),
            Token::LineComment(value) => writeln!(f, "//{}", value),
            Token::BlockComment(value) => write!(f, "/*{}*/", value),
            Token::Newline => writeln!(f),
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
            _ => unreachable!("failed to parse char escape"),
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
            Some(Ok(Component::Unescaped(s))) => cow_push_bytes(&mut result, s.as_bytes()),
            Some(Ok(Component::Terminator(t))) if t == terminator => {
                break;
            }
            Some(Ok(Component::Terminator(ch) | Component::Byte(ch))) => {
                result.get_or_insert_with(Cow::default).to_mut().push(ch)
            }
            Some(Ok(Component::Char(ch))) => {
                let mut buf = [0; 4];
                let ch = ch.encode_utf8(&mut buf);
                result
                    .get_or_insert_with(Cow::default)
                    .to_mut()
                    .extend_from_slice(ch.as_bytes())
            }
            Some(Err(_)) => {
                let start = lex.span().end + char_lexer.span().start;
                let end = lex.span().end + char_lexer.span().end;

                if char_lexer.slice().contains('\n') {
                    lex.extras
                        .errors
                        .push(ParseErrorKind::UnterminatedString { span: start..end });
                    break;
                } else {
                    match lex.extras.errors.last_mut() {
                        Some(
                            ParseErrorKind::InvalidStringCharacters { span: err_span }
                            | ParseErrorKind::InvalidStringEscape { span: err_span },
                        ) if err_span.end == start => {
                            *err_span = err_span.start..end;
                            continue;
                        }
                        _ => {
                            if char_lexer.slice().starts_with('\\') {
                                lex.extras
                                    .errors
                                    .push(ParseErrorKind::InvalidStringEscape { span: start..end });
                                continue;
                            } else {
                                lex.extras
                                    .errors
                                    .push(ParseErrorKind::InvalidStringCharacters {
                                        span: start..end,
                                    });
                                continue;
                            }
                        }
                    }
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
    }

    let mut comment_lexer = Component::lexer(lex.remainder());
    let mut result: Option<Cow<'a, str>> = None;

    let mut depth = 1u32;
    let mut last_end = None;
    let len = loop {
        match comment_lexer.next() {
            Some(Ok(Component::EndComment)) => {
                depth -= 1;
                if depth == 0 {
                    break comment_lexer.span().end;
                } else {
                    last_end = Some(comment_lexer.span().end);
                }
            }
            Some(Ok(Component::StartComment)) => {
                let start = lex.span().end + comment_lexer.span().start;
                let end = lex.span().end + comment_lexer.span().end;
                lex.extras
                    .errors
                    .push(ParseErrorKind::NestedBlockComment { span: start..end });
                depth += 1;
            }
            Some(Ok(Component::Newline)) => {
                cow_push_str(&mut result, "\n");
                let stripped = comment_lexer.remainder().trim_start();
                comment_lexer.bump(comment_lexer.remainder().len() - stripped.len());
                if stripped.starts_with('*') && !stripped.starts_with("*/") {
                    comment_lexer.bump(1);
                }
            }
            Some(Err(())) => cow_push_str(&mut result, comment_lexer.slice()),
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
    match cow {
        Some(cow) => cow.to_mut().push_str(s),
        None => *cow = Some(Cow::Borrowed(s)),
    }
}

fn cow_push_bytes<'a>(cow: &mut Option<Cow<'a, [u8]>>, s: &'a [u8]) {
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
