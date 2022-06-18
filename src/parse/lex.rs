use std::{convert::TryInto, fmt, num::IntErrorKind};

use logos::{Lexer, Logos};
use miette::SourceSpan;

use super::ParseError;

#[derive(Debug, Clone, Logos, PartialEq)]
#[logos(extras = TokenExtras)]
#[logos(subpattern exponent = r"[eE][+\-][0-9]+")]
pub(crate) enum Token {
    #[regex("[A-Za-z][A-Za-z_]*", ident)]
    Ident(String),
    #[regex("0[0-7]*", |lex| int(lex, 8, 1))]
    #[regex("[1-9][0-9]*", |lex| int(lex, 10, 0))]
    #[regex("0[xX][0-9A-Fa-f]+", |lex| int(lex, 16, 2))]
    IntLiteral(u64),
    #[regex(
        r#"([0-9]+\.[0-9]*(?&exponent)?)|([0-9]+(?&exponent))|\.[0-9]+(?&exponent)?"#,
        float
    )]
    FloatLiteral(f64),
    #[regex("false|true", bool)]
    BoolLiteral(bool),
    #[regex(r#"'|""#, string)]
    StringLiteral(String),
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
    #[token(",")]
    Comma,
    #[token("=")]
    Equals,
    #[token(";")]
    Semicolon,
    #[regex("//[^\n]*", line_comment)]
    LineComment(String),
    #[token(r#"/*"#, block_comment)]
    BlockComment(String),
    #[error]
    #[regex(r"[[:space:]]+", logos::skip)]
    Error,
}

impl Token {
    pub fn into_ident(self) -> Option<String> {
        match self {
            Token::Ident(value) => Some(value),
            Token::BoolLiteral(value) => Some(value.to_string()),
            Token::Syntax => Some("syntax".to_owned()),
            Token::Import => Some("import".to_owned()),
            Token::Weak => Some("weak".to_owned()),
            Token::Public => Some("public".to_owned()),
            Token::Package => Some("package".to_owned()),
            Token::Option => Some("option".to_owned()),
            Token::Enum => Some("enum".to_owned()),
            Token::Service => Some("service".to_owned()),
            Token::Rpc => Some("rpc".to_owned()),
            Token::Stream => Some("stream".to_owned()),
            Token::Returns => Some("returns".to_owned()),
            Token::Extend => Some("extend".to_owned()),
            Token::Message => Some("message".to_owned()),
            Token::Optional => Some("optional".to_owned()),
            Token::Required => Some("required".to_owned()),
            Token::Repeated => Some("repeated".to_owned()),
            Token::Map => Some("map".to_owned()),
            Token::Group => Some("group".to_owned()),
            Token::Oneof => Some("oneof".to_owned()),
            Token::Double => Some("double".to_owned()),
            Token::Float => Some("float".to_owned()),
            Token::Int32 => Some("int32".to_owned()),
            Token::Int64 => Some("int64".to_owned()),
            Token::Uint32 => Some("uint32".to_owned()),
            Token::Uint64 => Some("uint64".to_owned()),
            Token::Sint32 => Some("sint32".to_owned()),
            Token::Sint64 => Some("sint64".to_owned()),
            Token::Fixed32 => Some("fixed32".to_owned()),
            Token::Fixed64 => Some("fixed64".to_owned()),
            Token::Sfixed32 => Some("sfixed32".to_owned()),
            Token::Sfixed64 => Some("sfixed64".to_owned()),
            Token::Bool => Some("bool".to_owned()),
            Token::String => Some("string".to_owned()),
            Token::Bytes => Some("bytes".to_owned()),
            Token::Reserved => Some("reserved".to_owned()),
            Token::Extensions => Some("extensions".to_owned()),
            Token::To => Some("to".to_owned()),
            Token::Max => Some("max".to_owned()),
            _ => None,
        }
    }
}

impl fmt::Display for Token {
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
            Token::Comma => write!(f, ","),
            Token::Plus => write!(f, "+"),
            Token::Equals => write!(f, "="),
            Token::Semicolon => write!(f, ";"),
            Token::LineComment(value) => writeln!(f, "// {}", value),
            Token::BlockComment(value) => write!(f, "/* {} */", value),
            Token::Error => write!(f, "<ERROR>"),
        }
    }
}

#[derive(Default)]
pub(crate) struct TokenExtras {
    pub errors: Vec<ParseError>,
}

fn ident(lex: &mut Lexer<Token>) -> String {
    lex.slice().to_owned()
}

fn int(lex: &mut Lexer<Token>, radix: u32, prefix_len: usize) -> u64 {
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
            lex.extras.errors.push(ParseError::IntegerOutOfRange {
                span: (start..end).into(),
            });
            // TODO this is a really hacky way to recover from the error, is there a better way?
            Default::default()
        }
    }
}

fn float(lex: &mut Lexer<Token>) -> f64 {
    lex.slice().parse().expect("failed to parse float")
}

fn bool(lex: &mut Lexer<Token>) -> bool {
    lex.slice().parse().expect("faield to parse bool")
}

fn string(lex: &mut Lexer<Token>) -> String {
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

    let mut result = String::new();

    let mut char_lexer = Component::lexer(lex.remainder());
    let terminator = lex.slice().chars().next().expect("expected char");

    loop {
        match char_lexer.next() {
            Some(Component::Unescaped(s)) => {
                result.push_str(s);
            }
            Some(Component::Terminator(t)) if t == terminator => {
                break;
            }
            Some(Component::Terminator(ch) | Component::Char(ch)) => result.push(ch),
            Some(Component::Error) => {
                let start = lex.span().end + char_lexer.span().start;
                let end = lex.span().end + char_lexer.span().end;
                let span = SourceSpan::from(start..end);

                if char_lexer.slice().contains('\n') {
                    lex.extras
                        .errors
                        .push(ParseError::UnterminatedString { span });
                    break;
                } else if let Some(err) = lex.extras.errors.last_mut() {
                    match err {
                        ParseError::InvalidStringCharacters { span: err_span }
                        | ParseError::InvalidStringEscape { span: err_span } => {
                            // If the last character was invalid, extend the span of its error
                            // instead of adding a new error.
                            if (err_span.offset() + err_span.len()) == start {
                                *err_span = SourceSpan::from(err_span.offset()..end);
                                continue;
                            }
                        }
                        _ => (),
                    }
                }

                if char_lexer.slice().starts_with('\\') {
                    lex.extras
                        .errors
                        .push(ParseError::InvalidStringEscape { span });
                    continue;
                } else {
                    lex.extras
                        .errors
                        .push(ParseError::InvalidStringCharacters { span });
                    continue;
                }
            }
            None => {
                lex.extras
                    .errors
                    .push(ParseError::UnexpectedEof { expected: None });
                break;
            }
        }
    }

    lex.bump(char_lexer.span().end);
    result
}

fn line_comment(lex: &mut Lexer<Token>) -> String {
    lex.slice()[2..].trim().to_owned()
}

fn block_comment(lex: &mut Lexer<Token>) -> Result<String, ()> {
    #[derive(Logos)]
    enum Component {
        #[token("*/")]
        EndComment,
        #[token("/*")]
        StartComment,
        #[error]
        Text,
    }

    let mut comment_lexer = Component::lexer(lex.remainder()).spanned();

    let mut depth = 1u32;
    let mut last_end = None;
    let len = loop {
        match comment_lexer.next() {
            Some((Component::EndComment, span)) => {
                depth -= 1;
                if depth == 0 {
                    break span.end;
                } else {
                    last_end = Some(span.end);
                }
            }
            Some((Component::StartComment, span)) => {
                let start = lex.span().end + span.start;
                let end = lex.span().end + span.end;
                lex.extras.errors.push(ParseError::NestedBlockComment {
                    span: SourceSpan::from(start..end),
                });
                depth += 1;
            }
            Some((Component::Text, _)) => continue,
            None => {
                if let Some(last_end) = last_end {
                    // This must be a nested block comment
                    break last_end;
                } else {
                    lex.extras
                        .errors
                        .push(ParseError::UnexpectedEof { expected: None });
                    break lex.remainder().len();
                }
            }
        }
    };

    lex.bump(len);
    return Ok(lex.slice()[2..][..len]
        .trim_end_matches("*/")
        .trim()
        .to_owned());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_tokens() {
        let source = r#"hello 052 42 0x2A 5. 0.5 0.42e+2 2e-4 .2e+3 true false "hello \a\b\f\n\r\t\v\\\'\" \052 \x2a" 'hello 😀'"#;
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next().unwrap(), Token::Ident("hello".to_owned()));
        assert_eq!(lexer.next().unwrap(), Token::IntLiteral(42));
        assert_eq!(lexer.next().unwrap(), Token::IntLiteral(42));
        assert_eq!(lexer.next().unwrap(), Token::IntLiteral(42));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(5.));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(0.5));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(0.42e+2));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(2e-4));
        assert_eq!(lexer.next().unwrap(), Token::FloatLiteral(0.2e+3));
        assert_eq!(lexer.next().unwrap(), Token::BoolLiteral(true));
        assert_eq!(lexer.next().unwrap(), Token::BoolLiteral(false));
        assert_eq!(
            lexer.next().unwrap(),
            Token::StringLiteral("hello \x07\x08\x0c\n\r\t\x0b\\'\" * *".to_owned())
        );
        assert_eq!(
            lexer.next().unwrap(),
            Token::StringLiteral("hello 😀".to_owned())
        );
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
                span: SourceSpan::from((0, source.len() - 2)),
            }]
        );
    }

    #[test]
    fn invalid_token() {
        let source = "@ foo";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Error));
        assert_eq!(lexer.next(), Some(Token::Ident("foo".to_owned())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn invalid_string_char() {
        let source = "\"\x00\" foo";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::StringLiteral(String::new())));
        assert_eq!(lexer.next(), Some(Token::Ident("foo".to_owned())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseError::InvalidStringCharacters {
                span: SourceSpan::from((1, 1)),
            }]
        );
    }

    #[test]
    fn unterminated_string() {
        let source = "\"hello \n foo";
        let mut lexer = Token::lexer(source);

        assert_eq!(
            lexer.next(),
            Some(Token::StringLiteral("hello ".to_owned()))
        );
        assert_eq!(lexer.next(), Some(Token::Ident("foo".to_owned())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseError::UnterminatedString {
                span: SourceSpan::from((7, 1))
            }]
        );
    }

    #[test]
    fn invalid_string_escape() {
        let source = r#""\m" foo"#;
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::StringLiteral("m".to_owned())));
        assert_eq!(lexer.next(), Some(Token::Ident("foo".to_owned())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseError::InvalidStringEscape {
                span: SourceSpan::from((1, 1))
            }]
        );
    }

    #[test]
    fn merge_string_errors() {
        let source = "\"\\\x00\" foo";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::StringLiteral("".to_owned())));
        assert_eq!(lexer.next(), Some(Token::Ident("foo".to_owned())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseError::InvalidStringEscape {
                span: SourceSpan::from((1, 2))
            }]
        );
    }

    #[test]
    fn line_comment() {
        let source = "foo // bar \n quz";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Ident("foo".to_owned())));
        assert_eq!(lexer.next(), Some(Token::LineComment("bar".to_owned())));
        assert_eq!(lexer.next(), Some(Token::Ident("quz".to_owned())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn block_comment() {
        let source = "foo /* bar\n */ quz";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Ident("foo".to_owned())));
        assert_eq!(lexer.next(), Some(Token::BlockComment("bar".to_owned())));
        assert_eq!(lexer.next(), Some(Token::Ident("quz".to_owned())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(lexer.extras.errors, vec![]);
    }

    #[test]
    fn nested_block_comment() {
        let source = "foo /* /* bar\n */ */ quz";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Ident("foo".to_owned())));
        assert_eq!(
            lexer.next(),
            Some(Token::BlockComment("/* bar\n */".to_owned()))
        );
        assert_eq!(lexer.next(), Some(Token::Ident("quz".to_owned())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseError::NestedBlockComment {
                span: SourceSpan::from((7, 2))
            }]
        );
    }

    #[test]
    fn nested_block_comment_unterminated() {
        let source = "foo /* /* bar\n */ quz";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Ident("foo".to_owned())));
        assert_eq!(lexer.next(), Some(Token::BlockComment("/* bar".to_owned())));
        assert_eq!(lexer.next(), Some(Token::Ident("quz".to_owned())));
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseError::NestedBlockComment {
                span: SourceSpan::from((7, 2))
            }]
        );
    }

    #[test]
    fn block_comment_unterminated() {
        let source = "foo /* bar\n quz";
        let mut lexer = Token::lexer(source);

        assert_eq!(lexer.next(), Some(Token::Ident("foo".to_owned())));
        assert_eq!(
            lexer.next(),
            Some(Token::BlockComment("bar\n quz".to_owned()))
        );
        assert_eq!(lexer.next(), None);

        debug_assert_eq!(
            lexer.extras.errors,
            vec![ParseError::UnexpectedEof { expected: None }]
        );
    }
}
