use std::{borrow::Cow, convert::TryInto, fmt, num::IntErrorKind};

use logos::{skip, Lexer, Logos};

use super::ParseError;

#[derive(Debug, Clone, Logos, PartialEq)]
#[logos(extras = TokenExtras)]
#[logos(subpattern exponent = r"[eE][+\-][0-9]+")]
pub(crate) enum Token<'a> {
    #[regex("[A-Za-z_][A-Za-z0-9_]*", ident)]
    Ident(Cow<'a, str>),
    #[regex("0[0-7]*", |lex| int(lex, 8, 1))]
    #[regex("[1-9][0-9]*", |lex| int(lex, 10, 0))]
    #[regex("0[xX][0-9A-Fa-f]+", |lex| int(lex, 16, 2))]
    IntLiteral(u64),
    #[regex(r#"[0-9]+\.[0-9]*(?&exponent)?"#, float)]
    #[regex(r#"[0-9]+(?&exponent)"#, float)]
    #[regex(r#"\.[0-9]+(?&exponent)?"#, float)]
    FloatLiteral(f64),
    #[regex("false|true", bool)]
    BoolLiteral(bool),
    #[regex(r#"'|""#, string)]
    StringLiteral(Cow<'a, str>),
    #[token("syntax")]
    Syntax,
    #[token("package")]
    Package,
    #[token("import")]
    Import,
    #[token("weak")]
    Weak,
    #[token("public")]
    Public,
    #[token("enum")]
    Enum,
    #[token("option")]
    Option,
    #[token("service")]
    Service,
    #[token("rpc")]
    Rpc,
    #[token("stream")]
    Stream,
    #[token("returns")]
    Returns,
    #[token("extend")]
    Extend,
    #[token("message")]
    Message,
    #[token("optional")]
    Optional,
    #[token("required")]
    Required,
    #[token("repeated")]
    Repeated,
    #[token("map")]
    Map,
    #[token("oneof")]
    Oneof,
    #[token("group")]
    Group,
    #[token("double")]
    Double,
    #[token("float")]
    Float,
    #[token("int32")]
    Int32,
    #[token("int64")]
    Int64,
    #[token("uint32")]
    Uint32,
    #[token("uint64")]
    Uint64,
    #[token("sint32")]
    Sint32,
    #[token("sint64")]
    Sint64,
    #[token("fixed32")]
    Fixed32,
    #[token("fixed64")]
    Fixed64,
    #[token("sfixed32")]
    Sfixed32,
    #[token("sfixed64")]
    Sfixed64,
    #[token("bool")]
    Bool,
    #[token("string")]
    String,
    #[token("bytes")]
    Bytes,
    #[token("reserved")]
    Reserved,
    #[token("extensions")]
    Extensions,
    #[token("to")]
    To,
    #[token("max")]
    Max,
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
    #[token(";")]
    Semicolon,
    #[regex(r#"//[^\n]*\n?"#, line_comment)]
    #[token(r#"/*"#, block_comment)]
    Comment(Cow<'a, str>),
    #[token("\n")]
    Newline,
    #[error]
    #[regex(r"[\t\v\f\r ]+", skip)]
    Error,
}

impl<'a> Token<'a> {
    pub fn as_ident(&self) -> Option<&str> {
        match self {
            Token::Ident(value) => Some(value),
            Token::BoolLiteral(false) => Some("false"),
            Token::BoolLiteral(true) => Some("true"),
            Token::Syntax => Some("syntax"),
            Token::Import => Some("import"),
            Token::Weak => Some("weak"),
            Token::Public => Some("public"),
            Token::Package => Some("package"),
            Token::Option => Some("option"),
            Token::Enum => Some("enum"),
            Token::Service => Some("service"),
            Token::Rpc => Some("rpc"),
            Token::Stream => Some("stream"),
            Token::Returns => Some("returns"),
            Token::Extend => Some("extend"),
            Token::Message => Some("message"),
            Token::Optional => Some("optional"),
            Token::Required => Some("required"),
            Token::Repeated => Some("repeated"),
            Token::Map => Some("map"),
            Token::Group => Some("group"),
            Token::Oneof => Some("oneof"),
            Token::Double => Some("double"),
            Token::Float => Some("float"),
            Token::Int32 => Some("int32"),
            Token::Int64 => Some("int64"),
            Token::Uint32 => Some("uint32"),
            Token::Uint64 => Some("uint64"),
            Token::Sint32 => Some("sint32"),
            Token::Sint64 => Some("sint64"),
            Token::Fixed32 => Some("fixed32"),
            Token::Fixed64 => Some("fixed64"),
            Token::Sfixed32 => Some("sfixed32"),
            Token::Sfixed64 => Some("sfixed64"),
            Token::Bool => Some("bool"),
            Token::String => Some("string"),
            Token::Bytes => Some("bytes"),
            Token::Reserved => Some("reserved"),
            Token::Extensions => Some("extensions"),
            Token::To => Some("to"),
            Token::Max => Some("max"),
            _ => None,
        }
    }

    pub fn to_static(&self) -> Token<'static> {
        match self {
            Token::Ident(value) => Token::Ident(Cow::Owned(value.clone().into_owned())),
            Token::IntLiteral(value) => Token::IntLiteral(*value),
            Token::FloatLiteral(value) => Token::FloatLiteral(*value),
            Token::BoolLiteral(value) => Token::BoolLiteral(*value),
            Token::StringLiteral(value) => {
                Token::StringLiteral(Cow::Owned(value.clone().into_owned()))
            }
            Token::Syntax => Token::Syntax,
            Token::Package => Token::Package,
            Token::Import => Token::Import,
            Token::Weak => Token::Weak,
            Token::Public => Token::Public,
            Token::Enum => Token::Enum,
            Token::Option => Token::Option,
            Token::Service => Token::Service,
            Token::Rpc => Token::Rpc,
            Token::Stream => Token::Stream,
            Token::Returns => Token::Returns,
            Token::Extend => Token::Extend,
            Token::Message => Token::Message,
            Token::Optional => Token::Optional,
            Token::Required => Token::Required,
            Token::Repeated => Token::Repeated,
            Token::Map => Token::Map,
            Token::Oneof => Token::Oneof,
            Token::Group => Token::Group,
            Token::Double => Token::Double,
            Token::Float => Token::Float,
            Token::Int32 => Token::Int32,
            Token::Int64 => Token::Int64,
            Token::Uint32 => Token::Uint32,
            Token::Uint64 => Token::Uint64,
            Token::Sint32 => Token::Sint32,
            Token::Sint64 => Token::Sint64,
            Token::Fixed32 => Token::Fixed32,
            Token::Fixed64 => Token::Fixed64,
            Token::Sfixed32 => Token::Sfixed32,
            Token::Sfixed64 => Token::Sfixed64,
            Token::Bool => Token::Bool,
            Token::String => Token::String,
            Token::Bytes => Token::Bytes,
            Token::Reserved => Token::Reserved,
            Token::Extensions => Token::Extensions,
            Token::To => Token::To,
            Token::Max => Token::Max,
            Token::Dot => Token::Dot,
            Token::Minus => Token::Minus,
            Token::Plus => Token::Plus,
            Token::LeftParen => Token::LeftParen,
            Token::RightParen => Token::RightParen,
            Token::LeftBrace => Token::LeftBrace,
            Token::RightBrace => Token::RightBrace,
            Token::LeftBracket => Token::LeftBracket,
            Token::RightBracket => Token::RightBracket,
            Token::LeftAngleBracket => Token::LeftAngleBracket,
            Token::RightAngleBracket => Token::RightAngleBracket,
            Token::Comma => Token::Comma,
            Token::Equals => Token::Equals,
            Token::Semicolon => Token::Semicolon,
            Token::Comment(value) => Token::Comment(Cow::Owned(value.clone().into_owned())),
            Token::Newline => Token::Newline,
            Token::Error => Token::Error,
        }
    }
}

impl<'a> fmt::Display for Token<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Ident(value) => write!(f, "{}", value),
            Token::IntLiteral(value) => write!(f, "{}", value),
            Token::FloatLiteral(value) => write!(f, "{}", value),
            Token::BoolLiteral(value) => write!(f, "{}", value),
            Token::StringLiteral(string) => {
                write!(f, "\"{}\"", string.escape_default())
            }
            Token::Syntax => write!(f, "syntax"),
            Token::Import => write!(f, "import"),
            Token::Weak => write!(f, "weak"),
            Token::Public => write!(f, "public"),
            Token::Package => write!(f, "package"),
            Token::Enum => write!(f, "enum"),
            Token::Option => write!(f, "option"),
            Token::Service => write!(f, "service"),
            Token::Stream => write!(f, "stream"),
            Token::Returns => write!(f, "returns"),
            Token::Extend => write!(f, "extend"),
            Token::Message => write!(f, "message"),
            Token::Optional => write!(f, "optional"),
            Token::Required => write!(f, "required"),
            Token::Repeated => write!(f, "repeated"),
            Token::Map => write!(f, "map"),
            Token::Oneof => write!(f, "oneof"),
            Token::Group => write!(f, "group"),
            Token::Double => write!(f, "double"),
            Token::Float => write!(f, "float"),
            Token::Int32 => write!(f, "int32"),
            Token::Int64 => write!(f, "int64"),
            Token::Uint32 => write!(f, "uint32"),
            Token::Uint64 => write!(f, "uint64"),
            Token::Sint32 => write!(f, "sint32"),
            Token::Sint64 => write!(f, "sint64"),
            Token::Fixed32 => write!(f, "fixed32"),
            Token::Fixed64 => write!(f, "fixed64"),
            Token::Sfixed32 => write!(f, "sfixed32"),
            Token::Sfixed64 => write!(f, "sfixed64"),
            Token::Bool => write!(f, "bool"),
            Token::String => write!(f, "string"),
            Token::Bytes => write!(f, "bytes"),
            Token::Reserved => write!(f, "reserved"),
            Token::Extensions => write!(f, "extensions"),
            Token::To => write!(f, "to"),
            Token::Max => write!(f, "max"),
            Token::Rpc => write!(f, "rpc"),
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
            Token::Semicolon => write!(f, ";"),
            Token::Comment(value) => write!(f, "/*{}*/", value),
            Token::Newline => writeln!(f),
            Token::Error => write!(f, "<ERROR>"),
        }
    }
}

#[derive(Default)]
pub(crate) struct TokenExtras {
    pub errors: Vec<ParseError>,
}

fn ident<'a>(lex: &mut Lexer<'a, Token<'a>>) -> Cow<'a, str> {
    Cow::Borrowed(lex.slice())
}

fn int<'a>(lex: &mut Lexer<'a, Token<'a>>, radix: u32, prefix_len: usize) -> u64 {
    if radix == 8 && lex.slice() == "0" {
        return 0;
    }

    debug_assert!(lex.slice().len() > prefix_len);
    match u64::from_str_radix(&lex.slice()[prefix_len..], radix) {
        Ok(value) => value,
        Err(err) => {
            debug_assert_eq!(err.kind(), &IntErrorKind::PosOverflow);
            let start = lex.span().start + prefix_len;
            let end = lex.span().end;
            lex.extras
                .errors
                .push(ParseError::IntegerOutOfRange { span: start..end });
            // Return a dummy value so we can continue parsing
            Default::default()
        }
    }
}

fn float<'a>(lex: &mut Lexer<'a, Token<'a>>) -> f64 {
    lex.slice().parse().expect("failed to parse float")
}

fn bool<'a>(lex: &mut Lexer<'a, Token<'a>>) -> bool {
    lex.slice().parse().expect("faield to parse bool")
}

fn string<'a>(lex: &mut Lexer<'a, Token<'a>>) -> Cow<'a, str> {
    #[derive(Logos)]
    enum Component<'a> {
        #[regex(r#"[^\x00\n\\'"]+"#)]
        Unescaped(&'a str),
        #[regex(r#"['"]"#, terminator)]
        Terminator(char),
        #[regex(r#"\\[xX][0-9A-Fa-f][0-9A-Fa-f]"#, hex_escape)]
        #[regex(r#"\\[0-7][0-7][0-7]"#, oct_escape)]
        #[regex(r#"\\[abfnrtv\\'"]"#, char_escape)]
        Char(char),
        #[error]
        Error,
    }

    fn terminator<'a>(lex: &mut Lexer<'a, Component<'a>>) -> char {
        debug_assert_eq!(lex.slice().chars().count(), 1);
        lex.slice().chars().next().unwrap()
    }

    fn hex_escape<'a>(lex: &mut Lexer<'a, Component<'a>>) -> char {
        u32::from_str_radix(&lex.slice()[2..], 16)
            .expect("expected valid hex escape")
            .try_into()
            .expect("two-digit hex escape should be valid char")
    }

    fn oct_escape<'a>(lex: &mut Lexer<'a, Component<'a>>) -> char {
        u32::from_str_radix(&lex.slice()[1..], 8)
            .expect("expected valid oct escape")
            .try_into()
            .expect("three-digit oct escape should be valid char")
    }

    fn char_escape<'a>(lex: &mut Lexer<'a, Component<'a>>) -> char {
        match lex.slice().as_bytes()[1] {
            b'a' => '\x07',
            b'b' => '\x08',
            b'f' => '\x0c',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'v' => '\x0b',
            b'\\' => '\\',
            b'\'' => '\'',
            b'"' => '"',
            _ => panic!("failed to parse char escape"),
        }
    }

    let mut result: Option<Cow<'a, str>> = None;

    let mut char_lexer = Component::lexer(lex.remainder());
    let terminator = lex.slice().chars().next().expect("expected char");

    loop {
        match char_lexer.next() {
            Some(Component::Unescaped(s)) => cow_push_str(&mut result, s),
            Some(Component::Terminator(t)) if t == terminator => {
                break;
            }
            Some(Component::Terminator(ch) | Component::Char(ch)) => {
                result.get_or_insert_with(Cow::default).to_mut().push(ch)
            }
            Some(Component::Error) => {
                let start = lex.span().end + char_lexer.span().start;
                let end = lex.span().end + char_lexer.span().end;

                if char_lexer.slice().contains('\n') {
                    lex.extras
                        .errors
                        .push(ParseError::UnterminatedString { span: start..end });
                    break;
                } else if let Some(err) = lex.extras.errors.last_mut() {
                    match err {
                        ParseError::InvalidStringCharacters { span: err_span }
                        | ParseError::InvalidStringEscape { span: err_span } => {
                            // If the last character was invalid, extend the span of its error
                            // instead of adding a new error.
                            if err_span.end == start {
                                *err_span = err_span.start..end;
                                continue;
                            }
                        }
                        _ => (),
                    }
                }

                if char_lexer.slice().starts_with('\\') {
                    lex.extras
                        .errors
                        .push(ParseError::InvalidStringEscape { span: start..end });
                    continue;
                } else {
                    lex.extras
                        .errors
                        .push(ParseError::InvalidStringCharacters { span: start..end });
                    continue;
                }
            }
            None => {
                lex.extras.errors.push(ParseError::UnexpectedEof {
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
    fn strip_line_comment(s: &str) -> Option<&str> {
        s.trim_start().strip_prefix("//")
    }

    let mut is_trailing = false;
    for ch in lex.source()[..lex.span().start].chars().rev() {
        if ch == '\n' {
            is_trailing = false;
            break;
        } else if !ch.is_ascii_whitespace() {
            is_trailing = true;
            break;
        }
    }

    let mut result = Cow::Borrowed(strip_line_comment(lex.slice()).expect("expected comment"));
    if !is_trailing {
        // Merge comments on subsequent lines
        let mut start = 0;
        for (end, _) in lex.remainder().match_indices('\n') {
            if let Some(comment) = strip_line_comment(&lex.remainder()[start..=end]) {
                result.to_mut().push_str(comment);
                start = end + 1;
            } else {
                break;
            }
        }
        lex.bump(start);
    }

    result
}

fn block_comment<'a>(lex: &mut Lexer<'a, Token<'a>>) -> Cow<'a, str> {
    #[derive(Logos)]
    enum Component {
        // Optionally include a trailing newline for consistency with line comments
        #[regex(r#"\*/[\t\v\f\r ]*\n?"#)]
        EndComment,
        #[token("/*")]
        StartComment,
        #[regex("\n")]
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
                    .push(ParseError::NestedBlockComment { span: start..end });
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
                    lex.extras.errors.push(ParseError::UnexpectedEof {
                        expected: "comment terminator".to_owned(),
                    });
                    break lex.remainder().len();
                }
            }
        }
    };

    lex.bump(len);
    result.unwrap_or_default()
}

fn cow_push_str<'a>(cow: &mut Option<Cow<'a, str>>, s: &'a str) {
    match cow {
        Some(cow) => cow.to_mut().push_str(s),
        None => *cow = Some(Cow::Borrowed(s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // use proptest::prelude::*;

    #[test]
    fn simple_tokens() {
        let source = r#"hell0 052 42 0x2A 5. 0.5 0.42e+2 2e-4 .2e+3 true
            false "hello \a\b\f\n\r\t\v\\\'\" \052 \x2a" 'hello 😀' _foo"#;
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next().unwrap(), Token::Ident("hell0".into()));
        assert_eq!(lexer.next().unwrap(), Token::IntLiteral(42));
        assert_eq!(lexer.next().unwrap(), Token::IntLiteral(42));
        assert_eq!(lexer.next().unwrap(), Token::IntLiteral(42));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(5.));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(0.5));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(0.42e+2));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(2e-4));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(0.2e+3));
        assert_eq!(lexer.next().unwrap(), Token::BoolLiteral(true));
        assert_eq!(lexer.next().unwrap(), Token::Newline);
        assert_eq!(lexer.next().unwrap(), Token::BoolLiteral(false));
        assert_eq!(
            lexer.next().unwrap(),
            Token::StringLiteral("hello \x07\x08\x0c\n\r\t\x0b\\'\" * *".into())
        );
        assert_eq!(
            lexer.next().unwrap(),
            Token::StringLiteral("hello 😀".into())
        );
        assert_eq!(lexer.next().unwrap(), Token::Ident("_foo".into()));
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
            vec![ParseError::IntegerOutOfRange {
                span: 0..(source.len() - 2),
            }]
        );
    }

    #[test]
    fn invalid_token() {
        let source = "@ foo";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Error));
        assert_eq!(lexer.next(), Some(Token::Ident("foo".into())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn invalid_string_char() {
        let source = "\"\x00\" foo";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::StringLiteral(Default::default())));
        assert_eq!(lexer.next(), Some(Token::Ident("foo".into())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseError::InvalidStringCharacters { span: 1..2 }]
        );
    }

    #[test]
    fn unterminated_string() {
        let source = "\"hello \n foo";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::StringLiteral("hello ".into())));
        assert_eq!(lexer.next(), Some(Token::Ident("foo".into())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseError::UnterminatedString { span: 7..8 }]
        );
    }

    #[test]
    fn invalid_string_escape() {
        let source = r#""\m" foo"#;
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::StringLiteral("m".into())));
        assert_eq!(lexer.next(), Some(Token::Ident("foo".into())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseError::InvalidStringEscape { span: 1..2 }]
        );
    }

    #[test]
    fn merge_string_errors() {
        let source = "\"\\\x00\" foo";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::StringLiteral("".into())));
        assert_eq!(lexer.next(), Some(Token::Ident("foo".into())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseError::InvalidStringEscape { span: 1..3 }]
        );
    }

    #[test]
    fn line_comment() {
        let source = "foo // bar \n quz";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Ident("foo".into())));
        assert_eq!(lexer.next(), Some(Token::Comment(" bar \n".into())));
        assert_eq!(lexer.next(), Some(Token::Ident("quz".into())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn line_comment_merge() {
        let source = "// merge\n// me\n 5\n // merge\n // me2\n quz // no\n//merge";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Comment(" merge\n me\n".into())));
        assert_eq!(lexer.next(), Some(Token::IntLiteral(5)));
        assert_eq!(lexer.next(), Some(Token::Newline));
        assert_eq!(lexer.next(), Some(Token::Comment(" merge\n me2\n".into())));
        assert_eq!(lexer.next(), Some(Token::Ident("quz".into())));
        assert_eq!(lexer.next(), Some(Token::Comment(" no\n".into())));
        assert_eq!(lexer.next(), Some(Token::Comment("merge".into())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn block_comment() {
        let source = "foo /* bar\n */ quz";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Ident("foo".into())));
        assert_eq!(lexer.next(), Some(Token::Comment(" bar\n".into())));
        assert_eq!(lexer.next(), Some(Token::Ident("quz".into())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn block_comment_multiline() {
        let source = "/* foo\n * bar\n quz*/";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Comment(" foo\n bar\nquz".into())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn nested_block_comment() {
        let source = "foo /* /* bar\n */ */ quz";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Ident("foo".into())));
        assert_eq!(lexer.next(), Some(Token::Comment("  bar\n".into())));
        assert_eq!(lexer.next(), Some(Token::Ident("quz".into())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseError::NestedBlockComment { span: 7..9 }]
        );
    }

    #[test]
    fn nested_block_comment_unterminated() {
        let source = "foo /* /* bar\n */ quz";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Ident("foo".into())));
        assert_eq!(lexer.next(), Some(Token::Comment("  bar\nquz".into())));
        assert_eq!(lexer.next(), Some(Token::Ident("quz".into())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseError::NestedBlockComment { span: 7..9 }]
        );
    }

    #[test]
    fn block_comment_unterminated() {
        let source = "foo /* bar\n quz";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Ident("foo".into())));
        assert_eq!(lexer.next(), Some(Token::Comment(" bar\nquz".into())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseError::UnexpectedEof {
                expected: "comment terminator".to_owned()
            }]
        );
    }

    #[test]
    fn block_comment_trailing_newline() {
        let source = "/* bar */\n";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Comment(" bar ".into())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
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
