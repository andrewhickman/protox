use std::{num::IntErrorKind, convert::TryInto};

use logos::{Lexer, Logos, Filter, Skip};
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
    #[token("//[^\n]*", line_comment)]
    LineComment(String),
    #[token(r#"\*/"#, end_block_comment)]
    EndBlockComment(String),
    #[error]
    #[token(r#"/\*"#, start_block_comment)]
    #[regex(r"[[:space:]]+", logos::skip)]
    Error,
}

pub(crate) struct TokenExtras {
    errors: Vec<LexError>,
    // Stack of block comments
    // (protobuf doesn't support nested block comments, but we track them anyway for better diagnostics)
    block_comments: Vec<usize>,
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
    NestedBlockComment {
        start: SourceOffset,
    },
    UnexpectedEof,
}

fn ident(lex: &mut Lexer<Token>) -> String {
    lex.slice().to_owned()
}

fn int(lex: &mut Lexer<Token>, radix: u32, prefix_len: usize) -> u64 {
    debug_assert!(lex.slice().len() > prefix_len);
    match u64::from_str_radix(&lex.slice()[prefix_len..], radix) {
        Ok(value) => value,
        Err(err) => {
            debug_assert_eq!(err.kind(), &IntErrorKind::PosOverflow);
            lex.extras.errors.push(LexError::IntegerOutOfRange {
                start: (lex.span().start + prefix_len).into(),
                end: lex.span().end.into(),
            });
            // TODO this is a really hacky way to recover from the error, is there a better way?
            Default::default()
        }
    }
}

fn float(lex: &mut Lexer<Token>) -> Result<f64, ()> {
    lex.slice().parse().map_err(drop)
}

fn bool(lex: &mut Lexer<Token>) -> Result<bool, ()> {
    lex.slice().parse().map_err(drop)
}

fn string(lex: &mut Lexer<Token>) -> String {
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
                return result;
            },
            Some(Char::Unescaped(ch) | Char::HexEscape(ch) | Char::OctEscape(ch)) => result.push(ch),
            Some(Char::Error) => {
                // TODO merge similar invalid character spans on adjacent spans
                // TODO this will give an incorrect string to the parser. Does that matter?
                lex.extras.errors.push(LexError::InvalidStringCharacter {
                    start: (lex.span().end + char_lexer.span().start).into(),
                });
                continue;
            }
            None => {
                lex.extras.errors.push(LexError::UnexpectedEof);
                return result;
            }
        }
    }
}

fn line_comment(lex: &mut Lexer<Token>) -> String {
    lex.slice()[2..].to_owned()
}

fn start_block_comment(lex: &mut Lexer<Token>) -> Skip {
    if !lex.extras.block_comments.is_empty() {
        lex.extras.errors.push(LexError::NestedBlockComment { start: lex.span().start.into() });
    }

    lex.extras.block_comments.push(lex.span().end);
    Skip
}

fn end_block_comment(lex: &mut Lexer<Token>) -> Result<String, ()> {
    match lex.extras.block_comments.pop() {
        Some(start) => return Ok(lex.source()[start..lex.span().start].to_owned()),
        None => Err(()),
    }
}
