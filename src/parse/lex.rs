use std::{num::IntErrorKind, convert::TryInto};

use logos::{Lexer, Logos};
use miette::SourceOffset;

#[derive(Debug, Clone, Logos)]
#[logos(extras = TokenExtras)]
pub(crate) enum Token {
    #[regex("[A-Za-z][A-Za-z_]*", ident)]
    Ident(String),
    #[regex("0[0-7]*", |lex| int(lex, 8, 1))]
    Octal(u64),
    #[regex("[1-9][0-9]*", |lex| int(lex, 10, 0))]
    Decimal(u64),
    #[regex("0[xX][0-9A-Fa-f]+", |lex| int(lex, 16, 2))]
    Hexadecimal(u64),
    #[regex(r#"([0-9]+\.[0-9]*([eE][+\-][0-9]+)?)|([0-9]+[eE][+\-][0-9]+)|\.[0-9]+([eE][+\-][0-9]+)?|inf|nan"#, float)]
    Float(f64),
    #[regex("false|true", bool)]
    Bool(bool),
    #[regex(r#"'|""#, string)]
    String(String),
    #[token(".")]
    Dot,
    #[token("-")]
    Minus,
    #[token("+")]
    Plus,
    #[error]
    #[regex(r"[[:space:]]+", logos::skip)]
    Error,
}

pub(crate) struct TokenExtras {
    errors: Vec<LexError>,
}

enum LexError {
    UnexpectedToken {
        start: SourceOffset,
    },
    IntegerOutOfRange {
        start: SourceOffset,
        end: SourceOffset,
    },
    InvalidStringCharacter {
        start: SourceOffset,
    },
    UnexpectedEof,
}

fn ident(lex: &mut Lexer<Token>) -> String {
    lex.slice().to_owned()
}

fn int(lex: &mut Lexer<Token>, radix: u32, prefix_len: usize) -> Result<u64, ()> {
    debug_assert!(lex.slice().len() > prefix_len);
    match u64::from_str_radix(&lex.slice()[prefix_len..], radix) {
        Ok(value) => Ok(value),
        Err(err) => {
            debug_assert_eq!(err.kind(), &IntErrorKind::PosOverflow);
            lex.extras.errors.push(LexError::IntegerOutOfRange {
                start: (lex.span().start + prefix_len).into(),
                end: lex.span().end.into(),
            });
            Err(())
        }
    }
}

fn float(lex: &mut Lexer<Token>) -> Result<f64, ()> {
    lex.slice().parse().map_err(drop)
}

fn bool(lex: &mut Lexer<Token>) -> Result<bool, ()> {
    lex.slice().parse().map_err(drop)
}

fn string(lex: &mut Lexer<Token>) -> Result<String, ()> {
    #[derive(Logos)]
    enum Char {
        #[regex(r#"[^\x00\n\\]"#, unescaped)]
        Unescaped(char),
        #[regex(r#"\[xX][0-9A-Fa-f][0-9A-Fa-f]"#, hex_escape)]
        HexEscape(char),
        #[regex(r#"\[0-7][0-7][0-7]"#, oct_escape)]
        OctEscape(char),
        #[error]
        Error,
    }

    fn unescaped(lex: &mut Lexer<Char>) -> char {
        debug_assert_eq!(lex.slice().chars().count(), 1);
        lex.slice().chars().next().expect("expected char")
    }

    fn hex_escape(lex: &mut Lexer<Char>) -> char {
        u32::from_str_radix(&lex.slice()[2..], 16)
            .expect("expected valid hex escape")
            .try_into()
            .expect("two-digit hex escape should be valid char")
    }

    fn oct_escape(lex: &mut Lexer<Char>) -> char {
        u32::from_str_radix(&lex.slice()[1..], 8)
            .expect("expected valid oct escape")
            .try_into()
            .expect("three-digit oct escape should be valid char")
    }

    let mut result = String::new();

    let mut char_lexer = Char::lexer(lex.remainder());
    let terminator = lex.slice().chars().next().expect("expected char");
    loop {
        match char_lexer.next() {
            Some(Char::Unescaped(ch)) if ch == terminator => {
                lex.bump(char_lexer.span().end);
                return Ok(result)
            },
            Some(Char::Unescaped(ch) | Char::HexEscape(ch) | Char::OctEscape(ch)) => result.push(ch),
            Some(Char::Error) => {
                lex.extras.errors.push(LexError::InvalidStringCharacter {
                    start: (lex.span().end + char_lexer.span().start).into(),
                });
                return Err(())
            }
            None => {
                lex.extras.errors.push(LexError::UnexpectedEof);
                return Err(())
            }
        }
    }
}