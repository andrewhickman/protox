use std::{
    fmt::{self, Write},
    iter::once,
};

use logos::{Lexer, Logos, Span};
use miette::Diagnostic;
use thiserror::Error;

mod comments;
mod lex;
#[cfg(test)]
mod tests;

use self::comments::Comments;
use self::lex::Token;
use crate::ast::{self, FieldLabel, FullIdent};

#[derive(Error, Debug, Diagnostic, PartialEq)]
pub(crate) enum ParseError {
    #[error("invalid token")]
    InvalidToken {
        #[label("found here")]
        span: Span,
    },
    #[error("integer is too large")]
    IntegerOutOfRange {
        #[label("integer defined here")]
        span: Span,
    },
    #[error("invalid string character")]
    InvalidStringCharacters {
        #[label("invalid characters")]
        span: Span,
    },
    #[error("unterminated string")]
    UnterminatedString {
        #[label("string starts here")]
        span: Span,
    },
    #[error("invalid string escape")]
    InvalidStringEscape {
        #[label("defined here")]
        span: Span,
    },
    #[error("nested block comments are not supported")]
    NestedBlockComment {
        #[label("defined here")]
        span: Span,
    },
    #[error("unknown syntax")]
    UnknownSyntax {
        #[label("defined here")]
        span: Span,
    },
    #[error("invalid identifier")]
    #[help("identifiers must consist of an letter followed by letters or numbers")]
    InvalidIdentifier {
        #[label("defined here")]
        span: Span,
    },
    #[error("invalid group name")]
    #[help("group names must consist of a capital letter followed by letters or numbers")]
    InvalidGroupName {
        #[label("defined here")]
        span: Span,
    },
    #[error("invalid group name")]
    #[help(
        "imports may not contain backslashes, repeated forward slashes, '.' or '..' components"
    )]
    InvalidImport {
        #[label("defined here")]
        span: Span,
    },
    #[error("multiple package names specified")]
    DuplicatePackage {
        #[label("defined here...")]
        first: Span,
        #[label("...and again here")]
        second: Span,
    },
    #[error("expected {expected}, but found '{found}'")]
    UnexpectedToken {
        expected: String,
        found: Token<'static>,
        #[label("found here")]
        span: Span,
    },
    #[error("expected {expected}, but reached end of file")]
    UnexpectedEof { expected: String },
}

pub(crate) fn parse(source: &str) -> Result<ast::File, Vec<ParseError>> {
    let mut parser = Parser::new(source);
    match parser.parse_file() {
        Ok(file) if parser.lexer.extras.errors.is_empty() => Ok(file),
        _ => Err(parser.lexer.extras.errors),
    }
}

struct Parser<'a> {
    lexer: Lexer<'a, Token<'a>>,
    peek: Option<(Token<'a>, Span)>,
    comments: Comments,
    syntax: ast::Syntax,
}

enum Statement {
    Empty,
    Package(ast::Package),
    Import(ast::Import),
    Option(ast::Option),
    Message(ast::Message),
    Enum(ast::Enum),
    Service(ast::Service),
    Extend(ast::Extend),
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Parser {
            lexer: Token::lexer(source),
            comments: Comments::new(),
            peek: None,
            syntax: ast::Syntax::default(),
        }
    }

    fn parse_file(&mut self) -> Result<ast::File, ()> {
        if self.bump_if_eq(Token::Syntax) {
            self.expect_eq(Token::Equals)?;
            self.syntax = match self.peek() {
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
                        self.add_error(ParseError::UnknownSyntax { span });
                        return Err(());
                    }
                },
                _ => self.unexpected_token("an identifier or '('")?,
            }
        }

        let mut package: Option<ast::Package> = None;
        let mut imports = Vec::new();
        let mut options = Vec::new();
        let mut items = Vec::new();

        loop {
            match self.parse_statement() {
                Ok(Some(Statement::Empty)) => continue,
                Ok(Some(Statement::Package(new_package))) => {
                    if let Some(existing_package) = &package {
                        self.add_error(ParseError::DuplicatePackage {
                            first: existing_package.span.clone(),
                            second: new_package.span,
                        })
                    } else {
                        package = Some(new_package);
                    }
                }
                Ok(Some(Statement::Import(import))) => imports.push(import),
                Ok(Some(Statement::Option(option))) => options.push(option),
                Ok(Some(Statement::Message(message))) => {
                    items.push(ast::FileItem::Message(message))
                }
                Ok(Some(Statement::Enum(enm))) => items.push(ast::FileItem::Enum(enm)),
                Ok(Some(Statement::Service(service))) => {
                    items.push(ast::FileItem::Service(service))
                }
                Ok(Some(Statement::Extend(extend))) => items.push(ast::FileItem::Extend(extend)),
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
            syntax: self.syntax,
            package,
            imports,
            options,
            items,
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
            Some((Token::Extend, _)) => Ok(Some(Statement::Extend(self.parse_extend()?))),
            Some((Token::Message, _)) => Ok(Some(Statement::Message(self.parse_message()?))),
            Some((Token::Enum, _)) => Ok(Some(Statement::Enum(self.parse_enum()?))),
            Some((Token::Service, _)) => Ok(Some(Statement::Service(self.parse_service()?))),
            None => Ok(None),
            _ => self.unexpected_token(
                "'enum', 'extend', 'import', 'message', 'option', 'service', 'package' or ';'",
            ),
        }
    }

    fn parse_package(&mut self) -> Result<ast::Package, ()> {
        let leading_comments = self.parse_leading_comments();

        let start = self.expect_eq(Token::Package)?;

        let name = self.parse_full_ident(&[ExpectedToken::SEMICOLON])?;

        let end = self.expect_eq(Token::Semicolon)?;

        let comments = self.parse_trailing_comment(leading_comments);

        Ok(ast::Package {
            name,
            comments,
            span: join_span(start, end),
        })
    }

    fn parse_import(&mut self) -> Result<ast::Import, ()> {
        let leading_comments = self.parse_leading_comments();

        let start = self.expect_eq(Token::Import)?;

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
        if !is_valid_import(&value.value) {
            self.add_error(ParseError::InvalidImport {
                span: value.span.clone(),
            });
        }

        let end = self.expect_eq(Token::Semicolon)?;

        let comments = self.parse_trailing_comment(leading_comments);

        Ok(ast::Import {
            kind,
            value,
            comments,
            span: join_span(start, end),
        })
    }

    fn parse_message(&mut self) -> Result<ast::Message, ()> {
        let leading_comments = self.parse_leading_comments();

        let start = self.expect_eq(Token::Message)?;

        let name = self.parse_ident()?;

        self.expect_eq(Token::LeftBrace)?;
        let comments = self.parse_trailing_comment(leading_comments);

        let (body, end) = self.parse_message_body()?;

        Ok(ast::Message {
            name,
            body,
            comments,
            span: join_span(start, end),
        })
    }

    fn parse_message_body(&mut self) -> Result<(ast::MessageBody, Span), ()> {
        let mut fields = Vec::new();
        let mut enums = Vec::new();
        let mut messages = Vec::new();
        let mut extends = Vec::new();
        let mut options = Vec::new();
        let mut reserved = Vec::new();
        let mut extensions = Vec::new();

        let end = loop {
            match self.peek() {
                Some((tok, _)) if is_field_start_token(&tok) => fields.push(self.parse_field()?),
                Some((Token::Oneof, _)) => {
                    fields.push(ast::MessageField::Oneof(self.parse_oneof()?))
                }
                Some((Token::Enum, _)) => enums.push(self.parse_enum()?),
                Some((Token::Message, _)) => messages.push(self.parse_message()?),
                Some((Token::Extend, _)) => extends.push(self.parse_extend()?),
                Some((Token::Option, _)) => options.push(self.parse_option()?),
                Some((Token::Reserved, _)) => reserved.push(self.parse_reserved()?),
                Some((Token::Extensions, _)) => extensions.push(self.parse_extensions()?),
                Some((Token::Semicolon, _)) => {
                    self.bump();
                    continue;
                }
                Some((Token::RightBrace, span)) => {
                    self.bump();
                    break span;
                }
                _ => self.unexpected_token(
                    "a message field, oneof, reserved range, enum, message, option or '}'",
                )?,
            }
        };

        Ok((
            ast::MessageBody {
                fields,
                enums,
                messages,
                extends,
                options,
                reserved,
                extensions,
            },
            end,
        ))
    }

    fn parse_field(&mut self) -> Result<ast::MessageField, ()> {
        let leading_comments = self.parse_leading_comments();

        let (label, start) = match self.peek() {
            Some((Token::Optional, span)) => {
                self.bump();
                (Some(FieldLabel::Optional), span)
            }
            Some((Token::Required, span)) => {
                self.bump();
                (Some(FieldLabel::Required), span)
            }
            Some((Token::Repeated, span)) => {
                self.bump();
                (Some(FieldLabel::Repeated), span)
            }
            Some((tok, span)) if is_field_start_token(&tok) => (None, span),
            _ => self.unexpected_token("a message field")?,
        };

        match self.peek() {
            Some((Token::Map, _)) => Ok(ast::MessageField::Map(
                self.parse_map_inner(leading_comments, label)?,
            )),
            Some((Token::Group, _)) => {
                self.bump();

                let name = self.parse_ident()?;
                if !is_valid_group_name(&name.value) {
                    self.add_error(ParseError::InvalidGroupName {
                        span: name.span.clone(),
                    });
                }

                self.expect_eq(Token::Equals)?;

                let number = self.parse_int()?;

                let options = match self.peek() {
                    Some((Token::LeftBracket, _)) => self.parse_options_list()?,
                    Some((Token::LeftBrace, _)) => vec![],
                    _ => self.unexpected_token("'{' or '['")?,
                };

                self.expect_eq(Token::LeftBrace)?;

                let comments = self.parse_trailing_comment(leading_comments);

                let (body, end) = self.parse_message_body()?;

                Ok(ast::MessageField::Group(ast::Group {
                    label,
                    options,
                    name,
                    number,
                    body,
                    comments,
                    span: join_span(start, end),
                }))
            }
            _ => {
                let ty = self.parse_field_type(&[ExpectedToken::Ident])?;

                let name = self.parse_ident()?;

                self.expect_eq(Token::Equals)?;

                let number = self.parse_int()?;

                let options = match self.peek() {
                    Some((Token::LeftBracket, _)) => self.parse_options_list()?,
                    Some((Token::Semicolon, _)) => vec![],
                    _ => self.unexpected_token("';' or '['")?,
                };

                let end = self.expect_eq(Token::Semicolon)?;
                let comments = self.parse_trailing_comment(leading_comments);

                Ok(ast::MessageField::Field(ast::Field {
                    label,
                    ty,
                    name,
                    number,
                    options,
                    comments,
                    span: join_span(start, end),
                }))
            }
        }
    }

    #[cfg(test)]
    fn parse_map(&mut self) -> Result<ast::Map, ()> {
        let leading_comments = self.parse_leading_comments();
        self.parse_map_inner(leading_comments, None)
    }

    fn parse_map_inner(
        &mut self,
        leading_comments: (Vec<String>, Option<String>),
        label: Option<FieldLabel>,
    ) -> Result<ast::Map, ()> {
        let start = self.expect_eq(Token::Map)?;

        self.expect_eq(Token::LeftAngleBracket)?;
        let key_ty = self.parse_key_type()?;
        self.expect_eq(Token::Comma)?;
        let ty = self.parse_field_type(&[ExpectedToken::RIGHT_ANGLE_BRACKET])?;
        self.expect_eq(Token::RightAngleBracket)?;

        let name = self.parse_ident()?;

        self.expect_eq(Token::Equals)?;

        let number = self.parse_int()?;

        let options = match self.peek() {
            Some((Token::LeftBracket, _)) => self.parse_options_list()?,
            Some((Token::Semicolon, _)) => vec![],
            _ => self.unexpected_token("';' or '['")?,
        };

        let end = self.expect_eq(Token::Semicolon)?;
        let comments = self.parse_trailing_comment(leading_comments);

        Ok(ast::Map {
            label,
            key_ty,
            ty,
            name,
            number,
            options,
            comments,
            span: join_span(start, end),
        })
    }

    fn parse_extend(&mut self) -> Result<ast::Extend, ()> {
        let leading_comments = self.parse_leading_comments();

        let start = self.expect_eq(Token::Extend)?;

        let extendee = self.parse_type_name(&[ExpectedToken::LEFT_BRACE])?;

        self.expect_eq(Token::LeftBrace)?;
        let comments = self.parse_trailing_comment(leading_comments);

        let mut fields = Vec::new();
        let end = loop {
            match self.peek() {
                Some((tok, _)) if is_field_start_token(&tok) => {
                    fields.push(self.parse_field()?);
                }
                Some((Token::Semicolon, _)) => {
                    self.bump();
                    continue;
                }
                Some((Token::RightBrace, span)) => {
                    self.bump();
                    break span;
                }
                _ => self.unexpected_token("a message field, '}' or ';'")?,
            }
        };

        Ok(ast::Extend {
            extendee,
            fields,
            comments,
            span: join_span(start, end),
        })
    }

    fn parse_service(&mut self) -> Result<ast::Service, ()> {
        let leading_comments = self.parse_leading_comments();

        let start = self.expect_eq(Token::Service)?;

        let name = self.parse_ident()?;

        self.expect_eq(Token::LeftBrace)?;
        let comments = self.parse_trailing_comment(leading_comments);

        let mut options = Vec::new();
        let mut methods = Vec::new();

        let end = loop {
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
                Some((Token::RightBrace, span)) => {
                    self.bump();
                    break span;
                }
                _ => self.unexpected_token("'rpc', '}', 'option' or ';'")?,
            }
        };

        Ok(ast::Service {
            name,
            methods,
            options,
            comments,
            span: join_span(start, end),
        })
    }

    fn parse_service_rpc(&mut self) -> Result<ast::Method, ()> {
        let leading_comments = self.parse_leading_comments();

        let start = self.expect_eq(Token::Rpc)?;

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

        let input_ty = self.parse_type_name(&[ExpectedToken::RIGHT_PAREN])?;

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

        let output_ty = self.parse_type_name(&[ExpectedToken::RIGHT_PAREN])?;

        self.expect_eq(Token::RightParen)?;

        let mut options = Vec::new();
        let end = match self.peek() {
            Some((Token::Semicolon, span)) => {
                self.bump();
                span
            }
            Some((Token::LeftBrace, _)) => {
                self.bump();
                loop {
                    match self.peek() {
                        Some((Token::Option, _)) => {
                            options.push(self.parse_option()?);
                        }
                        Some((Token::RightBrace, span)) => {
                            self.bump();
                            break span;
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
        };

        let comments = self.parse_trailing_comment(leading_comments);

        Ok(ast::Method {
            name,
            input_ty,
            is_client_streaming,
            output_ty,
            is_server_streaming,
            options,
            comments,
            span: join_span(start, end),
        })
    }

    fn parse_enum(&mut self) -> Result<ast::Enum, ()> {
        let leading_comments = self.parse_leading_comments();

        let start = self.expect_eq(Token::Enum)?;

        let name = self.parse_ident()?;

        self.expect_eq(Token::LeftBrace)?;
        let comments = self.parse_trailing_comment(leading_comments);

        let mut values = Vec::new();
        let mut options = Vec::new();
        let mut reserved = Vec::new();

        let end = loop {
            match self.peek() {
                Some((Token::Option, _)) => {
                    options.push(self.parse_option()?);
                }
                Some((Token::Reserved, _)) => {
                    reserved.push(self.parse_reserved()?);
                }
                Some((Token::Semicolon, _)) => {
                    self.bump();
                    continue;
                }
                Some((Token::Ident(_), _)) => {
                    values.push(self.parse_enum_value()?);
                }
                Some((Token::RightBrace, span)) => {
                    self.bump();
                    break span;
                }
                _ => self.unexpected_token("an identifier, '}', 'reserved' or 'option'")?,
            };
        };

        Ok(ast::Enum {
            name,
            options,
            reserved,
            values,
            comments,
            span: join_span(start, end),
        })
    }

    fn parse_enum_value(&mut self) -> Result<ast::EnumValue, ()> {
        let leading_comments = self.parse_leading_comments();

        let name = self.parse_ident()?;

        self.expect_eq(Token::Equals)?;

        let value = self.parse_int()?;

        let options = match self.peek() {
            Some((Token::Semicolon, _)) => vec![],
            Some((Token::LeftBracket, _)) => self.parse_options_list()?,
            _ => self.unexpected_token("';' or '['")?,
        };

        let end = self.expect_eq(Token::Semicolon)?;
        let comments = self.parse_trailing_comment(leading_comments);

        Ok(ast::EnumValue {
            span: join_span(name.span.clone(), end),
            name,
            value,
            options,
            comments,
        })
    }

    fn parse_oneof(&mut self) -> Result<ast::Oneof, ()> {
        let leading_comments = self.parse_leading_comments();
        let start = self.expect_eq(Token::Oneof)?;

        let name = self.parse_ident()?;

        self.expect_eq(Token::LeftBrace)?;
        let comments = self.parse_trailing_comment(leading_comments);

        let mut fields = Vec::new();
        let mut options = Vec::new();

        let end = loop {
            match self.peek() {
                Some((tok, _)) if is_field_start_token(&tok) => fields.push(self.parse_field()?),
                Some((Token::Option, _)) => options.push(self.parse_option()?),
                Some((Token::Semicolon, _)) => {
                    self.bump();
                    continue;
                }
                Some((Token::RightBrace, span)) => {
                    self.bump();
                    break span;
                }
                _ => self.unexpected_token("a message field, option or '}'")?,
            }
        };

        Ok(ast::Oneof {
            name,
            fields,
            options,
            comments,
            span: join_span(start, end),
        })
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

    fn parse_field_type(&mut self, terminators: &[ExpectedToken]) -> Result<ast::Ty, ()> {
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
        let leading_comments = self.parse_leading_comments();
        let start = self.expect_eq(Token::Reserved)?;

        match self.peek() {
            Some((Token::IntLiteral(_) | Token::Minus, _)) => {
                let ranges = self.parse_reserved_ranges(&[ExpectedToken::SEMICOLON])?;
                let end = self.expect_eq(Token::Semicolon)?;
                let comments = self.parse_trailing_comment(leading_comments);
                Ok(ast::Reserved {
                    kind: ast::ReservedKind::Ranges(ranges),
                    comments,
                    span: join_span(start, end),
                })
            }
            Some((Token::StringLiteral(_), _)) => {
                let (names, end) = self.parse_reserved_names()?;
                let comments = self.parse_trailing_comment(leading_comments);
                Ok(ast::Reserved {
                    kind: ast::ReservedKind::Names(names),
                    comments,
                    span: join_span(start, end),
                })
            }
            _ => self.unexpected_token("a positive integer or string"),
        }
    }

    fn parse_extensions(&mut self) -> Result<ast::Extensions, ()> {
        let leading_comments = self.parse_leading_comments();
        let start = self.expect_eq(Token::Extensions)?;

        let ranges =
            self.parse_reserved_ranges(&[ExpectedToken::SEMICOLON, ExpectedToken::LEFT_BRACKET])?;

        let options = match self.peek() {
            Some((Token::Semicolon, _)) => vec![],
            Some((Token::LeftBracket, _)) => self.parse_options_list()?,
            _ => self.unexpected_token("';' or '['")?,
        };

        let end = self.expect_eq(Token::Semicolon)?;

        let comments = self.parse_trailing_comment(leading_comments);

        Ok(ast::Extensions {
            ranges,
            options,
            comments,
            span: join_span(start, end),
        })
    }

    fn parse_reserved_names(&mut self) -> Result<(Vec<ast::Ident>, Span), ()> {
        let mut names = vec![self.parse_ident_string()?];

        let end = loop {
            match self.peek() {
                Some((Token::Comma, _)) => {
                    self.bump();
                    names.push(self.parse_ident_string()?);
                }
                Some((Token::Semicolon, span)) => {
                    self.bump();
                    break span;
                }
                _ => self.unexpected_token("',' or ';'")?,
            }
        };

        Ok((names, end))
    }

    fn parse_ident_string(&mut self) -> Result<ast::Ident, ()> {
        let string = self.parse_string()?;
        if !is_valid_ident(&string.value) {
            self.add_error(ParseError::InvalidIdentifier {
                span: string.span.clone(),
            })
        }
        Ok(ast::Ident {
            value: string.value,
            span: string.span,
        })
    }

    fn parse_reserved_ranges(
        &mut self,
        terminators: &[ExpectedToken],
    ) -> Result<Vec<ast::ReservedRange>, ()> {
        let mut ranges = vec![self.parse_reserved_range()?];

        loop {
            match self.peek() {
                Some((Token::Comma, _)) => {
                    self.bump();
                    ranges.push(self.parse_reserved_range()?);
                    continue;
                }
                Some((tok, _)) if terminators.iter().any(|e| e.matches(&tok)) => break,
                _ => self.unexpected_token(fmt_expected(
                    once(ExpectedToken::Token(Token::Dot)).chain(terminators.iter().cloned()),
                ))?,
            }
        }

        Ok(ranges)
    }

    fn parse_reserved_range(&mut self) -> Result<ast::ReservedRange, ()> {
        let start = self.parse_int()?;

        let end = match self.peek() {
            Some((Token::To, _)) => {
                self.bump();
                match self.peek() {
                    Some((Token::IntLiteral(_) | Token::Minus, _)) => {
                        ast::ReservedRangeEnd::Int(self.parse_int()?)
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

    fn parse_options_list(&mut self) -> Result<Vec<ast::OptionBody>, ()> {
        self.expect_eq(Token::LeftBracket)?;

        let mut options =
            vec![self.parse_option_body(&[ExpectedToken::COMMA, ExpectedToken::RIGHT_BRACKET])?];
        loop {
            match self.peek() {
                Some((Token::Comma, _)) => {
                    self.bump();
                    options.push(self.parse_option_body(&[
                        ExpectedToken::COMMA,
                        ExpectedToken::RIGHT_BRACKET,
                    ])?);
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
        let leading_comments = self.parse_leading_comments();
        let start = self.expect_eq(Token::Option)?;

        let body = self.parse_option_body(&[ExpectedToken::SEMICOLON])?;

        let end = self.expect_eq(Token::Semicolon)?;
        let comments = self.parse_trailing_comment(leading_comments);

        Ok(ast::Option {
            comments,
            span: join_span(start, end),
            body,
        })
    }

    fn parse_option_body(&mut self, terminators: &[ExpectedToken]) -> Result<ast::OptionBody, ()> {
        let name = match self.peek() {
            Some((Token::LeftParen, _)) => {
                self.bump();
                let full_ident = self.parse_full_ident(&[ExpectedToken::RIGHT_PAREN])?;
                self.expect_eq(Token::RightParen)?;
                full_ident
            }
            Some((Token::Ident(value), span)) => {
                self.bump();
                ast::FullIdent::from(ast::Ident {
                    value: value.into_owned(),
                    span,
                })
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
                ast::Constant::String(ast::String {
                    value: value.into_owned(),
                    span,
                })
            }
            Some((Token::BoolLiteral(value), span)) => {
                self.bump();
                ast::Constant::Bool(ast::Bool { value, span })
            }
            _ => self.unexpected_token("a constant")?,
        };

        Ok(ast::OptionBody {
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

    fn parse_type_name(&mut self, terminators: &[ExpectedToken]) -> Result<ast::TypeName, ()> {
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

    fn parse_full_ident(&mut self, terminators: &[ExpectedToken]) -> Result<ast::FullIdent, ()> {
        let mut result = vec![self.parse_ident()?];

        loop {
            match self.peek() {
                Some((Token::Dot, _)) => {
                    self.bump();
                }
                Some((tok, _)) if terminators.iter().any(|e| e.matches(&tok)) => {
                    return Ok(result.into());
                }
                _ => self.unexpected_token(fmt_expected(
                    once(ExpectedToken::Token(Token::Dot)).chain(terminators.iter().cloned()),
                ))?,
            }

            result.push(self.parse_ident()?);
        }
    }

    fn parse_ident(&mut self) -> Result<ast::Ident, ()> {
        self.expect(
            |tok, span| tok.as_ident().map(|value| ast::Ident::new(value, span)),
            "an identifier",
        )
    }

    fn parse_int(&mut self) -> Result<ast::Int, ()> {
        let (negative, start) = match self.peek() {
            Some((Token::Minus, span)) => {
                self.bump();
                (true, Some(span))
            }
            _ => (false, None),
        };

        match self.peek() {
            Some((Token::IntLiteral(value), end)) => {
                self.bump();

                let span = match start {
                    None => end,
                    Some(start) => join_span(start, end),
                };

                Ok(ast::Int {
                    negative,
                    value,
                    span,
                })
            }
            _ => self.unexpected_token("an integer"),
        }
    }

    fn parse_string(&mut self) -> Result<ast::String, ()> {
        match self.peek() {
            Some((Token::StringLiteral(value), span)) => {
                self.bump();
                Ok(ast::String {
                    value: value.into_owned(),
                    span,
                })
            }
            _ => self.unexpected_token("a string literal"),
        }
    }

    fn parse_leading_comments(&mut self) -> (Vec<String>, Option<String>) {
        if self.peek.is_none() {
            self.peek();
        }
        self.comments.take()
    }

    fn parse_trailing_comment(
        &mut self,
        (leading_detached_comments, leading_comment): (Vec<String>, std::option::Option<String>),
    ) -> ast::Comments {
        if let Some((Token::Newline, _)) = self.peek_comments() {
            self.bump_comment();
        }

        let trailing_comment = if let Some((Token::Comment(comment), _)) = self.peek_comments() {
            self.bump_comment();

            if matches!(
                self.peek_comments(),
                Some((Token::Newline | Token::RightBrace, _)) | None
            ) {
                Some(comment.into())
            } else {
                self.comments.comment(comment.into());
                None
            }
        } else {
            None
        };

        ast::Comments {
            leading_detached_comments,
            leading_comment,
            trailing_comment,
        }
    }

    fn expect_eq(&mut self, t: Token) -> Result<Span, ()> {
        match self.peek() {
            Some((tok, _)) if tok == t => Ok(self.bump()),
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

    fn skip_until(&mut self, tokens: &[Token]) {
        let mut count = 0;
        while self.bump_if(|tok| !tokens.contains(tok)) {
            count += 1;
            assert!(count < 500);
        }
    }

    fn bump_if_eq(&mut self, t: Token) -> bool {
        self.bump_if(|tok| *tok == t)
    }

    fn bump_if(&mut self, mut f: impl FnMut(&Token) -> bool) -> bool {
        match self.peek() {
            Some((tok, _)) if f(&tok) => {
                self.bump();
                true
            }
            _ => false,
        }
    }

    fn bump(&mut self) -> Span {
        match self.bump_comment() {
            (Token::Comment(comment), span) => {
                self.comments.comment(comment.into());
                span
            }
            (Token::Newline, span) => {
                self.comments.newline();
                span
            }
            (_, span) => {
                self.comments.reset();
                span
            }
        }
    }

    fn bump_comment(&mut self) -> (Token<'a>, Span) {
        self.peek
            .take()
            .expect("called bump without peek returning Some()")
    }

    fn peek(&mut self) -> Option<(Token<'a>, Span)> {
        loop {
            match self.peek_comments() {
                Some((Token::Comment(_), _)) => {
                    self.bump();
                }
                Some((Token::Newline, _)) => {
                    self.bump();
                }
                tok => {
                    return tok;
                }
            }
        }
    }

    fn peek_comments(&mut self) -> Option<(Token<'a>, Span)> {
        if self.peek.is_none() {
            self.peek = self.next();
        }
        self.peek.clone()
    }

    fn next(&mut self) -> Option<(Token<'a>, Span)> {
        debug_assert!(self.peek.is_none());
        match self.lexer.next() {
            Some(Token::Error) => {
                self.comments.reset();
                self.add_error(ParseError::InvalidToken {
                    span: self.lexer.span(),
                });
                Some((Token::Error, self.lexer.span()))
            }
            Some(tok) => Some((tok, self.lexer.span())),
            None => None,
        }
    }

    fn unexpected_token<T>(&mut self, expected: impl ToString) -> Result<T, ()> {
        match self.peek() {
            Some((Token::Error, _)) => Err(()),
            Some((found, span)) => {
                self.add_error(ParseError::UnexpectedToken {
                    expected: expected.to_string(),
                    found: found.to_static(),
                    span,
                });
                Err(())
            }
            None => {
                self.eof(expected);
                Err(())
            }
        }
    }

    fn eof(&mut self, expected: impl ToString) {
        self.add_error(ParseError::UnexpectedEof {
            expected: expected.to_string(),
        });
    }

    fn add_error(&mut self, err: ParseError) {
        self.lexer.extras.errors.push(err);
    }
}

#[derive(Debug, Clone)]
enum ExpectedToken {
    Token(Token<'static>),
    Ident,
}

impl ExpectedToken {
    const COMMA: Self = ExpectedToken::Token(Token::Comma);
    const SEMICOLON: Self = ExpectedToken::Token(Token::Semicolon);
    const LEFT_BRACE: Self = ExpectedToken::Token(Token::LeftBrace);
    const LEFT_BRACKET: Self = ExpectedToken::Token(Token::LeftBracket);
    const RIGHT_PAREN: Self = ExpectedToken::Token(Token::RightParen);
    const RIGHT_BRACKET: Self = ExpectedToken::Token(Token::RightBracket);
    const RIGHT_ANGLE_BRACKET: Self = ExpectedToken::Token(Token::RightAngleBracket);

    fn matches(&self, t: &Token) -> bool {
        match self {
            ExpectedToken::Token(e) => e == t,
            ExpectedToken::Ident => t.as_ident().is_some(),
        }
    }
}

impl fmt::Display for ExpectedToken {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ExpectedToken::Token(e) => write!(f, "'{}'", e),
            ExpectedToken::Ident => write!(f, "an identifier"),
        }
    }
}

fn join_span(start: Span, end: Span) -> Span {
    start.start..end.end
}

fn is_field_start_token(tok: &Token) -> bool {
    matches!(
        tok,
        Token::Map
            | Token::Group
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

fn fmt_expected(ts: impl Iterator<Item = ExpectedToken>) -> String {
    let ts: Vec<_> = ts.collect();

    let mut s = String::with_capacity(32);
    write!(s, "{}", ts[0]).unwrap();
    if ts.len() > 1 {
        for t in &ts[1..][..ts.len() - 2] {
            s.push_str(", ");
            write!(s, "{}", t).unwrap();
        }
        s.push_str(" or ");
        write!(s, "{}", ts[ts.len() - 1]).unwrap();
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

fn is_valid_import(s: &str) -> bool {
    for component in s.split('/') {
        if component.is_empty() || component.contains('\\') || component == "." || component == ".."
        {
            return false;
        }
    }

    true
}
