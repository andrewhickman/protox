use logos::{Lexer, Logos, Span};
use miette::{Diagnostic, SourceOffset, SourceSpan};
use thiserror::Error;

mod lex;
mod proto2;
mod proto3;

use self::lex::Token;
use crate::ast;

#[derive(Error, Debug, Diagnostic)]
#[error("error parsing file")]
#[diagnostic()]
pub enum ParseError {
    InvalidToken {
        span: SourceSpan,
    },
    IntegerOutOfRange {
        span: SourceSpan,
    },
    InvalidStringCharacters {
        span: SourceSpan,
    },
    UnterminatedString {
        span: SourceSpan,
    },
    InvalidStringEscape {
        span: SourceSpan,
    },
    NestedBlockComment {
        span: SourceSpan,
    },
    UnknownSyntax {
        span: SourceSpan,
    },
    UnexpectedToken {
        expected: String,
        found: Token,
        span: Span,
    },
    UnexpectedEof {
        expected: Option<String>,
    },
}

pub fn parse(source: &str) -> Result<ast::File, Vec<ParseError>> {
    let mut parser = Parser::new(source);
    match parser.parse_file() {
        Ok(file) => Ok(file),
        Err(()) => Err(parser.lexer.extras.errors),
    }
}

struct Parser<'a> {
    lexer: Lexer<'a, Token>,
    peek: Option<(Token, Span)>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Parser {
            lexer: Token::lexer(source),
            peek: None,
        }
    }

    fn parse_file(&mut self) -> Result<ast::File, ()> {
        self.skip_comments();
        let proto3 = if self.bump_if_eq(Token::Syntax) {
            match self.next() {
                Some((Token::String(syntax), span)) => match &*syntax {
                    "proto2" => false,
                    "proto3" => true,
                    _ => {
                        self.add_error(ParseError::UnknownSyntax { span: span.into() });
                        return Err(());
                    }
                },
                tok => {
                    self.unexpected_token("'proto2' or 'proto3'", tok);
                    return Err(());
                }
            }
        } else {
            false
        };

        let file = if proto3 {
            ast::File::Proto3(self.parse_proto3_file()?)
        } else {
            ast::File::Proto2(self.parse_proto2_file()?)
        };

        Ok(file)
    }

    fn skip_comments(&mut self) {
        while self.bump_if(|tok| match tok {
            Token::LineComment(_) | Token::BlockComment(_) => true,
            _ => false,
        }) {}
    }

    fn bump_if_eq(&mut self, t: Token) -> bool {
        self.bump_if(|tok| *tok == t)
    }

    fn bump_if(&mut self, f: impl FnMut(&Token) -> bool) -> bool {
        self.next_if(f).is_some()
    }

    fn next_if(&mut self, f: impl FnMut(&Token) -> bool) -> Option<(Token, Span)> {
        match self.peek() {
            Some((tok, _)) if f(tok) => Some(self.bump()),
            _ => None,
        }
    }

    fn bump(&mut self) -> (Token, Span) {
        self.peek
            .take()
            .expect("called bump without peek returning Some()")
    }

    fn peek(&mut self) -> Option<&(Token, Span)> {
        if self.peek.is_none() {
            self.peek = self.next();
        }
        self.peek.as_ref()
    }

    fn next(&mut self) -> Option<(Token, Span)> {
        if self.peek.is_some() {
            self.peek.take()
        } else {
            match self.lexer.next() {
                Some(Token::Error) => {
                    self.add_error(ParseError::InvalidToken {
                        span: self.lexer.span().into(),
                    });
                    Some((Token::Error, self.lexer.span()))
                }
                Some(tok) => Some((tok, self.lexer.span())),
                None => None,
            }
        }
    }

    fn unexpected_token(
        &mut self,
        expected: impl ToString,
        found: impl Into<Option<(Token, Span)>>,
    ) {
        match found.into() {
            Some((token, span)) => {
                self.add_error(ParseError::UnexpectedToken {
                    expected: expected.to_string(),
                    found: token,
                    span: span.into(),
                });
            }
            None => self.eof(Some(expected)),
        }
    }

    fn eof(&mut self, expected: Option<impl ToString>) {
        self.add_error(ParseError::UnexpectedEof {
            expected: expected.map(|s| s.to_string()),
        });
    }

    fn add_error(&mut self, err: ParseError) {
        self.lexer.extras.errors.push(err);
    }
}
