use logos::{Lexer, Logos, Span};
use miette::{Diagnostic, SourceOffset, SourceSpan};
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
        span: Span,
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
                    "proto2" => { self.bump() ; false} ,
                    "proto3" => { self.bump() ; true },
                    _ => {
                        self.add_error(ParseError::UnknownSyntax { span: span.into() });
                        return Err(());
                    }
                },
                tok => {
                    self.unexpected_token(
                        format!("identifier or '('"),
                        tok,
                    );
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

    fn parse_option(&mut self) -> Result<ast::Option, ()> {
        self.expect_eq(Token::Option)?;

        let namespace = match self.peek() {
            Some((Token::LeftParen, _)) => {
                self.bump();
                let full_ident = self.parse_full_ident(Token::RightParen)?;
                self.expect_eq(Token::RightParen)?;
                full_ident
            }
            Some((Token::Ident(ident), span)) => { self.bump() ; ast::FullIdent::from(ast::Ident {
                value: ident.to_owned(), span: span.clone()
            })},
            tok => {
                self.unexpected_token(
                    format!("identifier or '('"),
                    tok,
                );
                return Err(());
            }
        };

        let mut child: Option<Vec<ast::Ident>> = None;
        loop {
            match self.peek() {
                Some((Token::Dot, _)) => {
                    self.bump();
                }
                Some((Token::Equals, _)) => {
                    self.bump();
                    break;
                }
                tok => {
                    self.unexpected_token(
                        "'.' or '='",
                        tok,
                    );
                    return Err(());
                }
            }

            child.get_or_insert(vec![]).push(self.expect_ident()?);
        }

        let value = match self.peek() {
            Some((Token::Ident(_), _)) => {
                ast::Constant::FullIdent(self.parse_full_ident(Token::Semicolon)?)
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
            Some((Token::String(value), span)) => ast::Constant::String(ast::String {
                value,
                span,
            }),
            Some((Token::Bool(value), span)) => ast::Constant::BoolLiteral(ast::Bool {
                value,
                span,
            }),
            tok => {
                self.unexpected_token("constant", tok);
                return Err(());
            }
        };

        self.expect_eq(Token::Semicolon)?;

        Ok(ast::Option {
            namespace,
            name: child.map(FullIdent::from),
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
            tok => {
                self.unexpected_token("numeric literal", tok);
                return Err(());
            }
        }
    }

    fn parse_full_ident(&mut self, terminator: Token) -> Result<ast::FullIdent, ()> {
        let mut result = vec![self.expect_ident()?];

        loop {
            match self.peek() {
                Some((Token::Dot, _)) => {
                    self.bump();
                }
                Some((tok, _)) if tok == terminator => {
                    return Ok(result.into());
                }
                tok => {
                    self.unexpected_token(
                        format!("'{}' or '{}'", Token::Dot, terminator),
                        tok,
                    );
                    return Err(());
                }
            }

            result.push(self.expect_ident()?);
        }
    }

    fn expect_ident(&mut self) -> Result<ast::Ident, ()> {
        match self.peek() {
            Some((Token::Ident(value), span)) => { self.bump();  Ok(ast::Ident {
                value: value.to_owned(),
                span
            })},
            tok => {
                self.unexpected_token("identifier", tok);
                Err(())
            }
        }
    }

    fn expect_eq(&mut self, t: Token) -> Result<(), ()> {
        match self.peek() {
            Some((tok, _)) if tok == t => { self.bump() ; Ok(()) },
            tok => {
                self.unexpected_token(t, tok);
                Err(())
            }
        }
    }

    fn expect<T>(
        &mut self,
        mut f: impl FnMut(&Token) -> Option<T>,
        expected: impl ToString,
    ) -> Result<T, ()> {
        match self.peek() {
            Some((tok, span)) => match f(&tok) {
                Some(value) => {
                    self.bump();
                    Ok(value)
                }
                None => {
                    self.unexpected_token(expected, Some((tok, span)));
                    Err(())
                }
            },
            tok => {
                self.unexpected_token(expected, tok);
                Err(())
            }
        }
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

    fn unexpected_token(
        &mut self,
        expected: impl ToString,
        found: Option<(Token, Span)>,
    ) {
        match found {
            Some((Token::Error, _)) => {}
            Some((token, span)) => {
                self.add_error(ParseError::UnexpectedToken {
                    expected: expected.to_string(),
                    found: token.clone(),
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
