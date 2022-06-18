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
                _ => self.unexpected_token("an identifier or '('")?,
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
        self.expect_eq(Token::Service)?;

        let name = self.expect_ident()?;

        self.expect_eq(Token::LeftBrace)?;

        let mut options = Vec::new();
        let mut methods = Vec::new();

        loop {
            match self.peek() {
                Some((Token::Rpc, _)) => {
                    methods.push(self.parse_service_rpc()?);
                }
                Some((Token::Option, _)) => {
                    options.push(self.parse_option()?);
                }
                Some((Token::Semicolon, _)) => {
                    self.bump();
                    continue;
                }
                Some((Token::RightBrace, _)) => {
                    self.bump();
                    break;
                }
                _ => self.unexpected_token("'rpc', '}', 'option' or ';'")?,
            }
        }

        Ok(ast::Service {
            name,
            methods,
            options,
        })
    }

    fn parse_service_rpc(&mut self) -> Result<ast::Method, ()> {
        self.expect_eq(Token::Rpc)?;

        let name = self.expect_ident()?;

        self.expect_eq(Token::LeftParen)?;

        let is_client_streaming = match self.peek() {
            Some((Token::Stream, _)) => {
                self.bump();
                true
            }
            Some((Token::Dot | Token::Ident(_), _)) => false,
            _ => self.unexpected_token("'stream' or a type name")?,
        };

        let input_ty = self.parse_type_name(&[Token::RightParen])?;

        self.expect_eq(Token::RightParen)?;
        self.expect_eq(Token::Returns)?;
        self.expect_eq(Token::LeftParen)?;

        let is_server_streaming = match self.peek() {
            Some((Token::Stream, _)) => {
                self.bump();
                true
            }
            Some((Token::Dot | Token::Ident(_), _)) => false,
            _ => self.unexpected_token("'stream' or a type name")?,
        };

        let output_ty = self.parse_type_name(&[Token::RightParen])?;

        self.expect_eq(Token::RightParen)?;

        let mut options = Vec::new();
        match self.peek() {
            Some((Token::Semicolon, _)) => {
                self.bump();
            }
            Some((Token::LeftBrace, _)) => {
                self.bump();
                loop {
                    match self.peek() {
                        Some((Token::Option, _)) => {
                            options.push(self.parse_option()?);
                        }
                        Some((Token::RightBrace, _)) => {
                            self.bump();
                            break;
                        }
                        Some((Token::Semicolon, _)) => {
                            self.bump();
                            continue;
                        }
                        _ => self.unexpected_token("'option', '}' or ';'")?,
                    }
                }
            }
            _ => self.unexpected_token("';' or '{'")?,
        }

        Ok(ast::Method {
            name,
            input_ty,
            is_client_streaming,
            output_ty,
            is_server_streaming,
            options,
        })
    }

    fn parse_enum(&mut self) -> Result<ast::Enum, ()> {
        self.expect_eq(Token::Enum)?;

        let name = self.expect_ident()?;

        self.expect_eq(Token::LeftBrace)?;

        let mut values = Vec::new();
        let mut options = Vec::new();

        loop {
            match self.peek() {
                Some((Token::Option, _)) => {
                    options.push(self.parse_option()?);
                }
                Some((Token::Semicolon, _)) => {
                    self.bump();
                }
                Some((Token::Ident(_), _)) => {
                    values.push(self.parse_enum_value()?);
                }
                Some((Token::RightBrace, _)) => break,
                _ => self.unexpected_token("an identifier, '}', ';' or 'option'")?,
            };
        }

        Ok(ast::Enum {
            name,
            options,
            values,
        })
    }

    fn parse_enum_value(&mut self) -> Result<ast::EnumValue, ()> {
        let name = self.expect_ident()?;

        self.expect_eq(Token::Equals)?;

        let negative = self.bump_if_eq(Token::Minus);
        let value = match self.peek() {
            Some((Token::Int(value), span)) => {
                self.bump();
                ast::Int {
                    negative,
                    value,
                    span,
                }
            }
            _ => self.unexpected_token("an integer")?,
        };

        let options = match self.peek() {
            Some((Token::Semicolon, _)) => vec![],
            Some((Token::LeftBracket, _)) => self.parse_options_list()?,
            _ => self.unexpected_token("';' or '['")?,
        };

        self.expect_eq(Token::Semicolon)?;
        Ok(ast::EnumValue {
            name,
            value,
            options,
        })
    }

    fn parse_options_list(&mut self) -> Result<Vec<ast::Option>, ()> {
        self.expect_eq(Token::LeftBracket)?;

        let mut options = vec![self.parse_option_body(&[Token::Comma, Token::RightBracket])?];
        loop {
            match self.peek() {
                Some((Token::Comma, _)) => {
                    options.push(self.parse_option_body(&[Token::Comma, Token::RightBracket])?);
                }
                Some((Token::RightBracket, _)) => {
                    self.bump();
                    break;
                }
                _ => self.unexpected_token("',' or ']'")?,
            }
        }

        Ok(options)
    }

    fn parse_option(&mut self) -> Result<ast::Option, ()> {
        self.expect_eq(Token::Option)?;

        let option = self.parse_option_body(&[Token::Semicolon])?;

        self.expect_eq(Token::Semicolon)?;

        Ok(option)
    }

    fn parse_option_body(&mut self, terminators: &[Token]) -> Result<ast::Option, ()> {
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
            _ => self.unexpected_token("an identifier or '('")?,
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
                _ => self.unexpected_token("'.' or '='")?,
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
            _ => self.unexpected_token("a constant")?,
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
            _ => self.unexpected_token("numeric literal")?,
        }
    }

    fn parse_type_name(&mut self, terminators: &[Token]) -> Result<ast::TypeName, ()> {
        let leading_dot = match self.peek() {
            Some((Token::Dot, span)) => {
                self.bump();
                Some(span)
            }
            Some((Token::Ident(_), _)) => None,
            _ => self.unexpected_token("a type name")?,
        };

        let name = self.parse_full_ident(terminators)?;

        Ok(ast::TypeName { name, leading_dot })
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
                _ => self.unexpected_token(fmt_expected(
                    once(Token::Dot).chain(terminators.iter().cloned()),
                ))?,
            }

            result.push(self.expect_ident()?);
        }
    }

    fn expect_ident(&mut self) -> Result<ast::Ident, ()> {
        self.expect(
            |tok, span| tok.into_ident().map(|value| ast::Ident::new(value, span)),
            "an identifier",
        )
    }

    fn expect_eq(&mut self, t: Token) -> Result<(), ()> {
        match self.peek() {
            Some((tok, _)) if tok == t => {
                self.bump();
                Ok(())
            }
            _ => self.unexpected_token(format!("'{}'", t))?,
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

        self.unexpected_token(expected)?
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

    fn unexpected_token<T>(&mut self, expected: impl ToString) -> Result<T, ()> {
        match self.peek() {
            Some((Token::Error, _)) => Err(()),
            Some((found, span)) => {
                self.add_error(ParseError::UnexpectedToken {
                    expected: expected.to_string(),
                    found,
                    span: span.into(),
                });
                Err(())
            }
            None => {
                self.eof(Some(expected));
                Err(())
            }
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

fn fmt_expected(ts: impl Iterator<Item = Token>) -> String {
    let ts: Vec<_> = ts.collect();

    let mut s = String::with_capacity(32);
    write!(s, "'{}'", ts[0]).unwrap();
    if ts.len() > 1 {
        for t in &ts[1..][..ts.len() - 2] {
            write!(s, ", '{}'", t).unwrap();
        }
        write!(s, " or '{}'", ts[ts.len() - 1]).unwrap();
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
        case!(parse_option("option foo = 5;") => ast::Option {
            name: ast::FullIdent::from(ast::Ident::new("foo", 7..10)),
            field_name: None,
            value: ast::Constant::Int(ast::Int {
                negative: false,
                value: 5,
                span: 13..14,
            }),
        });
        case!(parse_option("option (foo.bar) = \"hello\";") => ast::Option {
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
        case!(parse_option("option (foo).bar = true;") => ast::Option {
            name: ast::FullIdent::from(ast::Ident::new("foo", 8..11)),
            field_name: Some(ast::FullIdent::from(ast::Ident::new("bar", 13..16))),
            value: ast::Constant::Bool(ast::Bool {
                value: true,
                span: 19..23,
            }),
        });
        case!(parse_option("option ;") => Err(vec![ParseError::UnexpectedToken {
            expected: "an identifier or '('".to_owned(),
            found: Token::Semicolon,
            span: SourceSpan::from(7..8),
        }]));
        case!(parse_option("option foo (") => Err(vec![ParseError::UnexpectedToken {
            expected: "'.' or '='".to_owned(),
            found: Token::LeftParen,
            span: SourceSpan::from(11..12),
        }]));
        case!(parse_option("option foo.]") => Err(vec![ParseError::UnexpectedToken {
            expected: "an identifier".to_owned(),
            found: Token::RightBracket,
            span: SourceSpan::from(11..12),
        }]));
        case!(parse_option("option foo = =") => Err(vec![ParseError::UnexpectedToken {
            expected: "a constant".to_owned(),
            found: Token::Equals,
            span: SourceSpan::from(13..14),
        }]));
        case!(parse_option("option foo = 1 )") => Err(vec![ParseError::UnexpectedToken {
            expected: "';'".to_owned(),
            found: Token::RightParen,
            span: SourceSpan::from(15..16),
        }]));
    }

    #[test]
    fn parse_enum() {
        case!(parse_enum("enum Foo {}") => ast::Enum {
            name: ast::Ident::new("Foo", 5..8),
            values: vec![],
            options: vec![],
        });
        case!(parse_enum("enum Foo { ; ; }") => ast::Enum {
            name: ast::Ident::new("Foo", 5..8),
            values: vec![],
            options: vec![],
        });
        case!(parse_enum("enum Foo { BAR = 1; }") => ast::Enum {
            name: ast::Ident::new("Foo", 5..8),
            values: vec![ast::EnumValue {
                name: ast::Ident::new("BAR", 11..14),
                value: ast::Int {
                    negative: false,
                    value: 1,
                    span: 17..18,
                },
                options: vec![],
            }],
            options: vec![],
        });
        case!(parse_enum("enum Foo { option bar = 'quz' ; VAL = -1; }") => ast::Enum {
            name: ast::Ident::new("Foo", 5..8),
            values: vec![ast::EnumValue {
                name: ast::Ident::new("VAL", 32..35),
                value: ast::Int {
                    negative: true,
                    value: 1,
                    span: 39..40,
                },
                options: vec![],
            }],
            options: vec![ast::Option {
                name: ast::FullIdent::from(ast::Ident::new("bar", 18..21)),
                field_name: None,
                value: ast::Constant::String(ast::String {
                    value: "quz".to_owned(),
                    span: 24..29
                }),
            }],
        });
        case!(parse_enum("enum Foo { BAR = 0 [opt = 0.5]; }") => ast::Enum {
            name: ast::Ident::new("Foo", 5..8),
            values: vec![ast::EnumValue {
                name: ast::Ident::new("BAR", 11..14),
                value: ast::Int {
                    negative: false,
                    value: 0,
                    span: 17..18,
                },
                options: vec![ast::Option {
                    name: ast::FullIdent::from(ast::Ident::new("opt", 20..23)),
                    field_name: None,
                    value: ast::Constant::Float(ast::Float {
                        value: 0.5,
                        span: 26..29
                    }),
                }],
            }],
            options: vec![],
        });
        case!(parse_enum("enum 3") => Err(vec![ParseError::UnexpectedToken {
            expected: "an identifier".to_owned(),
            found: Token::Int(3),
            span: SourceSpan::from(5..6),
        }]));
        case!(parse_enum("enum Foo 0.1") => Err(vec![ParseError::UnexpectedToken {
            expected: "'{'".to_owned(),
            found: Token::Float(0.1),
            span: SourceSpan::from(9..12),
        }]));
        case!(parse_enum("enum Foo {]") => Err(vec![ParseError::UnexpectedToken {
            expected: "an identifier, '}', ';' or 'option'".to_owned(),
            found: Token::RightBracket,
            span: SourceSpan::from(10..11),
        }]));
        case!(parse_enum("enum Foo { BAR .") => Err(vec![ParseError::UnexpectedToken {
            expected: "'='".to_owned(),
            found: Token::Dot,
            span: SourceSpan::from(15..16),
        }]));
        case!(parse_enum("enum Foo { BAR = foo") => Err(vec![ParseError::UnexpectedToken {
            expected: "an integer".to_owned(),
            found: Token::Ident("foo".to_owned()),
            span: SourceSpan::from(17..20),
        }]));
    }

    #[test]
    fn parse_service() {
        case!(parse_service("service Foo {}") => ast::Service {
            name: ast::Ident::new("Foo", 8..11),
            options: vec![],
            methods: vec![],
        });
        case!(parse_service("service Foo { ; ; }") => ast::Service {
            name: ast::Ident::new("Foo", 8..11),
            options: vec![],
            methods: vec![],
        });
        case!(parse_service("service service { }") => ast::Service {
            name: ast::Ident::new("service", 8..15),
            options: vec![],
            methods: vec![],
        });
        case!(parse_service("service Foo { rpc bar(A) returns (B.C); }") => ast::Service {
            name: ast::Ident::new("Foo", 8..11),
            options: vec![],
            methods: vec![ast::Method {
                name: ast::Ident::new("bar", 18..21),
                is_client_streaming: false,
                input_ty: ast::TypeName {
                    leading_dot: None,
                    name: FullIdent::from(ast::Ident::new("A", 22..23)),
                },
                is_server_streaming: false,
                output_ty: ast::TypeName {
                    leading_dot: None,
                    name: FullIdent::from(vec![
                        ast::Ident::new("B", 34..35),
                        ast::Ident::new("C", 36..37),
                    ]),
                },
                options: vec![],
            }],
        });
        case!(parse_service("service Foo { rpc bar(stream .A.B) returns (stream .C); }") => ast::Service {
            name: ast::Ident::new("Foo", 8..11),
            options: vec![],
            methods: vec![ast::Method {
                name: ast::Ident::new("bar", 18..21),
                is_client_streaming: true,
                input_ty: ast::TypeName {
                    leading_dot: Some(29..30),
                    name: FullIdent::from(vec![
                        ast::Ident::new("A", 30..31),
                        ast::Ident::new("B", 32..33),
                    ]),
                },
                is_server_streaming: true,
                output_ty: ast::TypeName {
                    leading_dot: Some(51..52),
                    name: FullIdent::from(ast::Ident::new("C", 52..53)),
                },
                options: vec![],
            }],
        });
        case!(parse_service("service Foo { rpc bar(A) returns (B.C) { } }") => ast::Service {
            name: ast::Ident::new("Foo", 8..11),
            options: vec![],
            methods: vec![ast::Method {
                name: ast::Ident::new("bar", 18..21),
                is_client_streaming: false,
                input_ty: ast::TypeName {
                    leading_dot: None,
                    name: FullIdent::from(ast::Ident::new("A", 22..23)),
                },
                is_server_streaming: false,
                output_ty: ast::TypeName {
                    leading_dot: None,
                    name: FullIdent::from(vec![
                        ast::Ident::new("B", 34..35),
                        ast::Ident::new("C", 36..37),
                    ]),
                },
                options: vec![],
            }],
        });
        case!(parse_service("service Foo { rpc bar(A) returns (B.C) { ; ; } }") => ast::Service {
            name: ast::Ident::new("Foo", 8..11),
            options: vec![],
            methods: vec![ast::Method {
                name: ast::Ident::new("bar", 18..21),
                is_client_streaming: false,
                input_ty: ast::TypeName {
                    leading_dot: None,
                    name: FullIdent::from(ast::Ident::new("A", 22..23)),
                },
                is_server_streaming: false,
                output_ty: ast::TypeName {
                    leading_dot: None,
                    name: FullIdent::from(vec![
                        ast::Ident::new("B", 34..35),
                        ast::Ident::new("C", 36..37),
                    ]),
                },
                options: vec![],
            }],
        });
        case!(parse_service("service Foo { rpc bar(A) returns (B.C) { option opt = -1; } }") => ast::Service {
            name: ast::Ident::new("Foo", 8..11),
            options: vec![],
            methods: vec![ast::Method {
                name: ast::Ident::new("bar", 18..21),
                is_client_streaming: false,
                input_ty: ast::TypeName {
                    leading_dot: None,
                    name: FullIdent::from(ast::Ident::new("A", 22..23)),
                },
                is_server_streaming: false,
                output_ty: ast::TypeName {
                    leading_dot: None,
                    name: FullIdent::from(vec![
                        ast::Ident::new("B", 34..35),
                        ast::Ident::new("C", 36..37),
                    ]),
                },
                options: vec![ast::Option {
                    name: ast::FullIdent::from(ast::Ident::new("opt", 48..51)),
                    field_name: None,
                    value: ast::Constant::Int(ast::Int {
                        negative: true,
                        value: 1,
                        span: 55..56,
                    })
                }],
            }],
        });
        case!(parse_service("service ;") => Err(vec![ParseError::UnexpectedToken {
            expected: "an identifier".to_owned(),
            found: Token::Semicolon,
            span: SourceSpan::from(8..9),
        }]));
        case!(parse_service("service Foo (") => Err(vec![ParseError::UnexpectedToken {
            expected: "'{'".to_owned(),
            found: Token::LeftParen,
            span: SourceSpan::from(12..13),
        }]));
        case!(parse_service("service Foo { bar") => Err(vec![ParseError::UnexpectedToken {
            expected: "'rpc', '}', 'option' or ';'".to_owned(),
            found: Token::Ident("bar".to_owned()),
            span: SourceSpan::from(14..17),
        }]));
        case!(parse_service("service Foo { rpc =") => Err(vec![ParseError::UnexpectedToken {
            expected: "an identifier".to_owned(),
            found: Token::Equals,
            span: SourceSpan::from(18..19),
        }]));
        case!(parse_service("service Foo { rpc bar{") => Err(vec![ParseError::UnexpectedToken {
            expected: "'('".to_owned(),
            found: Token::LeftBrace,
            span: SourceSpan::from(21..22),
        }]));
        case!(parse_service("service Foo { rpc bar(+") => Err(vec![ParseError::UnexpectedToken {
            expected: "'stream' or a type name".to_owned(),
            found: Token::Plus,
            span: SourceSpan::from(22..23),
        }]));
        case!(parse_service("service Foo { rpc bar(A(") => Err(vec![ParseError::UnexpectedToken {
            expected: "'.' or ')'".to_owned(),
            found: Token::LeftParen,
            span: SourceSpan::from(23..24),
        }]));
        case!(parse_service("service Foo { rpc bar(A) [") => Err(vec![ParseError::UnexpectedToken {
            expected: "'returns'".to_owned(),
            found: Token::LeftBracket,
            span: SourceSpan::from(25..26),
        }]));
        case!(parse_service("service Foo { rpc bar(A) returns =") => Err(vec![ParseError::UnexpectedToken {
            expected: "'('".to_owned(),
            found: Token::Equals,
            span: SourceSpan::from(33..34),
        }]));
        case!(parse_service("service Foo { rpc bar(A) returns ()") => Err(vec![ParseError::UnexpectedToken {
            expected: "'stream' or a type name".to_owned(),
            found: Token::RightParen,
            span: SourceSpan::from(34..35),
        }]));
        case!(parse_service("service Foo { rpc bar(A) returns (stream =)") => Err(vec![ParseError::UnexpectedToken {
            expected: "a type name".to_owned(),
            found: Token::Equals,
            span: SourceSpan::from(41..42),
        }]));
        case!(parse_service("service Foo { rpc bar(A) returns (stream B}") => Err(vec![ParseError::UnexpectedToken {
            expected: "'.' or ')'".to_owned(),
            found: Token::RightBrace,
            span: SourceSpan::from(42..43),
        }]));
        case!(parse_service("service Foo { rpc bar(A) returns (stream B) )") => Err(vec![ParseError::UnexpectedToken {
            expected: "';' or '{'".to_owned(),
            found: Token::RightParen,
            span: SourceSpan::from(44..45),
        }]));
        case!(parse_service("service Foo { rpc bar(A) returns (stream B) {rpc") => Err(vec![ParseError::UnexpectedToken {
            expected: "'option', '}' or ';'".to_owned(),
            found: Token::Rpc,
            span: SourceSpan::from(45..48),
        }]));
    }
}
