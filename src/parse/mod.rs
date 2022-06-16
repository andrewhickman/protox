use std::{fmt::Write, iter::once};

use logos::{Lexer, Logos, Span};
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

mod lex;
mod proto2;
mod proto3;

use self::lex::Token;
use crate::ast::{self, FullIdent};

#[derive(Error, Debug, Diagnostic, PartialEq)]
#[error("error parsing file")]
#[diagnostic()]
pub(crate) enum ParseError {
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
        span: SourceSpan,
    },
    UnexpectedEof {
        expected: Option<String>,
    },
}

pub(crate) fn parse(source: &str) -> Result<ast::File, Vec<ParseError>> {
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
            match self.peek() {
                Some((Token::String(syntax), span)) => match &*syntax {
                    "proto2" => {
                        self.bump();
                        false
                    }
                    "proto3" => {
                        self.bump();
                        true
                    }
                    _ => {
                        self.add_error(ParseError::UnknownSyntax { span: span.into() });
                        return Err(());
                    }
                },
                tok => {
                    self.unexpected_token("identifier or '('");
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

    fn parse_service(&mut self) -> Result<ast::Service, ()> {
        todo!()
    }

    fn parse_enum(&mut self) -> Result<ast::Enum, ()> {
        todo!()
    }

    fn parse_option_expr(&mut self) -> Result<ast::Option, ()> {
        self.expect_eq(Token::Option)?;

        let option = self.parse_option(&[Token::Semicolon])?;

        self.expect_eq(Token::Semicolon)?;

        Ok(option)
    }

    fn parse_option(&mut self, terminators: &[Token]) -> Result<ast::Option, ()> {
        let name = match self.peek() {
            Some((Token::LeftParen, _)) => {
                self.bump();
                let full_ident = self.parse_full_ident(&[Token::RightParen])?;
                self.expect_eq(Token::RightParen)?;
                full_ident
            }
            Some((Token::Ident(value), span)) => {
                self.bump();
                ast::FullIdent::from(ast::Ident { value, span })
            }
            _ => {
                self.unexpected_token("identifier or '('");
                return Err(());
            }
        };

        let mut field_name: Option<Vec<ast::Ident>> = None;
        loop {
            match self.peek() {
                Some((Token::Dot, _)) => {
                    self.bump();
                }
                Some((Token::Equals, _)) => {
                    self.bump();
                    break;
                }
                _ => {
                    self.unexpected_token("'.' or '='");
                    return Err(());
                }
            }

            field_name.get_or_insert(vec![]).push(self.expect_ident()?);
        }

        let value = match self.peek() {
            Some((Token::Ident(_), _)) => {
                ast::Constant::FullIdent(self.parse_full_ident(terminators)?)
            }
            Some((Token::Plus, _)) => {
                self.bump();
                self.parse_int_or_float(false)?
            }
            Some((Token::Minus, _)) => {
                self.bump();
                self.parse_int_or_float(true)?
            }
            Some((Token::Int(_) | Token::Float(_), _)) => self.parse_int_or_float(false)?,
            Some((Token::String(value), span)) => {
                self.bump();
                ast::Constant::String(ast::String { value, span })
            }
            Some((Token::Bool(value), span)) => {
                self.bump();
                ast::Constant::Bool(ast::Bool { value, span })
            }
            _ => {
                self.unexpected_token("constant");
                return Err(());
            }
        };

        Ok(ast::Option {
            name,
            field_name: field_name.map(FullIdent::from),
            value,
        })
    }

    fn parse_int_or_float(&mut self, negate: bool) -> Result<ast::Constant, ()> {
        match self.peek() {
            Some((Token::Int(value), span)) => {
                self.bump();
                Ok(ast::Constant::Int(ast::Int {
                    value,
                    span,
                    negative: negate,
                }))
            }
            Some((Token::Float(value), span)) => {
                self.bump();
                Ok(ast::Constant::Float(ast::Float {
                    value: if negate { -value } else { value },
                    span,
                }))
            }
            _ => {
                self.unexpected_token("numeric literal");
                Err(())
            }
        }
    }

    fn parse_full_ident(&mut self, terminators: &[Token]) -> Result<ast::FullIdent, ()> {
        let mut result = vec![self.expect_ident()?];

        loop {
            match self.peek() {
                Some((Token::Dot, _)) => {
                    self.bump();
                }
                Some((tok, _)) if terminators.contains(&tok) => {
                    return Ok(result.into());
                }
                _ => {
                    self.unexpected_token(fmt_expected(
                        once(Token::Dot).chain(terminators.iter().cloned()),
                    ));
                    return Err(());
                }
            }

            result.push(self.expect_ident()?);
        }
    }

    fn expect_ident(&mut self) -> Result<ast::Ident, ()> {
        self.expect(
            |tok, span| tok.into_ident().map(|value| ast::Ident::new(value, span)),
            "identifier",
        )
    }

    fn expect_eq(&mut self, t: Token) -> Result<(), ()> {
        match self.peek() {
            Some((tok, _)) if tok == t => {
                self.bump();
                Ok(())
            }
            _ => {
                self.unexpected_token(t);
                Err(())
            }
        }
    }

    fn expect<T>(
        &mut self,
        mut f: impl FnMut(Token, Span) -> Option<T>,
        expected: impl ToString,
    ) -> Result<T, ()> {
        if let Some((tok, span)) = self.peek() {
            if let Some(value) = f(tok, span) {
                self.bump();
                return Ok(value);
            }
        };

        self.unexpected_token(expected);
        Err(())
    }

    fn skip_comments(&mut self) {
        while self.bump_if(|tok| matches!(tok, Token::LineComment(_) | Token::BlockComment(_))) {}
    }

    fn bump_if_eq(&mut self, t: Token) -> bool {
        self.bump_if(|tok| *tok == t)
    }

    fn bump_if(&mut self, f: impl FnMut(&Token) -> bool) -> bool {
        self.next_if(f).is_some()
    }

    fn next_if(&mut self, mut f: impl FnMut(&Token) -> bool) -> Option<(Token, Span)> {
        match self.peek() {
            Some((tok, _)) if f(&tok) => Some(self.bump()),
            _ => None,
        }
    }

    fn bump(&mut self) -> (Token, Span) {
        self.peek
            .take()
            .expect("called bump without peek returning Some()")
    }

    fn peek(&mut self) -> Option<(Token, Span)> {
        if self.peek.is_none() {
            self.peek = self.next();
        }
        self.peek.clone()
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

    fn unexpected_token(&mut self, expected: impl ToString) {
        match self.peek() {
            Some((Token::Error, _)) => {}
            Some((found, span)) => {
                self.add_error(ParseError::UnexpectedToken {
                    expected: expected.to_string(),
                    found,
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

fn fmt_expected(mut ts: impl Iterator<Item = Token>) -> String {
    let ts: Vec<_> = ts.collect();

    let mut s = String::with_capacity(32);
    write!(s, "'{}'", ts[0]).unwrap();
    if ts.len() > 1 {
        for t in &ts[0..][..ts.len() - 1] {
            write!(s, ", '{}'", ts[0]).unwrap();
        }
        write!(s, "or '{}'", ts[ts.len() - 1]).unwrap();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! case {
        ($method:ident($source:expr) => Err($errors:expr)) => {
            let mut parser = Parser::new($source);
            parser.$method().unwrap_err();
            assert_eq!(parser.lexer.extras.errors, $errors);
        };
        ($method:ident($source:expr) => $ast:expr) => {
            let mut parser = Parser::new($source);
            let result = parser.$method();
            assert_eq!(parser.lexer.extras.errors, vec![]);
            assert_eq!(result.unwrap(), $ast);
        };
    }

    #[test]
    pub fn parse_option() {
        case!(parse_option_expr("option foo = 5;") => ast::Option {
            name: ast::FullIdent::from(ast::Ident::new("foo", 7..10)),
            field_name: None,
            value: ast::Constant::Int(ast::Int {
                negative: false,
                value: 5,
                span: 13..14,
            }),
        });
        case!(parse_option_expr("option (foo.bar) = \"hello\";") => ast::Option {
            name: ast::FullIdent::from(vec![
                ast::Ident::new("foo", 8..11),
                ast::Ident::new("bar", 12..15),
            ]),
            field_name: None,
            value: ast::Constant::String(ast::String {
                value: "hello".to_string(),
                span: 19..26,
            }),
        });
        case!(parse_option_expr("option (foo).bar = true;") => ast::Option {
            name: ast::FullIdent::from(ast::Ident::new("foo", 8..11)),
            field_name: Some(ast::FullIdent::from(ast::Ident::new("bar", 13..16))),
            value: ast::Constant::Bool(ast::Bool {
                value: true,
                span: 19..23,
            }),
        });
        case!(parse_option_expr("option ;") => Err(vec![ParseError::UnexpectedToken {
            expected: "identifier or '('".to_owned(),
            found: Token::Semicolon,
            span: SourceSpan::from(7..8),
        }]));
    }
}
