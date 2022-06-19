use std::{fmt::Write, iter::once};

use logos::{Lexer, Logos, Span};
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

mod lex;
#[cfg(test)]
mod tests;

use self::lex::Token;
use crate::ast::{self, FieldLabel, FullIdent};

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
    InvalidIdentifier {
        span: SourceSpan,
    },
    InvalidGroupName {
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
        Ok(file) if parser.lexer.extras.errors.is_empty() => Ok(file),
        _ => Err(parser.lexer.extras.errors),
    }
}

struct Parser<'a> {
    lexer: Lexer<'a, Token>,
    peek: Option<(Token, Span)>,
}

enum Statement {
    Empty,
    Package(ast::Package),
    Import(ast::Import),
    Option(ast::Option),
    Definition(ast::Definition),
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
        let syntax = if self.bump_if_eq(Token::Syntax) {
            match self.peek() {
                Some((Token::StringLiteral(syntax), span)) => match &*syntax {
                    "proto2" => {
                        self.bump();
                        ast::Syntax::Proto2
                    }
                    "proto3" => {
                        self.bump();
                        ast::Syntax::Proto3
                    }
                    _ => {
                        self.add_error(ParseError::UnknownSyntax { span: span.into() });
                        return Err(());
                    }
                },
                _ => self.unexpected_token("an identifier or '('")?,
            }
        } else {
            ast::Syntax::Proto2
        };

        let mut packages = Vec::new();
        let mut imports = Vec::new();
        let mut options = Vec::new();
        let mut definitions = Vec::new();

        loop {
            match self.parse_statement() {
                Ok(Some(Statement::Empty)) => continue,
                Ok(Some(Statement::Package(package))) => packages.push(package),
                Ok(Some(Statement::Import(import))) => imports.push(import),
                Ok(Some(Statement::Option(option))) => options.push(option),
                Ok(Some(Statement::Definition(definition))) => definitions.push(definition),
                Ok(None) => break,
                Err(()) => self.skip_until(&[
                    Token::Enum,
                    Token::Extend,
                    Token::Import,
                    Token::Message,
                    Token::Option,
                    Token::Service,
                    Token::Package,
                ]),
            }
        }

        Ok(ast::File {
            syntax,
            packages,
            imports,
            options,
            definitions,
        })
    }

    fn parse_statement(&mut self) -> Result<Option<Statement>, ()> {
        match self.peek() {
            Some((Token::Semicolon, _)) => {
                self.bump();
                Ok(Some(Statement::Empty))
            }
            Some((Token::Import, _)) => Ok(Some(Statement::Import(self.parse_import()?))),
            Some((Token::Package, _)) => Ok(Some(Statement::Package(self.parse_package()?))),
            Some((Token::Option, _)) => Ok(Some(Statement::Option(self.parse_option()?))),
            Some((Token::Extend, _)) => Ok(Some(Statement::Definition(
                ast::Definition::Extension(self.parse_extension()?),
            ))),
            Some((Token::Message, _)) => Ok(Some(Statement::Definition(ast::Definition::Message(
                self.parse_message()?,
            )))),
            Some((Token::Enum, _)) => Ok(Some(Statement::Definition(ast::Definition::Enum(
                self.parse_enum()?,
            )))),
            Some((Token::Service, _)) => Ok(Some(Statement::Definition(ast::Definition::Service(
                self.parse_service()?,
            )))),
            None => Ok(None),
            _ => self.unexpected_token(
                "'enum', 'extend', 'import', 'message', 'option', 'service', 'package' or ';'",
            ),
        }
    }

    fn parse_package(&mut self) -> Result<ast::Package, ()> {
        self.expect_eq(Token::Package)?;

        let name = self.parse_full_ident(&[Token::Semicolon])?;

        self.expect_eq(Token::Semicolon)?;

        Ok(ast::Package { name })
    }

    fn parse_import(&mut self) -> Result<ast::Import, ()> {
        self.expect_eq(Token::Import)?;

        let kind = match self.peek() {
            Some((Token::Weak, _)) => {
                self.bump();
                Some(ast::ImportKind::Weak)
            }
            Some((Token::Public, _)) => {
                self.bump();
                Some(ast::ImportKind::Public)
            }
            Some((Token::StringLiteral(_), _)) => None,
            _ => self.unexpected_token("a string literal, 'public' or 'weak'")?,
        };

        let value = self.parse_string()?;

        Ok(ast::Import { kind, value })
    }

    fn parse_message(&mut self) -> Result<ast::Message, ()> {
        self.expect_eq(Token::Message)?;

        let name = self.parse_ident()?;

        let body = self.parse_message_body()?;

        Ok(ast::Message { name, body })
    }

    fn parse_message_body(&mut self) -> Result<ast::MessageBody, ()> {
        let mut fields = Vec::new();
        let mut oneofs = Vec::new();
        let mut enums = Vec::new();
        let mut messages = Vec::new();
        let mut extensions = Vec::new();
        let mut options = Vec::new();
        let mut reserved = Vec::new();
        let mut extension_ranges = Vec::new();

        self.expect_eq(Token::LeftBrace)?;

        loop {
            match self.peek() {
                Some((tok, _)) if is_field_start_token(&tok) => fields.push(self.parse_field()?),
                Some((Token::Oneof, _)) => oneofs.push(self.parse_oneof()?),
                Some((Token::Enum, _)) => enums.push(self.parse_enum()?),
                Some((Token::Message, _)) => messages.push(self.parse_message()?),
                Some((Token::Extend, _)) => extensions.push(self.parse_extension()?),
                Some((Token::Option, _)) => options.push(self.parse_option()?),
                Some((Token::Reserved, _)) => reserved.push(self.parse_reserved()?),
                Some((Token::Extensions, _)) => {
                    extension_ranges.extend(self.parse_extension_range()?)
                }
                Some((Token::Semicolon, _)) => {
                    self.bump();
                    continue;
                }
                Some((Token::RightBrace, _)) => {
                    self.bump();
                    break;
                }
                _ => self.unexpected_token(
                    "a message field, oneof, reserved range, enum, message or '}'",
                )?,
            }
        }

        Ok(ast::MessageBody {
            fields,
            oneofs,
            enums,
            messages,
            extensions,
            options,
            reserved,
            extension_ranges,
        })
    }

    fn parse_field(&mut self) -> Result<ast::MessageField, ()> {
        let label = match self.peek() {
            Some((Token::Optional, _)) => {
                self.bump();
                Some(FieldLabel::Optional)
            }
            Some((Token::Required, _)) => {
                self.bump();
                Some(FieldLabel::Required)
            }
            Some((Token::Repeated, _)) => {
                self.bump();
                Some(FieldLabel::Repeated)
            }
            Some((Token::Map, _)) => {
                return Ok(ast::MessageField::Map(self.parse_map()?));
            }
            Some((tok, _)) if is_field_start_token(&tok) => None,
            _ => self.unexpected_token("a message field")?,
        };

        match self.peek() {
            Some((Token::Group, _)) => {
                self.bump();

                let name = self.parse_ident()?;
                if !is_valid_group_name(&name.value) {
                    self.add_error(ParseError::InvalidGroupName {
                        span: SourceSpan::from(name.span.clone()),
                    });
                }

                self.expect_eq(Token::Equals)?;

                let number = self.parse_positive_int()?;

                let body = self.parse_message_body()?;

                Ok(ast::MessageField::Group(ast::Group {
                    label,
                    name,
                    number,
                    body,
                }))
            }
            _ => {
                let ty = self.parse_field_type(&[Token::Ident(Default::default())])?;

                let name = self.parse_ident()?;

                self.expect_eq(Token::Equals)?;

                let number = self.parse_positive_int()?;

                let options = match self.peek() {
                    Some((Token::LeftBracket, _)) => {
                        let options = self.parse_options_list()?;
                        self.expect_eq(Token::Semicolon)?;
                        options
                    }
                    Some((Token::Semicolon, _)) => {
                        self.bump();
                        vec![]
                    }
                    _ => self.unexpected_token("';' or '['")?,
                };

                Ok(ast::MessageField::Field(ast::Field {
                    label,
                    ty,
                    name,
                    number,
                    options,
                }))
            }
        }
    }

    fn parse_map(&mut self) -> Result<ast::Map, ()> {
        self.expect_eq(Token::Map)?;

        self.expect_eq(Token::LeftAngleBracket)?;
        let key_ty = self.parse_key_type()?;
        self.expect_eq(Token::Comma)?;
        let ty = self.parse_field_type(&[Token::RightAngleBracket])?;
        self.expect_eq(Token::RightAngleBracket)?;

        let name = self.parse_ident()?;

        self.expect_eq(Token::Equals)?;

        let number = self.parse_positive_int()?;

        let options = match self.peek() {
            Some((Token::LeftBracket, _)) => {
                let options = self.parse_options_list()?;
                self.expect_eq(Token::Semicolon)?;
                options
            }
            Some((Token::Semicolon, _)) => {
                self.bump();
                vec![]
            }
            _ => self.unexpected_token("';' or '['")?,
        };

        Ok(ast::Map {
            key_ty,
            ty,
            name,
            number,
            options,
        })
    }

    fn parse_extension(&mut self) -> Result<ast::Extension, ()> {
        self.expect_eq(Token::Extend)?;

        let extendee = self.parse_type_name(&[Token::LeftBrace])?;

        self.expect_eq(Token::LeftBrace)?;

        let mut fields = Vec::new();
        loop {
            match self.peek() {
                Some((tok, _)) if is_field_start_token(&tok) || tok == Token::Group => {
                    fields.push(self.parse_field()?);
                }
                Some((Token::Semicolon, _)) => {
                    self.bump();
                    continue;
                }
                Some((Token::RightBrace, _)) => {
                    self.bump();
                    break;
                }
                _ => self.unexpected_token("a message field, '}' or ';'")?,
            }
        }

        Ok(ast::Extension { extendee, fields })
    }

    fn parse_service(&mut self) -> Result<ast::Service, ()> {
        self.expect_eq(Token::Service)?;

        let name = self.parse_ident()?;

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

        let name = self.parse_ident()?;

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

        let name = self.parse_ident()?;

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
        let name = self.parse_ident()?;

        self.expect_eq(Token::Equals)?;

        let negative = self.bump_if_eq(Token::Minus);
        let value = match self.peek() {
            Some((Token::IntLiteral(value), span)) => {
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

    fn parse_oneof(&mut self) -> Result<ast::Oneof, ()> {
        todo!()
    }

    fn parse_key_type(&mut self) -> Result<ast::KeyTy, ()> {
        let ty = match self.peek() {
            Some((Token::Int32, _)) => ast::KeyTy::Int32,
            Some((Token::Int64, _)) => ast::KeyTy::Int64,
            Some((Token::Uint32, _)) => ast::KeyTy::Uint32,
            Some((Token::Uint64, _)) => ast::KeyTy::Uint64,
            Some((Token::Sint32, _)) => ast::KeyTy::Sint32,
            Some((Token::Sint64, _)) => ast::KeyTy::Sint64,
            Some((Token::Fixed32, _)) => ast::KeyTy::Fixed32,
            Some((Token::Fixed64, _)) => ast::KeyTy::Fixed64,
            Some((Token::Sfixed32, _)) => ast::KeyTy::Sfixed32,
            Some((Token::Sfixed64, _)) => ast::KeyTy::Sfixed64,
            Some((Token::Bool, _)) => ast::KeyTy::Bool,
            Some((Token::String, _)) => ast::KeyTy::String,
            _ => self.unexpected_token("an integer type or 'string'")?,
        };

        self.bump();
        Ok(ty)
    }

    fn parse_field_type(&mut self, terminators: &[Token]) -> Result<ast::Ty, ()> {
        let scalar_ty = match self.peek() {
            Some((Token::Double, _)) => ast::Ty::Double,
            Some((Token::Float, _)) => ast::Ty::Float,
            Some((Token::Int32, _)) => ast::Ty::Int32,
            Some((Token::Int64, _)) => ast::Ty::Int64,
            Some((Token::Uint32, _)) => ast::Ty::Uint32,
            Some((Token::Uint64, _)) => ast::Ty::Uint64,
            Some((Token::Sint32, _)) => ast::Ty::Sint32,
            Some((Token::Sint64, _)) => ast::Ty::Sint64,
            Some((Token::Fixed32, _)) => ast::Ty::Fixed32,
            Some((Token::Fixed64, _)) => ast::Ty::Fixed64,
            Some((Token::Sfixed32, _)) => ast::Ty::Sfixed32,
            Some((Token::Sfixed64, _)) => ast::Ty::Sfixed64,
            Some((Token::Bool, _)) => ast::Ty::Bool,
            Some((Token::String, _)) => ast::Ty::String,
            Some((Token::Bytes, _)) => ast::Ty::Bytes,
            Some((Token::Dot | Token::Ident(_), _)) => {
                return Ok(ast::Ty::Named(self.parse_type_name(terminators)?))
            }
            _ => self.unexpected_token("a field type")?,
        };

        self.bump();
        Ok(scalar_ty)
    }

    fn parse_reserved(&mut self) -> Result<ast::Reserved, ()> {
        self.expect_eq(Token::Reserved)?;

        match self.peek() {
            Some((Token::IntLiteral(_), _)) => {
                Ok(ast::Reserved::Ranges(self.parse_reserved_ranges()?))
            }
            Some((Token::StringLiteral(_), _)) => {
                Ok(ast::Reserved::Names(self.parse_reserved_names()?))
            }
            _ => self.unexpected_token("a positive integer or string"),
        }
    }

    fn parse_extension_range(&mut self) -> Result<Vec<ast::ReservedRange>, ()> {
        self.expect_eq(Token::Extensions)?;

        self.parse_reserved_ranges()
    }

    fn parse_reserved_names(&mut self) -> Result<Vec<ast::Ident>, ()> {
        let mut names = vec![self.parse_ident_string()?];

        loop {
            match self.peek() {
                Some((Token::Comma, _)) => {
                    self.bump();
                    names.push(self.parse_ident_string()?);
                }
                Some((Token::Semicolon, _)) => {
                    self.bump();
                    break;
                }
                _ => self.unexpected_token("',' or ';'")?,
            }
        }

        Ok(names)
    }

    fn parse_ident_string(&mut self) -> Result<ast::Ident, ()> {
        let string = self.parse_string()?;
        if !is_valid_ident(&string.value) {
            self.add_error(ParseError::InvalidIdentifier {
                span: SourceSpan::from(string.span.clone()),
            })
        }
        Ok(ast::Ident {
            value: string.value,
            span: string.span,
        })
    }

    fn parse_reserved_ranges(&mut self) -> Result<Vec<ast::ReservedRange>, ()> {
        let mut ranges = vec![self.parse_reserved_range()?];

        loop {
            match self.peek() {
                Some((Token::Comma, _)) => {
                    self.bump();
                    ranges.push(self.parse_reserved_range()?);
                    continue;
                }
                Some((Token::Semicolon, _)) => {
                    self.bump();
                    break;
                }
                _ => self.unexpected_token("',' or ';'")?,
            }
        }

        Ok(ranges)
    }

    fn parse_reserved_range(&mut self) -> Result<ast::ReservedRange, ()> {
        let start = self.parse_positive_int()?;

        let end = match self.peek() {
            Some((Token::To, _)) => {
                self.bump();
                match self.peek() {
                    Some((Token::IntLiteral(value), span)) => {
                        self.bump();
                        ast::ReservedRangeEnd::Int(ast::Int {
                            negative: false,
                            value,
                            span,
                        })
                    }
                    Some((Token::Max, _)) => {
                        self.bump();
                        ast::ReservedRangeEnd::Max
                    }
                    _ => self.unexpected_token("an integer or 'max'")?,
                }
            }
            Some((Token::Comma | Token::Semicolon, _)) => ast::ReservedRangeEnd::None,
            _ => self.unexpected_token("'to', ',' or ';'")?,
        };

        Ok(ast::ReservedRange { start, end })
    }

    fn parse_options_list(&mut self) -> Result<Vec<ast::Option>, ()> {
        self.expect_eq(Token::LeftBracket)?;

        let mut options = vec![self.parse_option_body(&[Token::Comma, Token::RightBracket])?];
        loop {
            match self.peek() {
                Some((Token::Comma, _)) => {
                    self.bump();
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

            field_name.get_or_insert(vec![]).push(self.parse_ident()?);
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
            Some((Token::IntLiteral(_) | Token::FloatLiteral(_), _)) => {
                self.parse_int_or_float(false)?
            }
            Some((Token::StringLiteral(value), span)) => {
                self.bump();
                ast::Constant::String(ast::String { value, span })
            }
            Some((Token::BoolLiteral(value), span)) => {
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
            Some((Token::IntLiteral(value), span)) => {
                self.bump();
                Ok(ast::Constant::Int(ast::Int {
                    value,
                    span,
                    negative: negate,
                }))
            }
            Some((Token::FloatLiteral(value), span)) => {
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
        let mut result = vec![self.parse_ident()?];

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

            result.push(self.parse_ident()?);
        }
    }

    fn parse_ident(&mut self) -> Result<ast::Ident, ()> {
        self.expect(
            |tok, span| tok.into_ident().map(|value| ast::Ident::new(value, span)),
            "an identifier",
        )
    }

    fn parse_positive_int(&mut self) -> Result<ast::Int, ()> {
        match self.peek() {
            Some((Token::IntLiteral(value), span)) => {
                self.bump();
                Ok(ast::Int {
                    negative: false,
                    value,
                    span,
                })
            }
            _ => self.unexpected_token("a positive integer")?,
        }
    }

    fn parse_string(&mut self) -> Result<ast::String, ()> {
        match self.peek() {
            Some((Token::StringLiteral(value), span)) => {
                self.bump();
                Ok(ast::String { value, span })
            }
            _ => self.unexpected_token("a string literal"),
        }
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

    fn skip_until(&mut self, tokens: &[Token]) {
        while !self.bump_if(|tok| tokens.contains(tok)) && self.peek().is_some() {}
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

fn is_field_start_token(tok: &Token) -> bool {
    matches!(
        tok,
        Token::Map
            | Token::Repeated
            | Token::Optional
            | Token::Required
            | Token::Double
            | Token::Float
            | Token::Int32
            | Token::Int64
            | Token::Uint32
            | Token::Uint64
            | Token::Sint32
            | Token::Sint64
            | Token::Fixed32
            | Token::Fixed64
            | Token::Sfixed32
            | Token::Sfixed64
            | Token::Bool
            | Token::String
            | Token::Bytes
            | Token::Dot
            | Token::Ident(_),
    )
}

fn fmt_expected(ts: impl Iterator<Item = Token>) -> String {
    fn fmt_token(s: &mut String, t: &Token) {
        if let Token::Ident(_) = t {
            s.push_str("an identifier");
        } else {
            write!(s, "'{}'", t).unwrap();
        }
    }

    let ts: Vec<_> = ts.collect();

    let mut s = String::with_capacity(32);
    fmt_token(&mut s, &ts[0]);
    if ts.len() > 1 {
        for t in &ts[1..][..ts.len() - 2] {
            s.push_str(", ");
            fmt_token(&mut s, t);
        }
        s.push_str(" or ");
        fmt_token(&mut s, &ts[ts.len() - 1]);
    }
    s
}

fn is_valid_ident(s: &str) -> bool {
    !s.is_empty()
        && s.as_bytes()[0].is_ascii_alphabetic()
        && s.as_bytes()[1..]
            .iter()
            .all(|&ch| ch.is_ascii_alphanumeric() || ch == b'_')
}

fn is_valid_group_name(s: &str) -> bool {
    !s.is_empty()
        && s.as_bytes()[0].is_ascii_uppercase()
        && s.as_bytes()[1..]
            .iter()
            .all(|&ch| ch.is_ascii_alphanumeric() || ch == b'_')
}
