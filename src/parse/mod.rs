use std::{
    fmt::{self, Write},
    iter::once,
    path::Path,
};

use logos::{Lexer, Logos, Span};
use miette::Diagnostic;
use thiserror::Error;

mod comments;
mod lex;
#[cfg(test)]
mod tests;
mod text_format;

use self::comments::Comments;
use self::lex::Token;
use crate::{
    ast::{self, FieldLabel},
    join_span,
};

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
    #[error("string is not valid utf-8")]
    InvalidUtf8String {
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
        #[label("defined here…")]
        first: Span,
        #[label("…and again here")]
        second: Span,
    },
    #[error("whitespace is required between an integer literal and an identifier")]
    NoSpaceBetweenIntAndIdent {
        #[label("found here")]
        span: Span,
    },
    #[error("'#' comments are not allowed here")]
    HashCommentOutsideTextFormat {
        #[label("found here")]
        span: Span,
    },
    #[error("'f' suffix for float literals is not allowed")]
    FloatSuffixOutsideTextFormat {
        #[label("found here")]
        span: Span,
    },
    #[error("a colon is required between a field name and scalar value")]
    MissingColonForScalarTextFormatField {
        #[label("expected ':' after field name here")]
        field_name: Span,
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
}

#[derive(Debug, Clone)]
enum ExpectedToken {
    Token(Token<'static>),
    Ident,
}

enum Statement {
    Empty(Span),
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
        }
    }

    fn parse_file(&mut self) -> Result<ast::File, ()> {
        let mut file_span = Span::default();
        let mut syntax = ast::Syntax::default();
        let mut syntax_span = None;
        match self.peek() {
            Some((Token::Syntax, _)) => {
                let (parsed_syntax, span, comments) = self.parse_syntax()?;
                file_span = span.clone();
                syntax = parsed_syntax;
                syntax_span = Some((span, comments));
            }
            Some((_, span)) => {
                file_span = span;
            }
            None => (),
        }

        let mut package: Option<ast::Package> = None;
        let mut imports = Vec::new();
        let mut options = Vec::new();
        let mut items = Vec::new();

        loop {
            match self.parse_statement() {
                Ok(Some(statement)) => {
                    file_span = join_span(file_span, statement.span());
                    match statement {
                        Statement::Empty(_) => continue,
                        Statement::Package(new_package) => {
                            if let Some(existing_package) = &package {
                                self.add_error(ParseError::DuplicatePackage {
                                    first: existing_package.span.clone(),
                                    second: new_package.span,
                                })
                            } else {
                                package = Some(new_package);
                            }
                        }
                        Statement::Import(import) => imports.push(import),
                        Statement::Option(option) => options.push(option),
                        Statement::Message(message) => items.push(ast::FileItem::Message(message)),
                        Statement::Enum(enm) => items.push(ast::FileItem::Enum(enm)),
                        Statement::Service(service) => items.push(ast::FileItem::Service(service)),
                        Statement::Extend(extend) => items.push(ast::FileItem::Extend(extend)),
                    }
                }
                Ok(None) => break,
                Err(()) => {
                    debug_assert!(!self.lexer.extras.errors.is_empty());
                    self.skip_until(&[
                        Token::Enum,
                        Token::Extend,
                        Token::Import,
                        Token::Message,
                        Token::Option,
                        Token::Service,
                        Token::Package,
                    ])
                }
            }
        }

        Ok(ast::File {
            syntax,
            syntax_span,
            package,
            imports,
            options,
            items,
            span: file_span,
        })
    }

    fn parse_syntax(&mut self) -> Result<(ast::Syntax, Span, ast::Comments), ()> {
        let leading_comments = self.parse_leading_comments();

        let start = self.expect_eq(Token::Syntax)?;
        self.expect_eq(Token::Equals)?;

        let syntax = match self.peek() {
            Some((Token::StringLiteral(_), _)) => {
                let value = self.parse_string()?;
                match value.value.as_slice() {
                    b"proto2" => ast::Syntax::Proto2,
                    b"proto3" => ast::Syntax::Proto3,
                    _ => {
                        self.add_error(ParseError::UnknownSyntax {
                            span: value.span.clone(),
                        });
                        return Err(());
                    }
                }
            }
            _ => self.unexpected_token("an identifier or '('")?,
        };

        let end = self.expect_eq(Token::Semicolon)?;

        let comments = self.parse_trailing_comment(leading_comments);

        Ok((syntax, join_span(start, end), comments))
    }

    fn parse_statement(&mut self) -> Result<Option<Statement>, ()> {
        match self.peek() {
            Some((Token::Semicolon, span)) => {
                self.bump();
                Ok(Some(Statement::Empty(span)))
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
            Some((Token::Weak, span)) => {
                self.bump();
                Some((ast::ImportKind::Weak, span))
            }
            Some((Token::Public, span)) => {
                self.bump();
                Some((ast::ImportKind::Public, span))
            }
            Some((Token::StringLiteral(_), _)) => None,
            _ => self.unexpected_token("a string literal, 'public' or 'weak'")?,
        };

        let (value, value_span) = self.parse_utf8_string()?;
        if !is_valid_import(&value) {
            self.add_error(ParseError::InvalidImport {
                span: value_span.clone(),
            });
        }

        let end = self.expect_eq(Token::Semicolon)?;

        let comments = self.parse_trailing_comment(leading_comments);

        Ok(ast::Import {
            kind,
            value,
            value_span,
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
        let mut items = Vec::new();
        let mut options = Vec::new();
        let mut reserved = Vec::new();
        let mut extensions = Vec::new();

        let end = loop {
            match self.peek() {
                Some((tok, _)) if is_field_start_token(&tok) => {
                    items.push(ast::MessageItem::Field(self.parse_field()?))
                }
                Some((Token::Oneof, _)) => items.push(ast::MessageItem::Oneof(self.parse_oneof()?)),
                Some((Token::Enum, _)) => items.push(ast::MessageItem::Enum(self.parse_enum()?)),
                Some((Token::Message, _)) => {
                    items.push(ast::MessageItem::Message(self.parse_message()?))
                }
                Some((Token::Extend, _)) => {
                    items.push(ast::MessageItem::Extend(self.parse_extend()?))
                }
                Some((Token::Option, _)) => options.push(self.parse_option()?),
                Some((Token::Reserved, _)) => reserved.push(self.parse_reserved()?),
                Some((Token::Extensions, _)) => extensions.push(self.parse_extensions()?),
                Some((Token::Semicolon, _)) => {
                    self.bump();
                    continue;
                }
                Some((Token::RightBrace, _)) => break self.bump(),
                _ => self.unexpected_token(
                    "a message field, oneof, reserved range, enum, message, option or '}'",
                )?,
            }
        };

        Ok((
            ast::MessageBody {
                items,
                options,
                reserved,
                extensions,
            },
            end,
        ))
    }

    fn parse_field(&mut self) -> Result<ast::Field, ()> {
        let leading_comments = self.parse_leading_comments();

        let (label, start) = match self.peek() {
            Some((Token::Optional, span)) => {
                self.bump();
                (Some((FieldLabel::Optional, span.clone())), span)
            }
            Some((Token::Required, span)) => {
                self.bump();
                (Some((FieldLabel::Required, span.clone())), span)
            }
            Some((Token::Repeated, span)) => {
                self.bump();
                (Some((FieldLabel::Repeated, span.clone())), span)
            }
            Some((tok, span)) if is_field_start_token(&tok) => (None, span),
            _ => self.unexpected_token("a message field")?,
        };

        match self.peek() {
            Some((Token::Map, _)) => self.parse_map(leading_comments, start, label),
            Some((Token::Group, _)) => self.parse_group(leading_comments, start, label),
            _ => self.parse_normal_field(leading_comments, start, label),
        }
    }

    fn parse_map(
        &mut self,
        leading_comments: (Vec<String>, Option<String>),
        start: Span,
        label: Option<(FieldLabel, Span)>,
    ) -> Result<ast::Field, ()> {
        let ty_start = self.expect_eq(Token::Map)?;

        self.expect_eq(Token::LeftAngleBracket)?;
        let (key_ty, key_ty_span) = self.parse_field_type(&[ExpectedToken::COMMA])?;
        self.expect_eq(Token::Comma)?;
        let (value_ty, value_ty_span) =
            self.parse_field_type(&[ExpectedToken::RIGHT_ANGLE_BRACKET])?;
        let ty_end = self.expect_eq(Token::RightAngleBracket)?;

        let name = self.parse_ident()?;

        self.expect_eq(Token::Equals)?;

        let number = self.parse_int()?;

        let options = match self.peek() {
            Some((Token::LeftBracket, _)) => Some(self.parse_options_list()?),
            Some((Token::Semicolon, _)) => None,
            _ => self.unexpected_token("';' or '['")?,
        };

        let end = self.expect_eq(Token::Semicolon)?;
        let comments = self.parse_trailing_comment(leading_comments);

        Ok(ast::Field {
            label,
            kind: ast::FieldKind::Map {
                ty_span: join_span(ty_start, ty_end),
                key_ty,
                key_ty_span,
                value_ty,
                value_ty_span,
            },
            name,
            number,
            options,
            comments,
            span: join_span(start, end),
        })
    }

    fn parse_group(
        &mut self,
        leading_comments: (Vec<String>, Option<String>),
        start: Span,
        label: Option<(FieldLabel, Span)>,
    ) -> Result<ast::Field, ()> {
        let ty_span = self.expect_eq(Token::Group)?;

        let name = self.parse_ident()?;
        if !is_valid_group_name(&name.value) {
            self.add_error(ParseError::InvalidGroupName {
                span: name.span.clone(),
            });
        }

        self.expect_eq(Token::Equals)?;

        let number = self.parse_int()?;

        let options = match self.peek() {
            Some((Token::LeftBracket, _)) => Some(self.parse_options_list()?),
            Some((Token::LeftBrace, _)) => None,
            _ => self.unexpected_token("'{' or '['")?,
        };

        self.expect_eq(Token::LeftBrace)?;

        let comments = self.parse_trailing_comment(leading_comments);

        let (body, end) = self.parse_message_body()?;

        Ok(ast::Field {
            label,
            options,
            name,
            number,
            kind: ast::FieldKind::Group { ty_span, body },
            comments,
            span: join_span(start, end),
        })
    }

    fn parse_normal_field(
        &mut self,
        leading_comments: (Vec<String>, Option<String>),
        start: Span,
        label: Option<(FieldLabel, Span)>,
    ) -> Result<ast::Field, ()> {
        let (ty, ty_span) = self.parse_field_type(&[ExpectedToken::Ident])?;

        let name = self.parse_ident()?;

        self.expect_eq(Token::Equals)?;

        let number = self.parse_int()?;

        let options = match self.peek() {
            Some((Token::LeftBracket, _)) => Some(self.parse_options_list()?),
            Some((Token::Semicolon, _)) => None,
            _ => self.unexpected_token("';' or '['")?,
        };

        let end = self.expect_eq(Token::Semicolon)?;
        let comments = self.parse_trailing_comment(leading_comments);

        Ok(ast::Field {
            label,
            kind: ast::FieldKind::Normal { ty, ty_span },
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
                Some((Token::RightBrace, _)) => break self.bump(),
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
                Some((Token::RightBrace, _)) => break self.bump(),
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

        let client_streaming = match self.peek() {
            Some((Token::Stream, _)) => Some(self.bump()),
            Some((Token::Dot | Token::Ident(_), _)) => None,
            _ => self.unexpected_token("'stream' or a type name")?,
        };

        let input_ty = self.parse_type_name(&[ExpectedToken::RIGHT_PAREN])?;

        self.expect_eq(Token::RightParen)?;
        self.expect_eq(Token::Returns)?;
        self.expect_eq(Token::LeftParen)?;

        let server_streaming = match self.peek() {
            Some((Token::Stream, _)) => Some(self.bump()),
            Some((Token::Dot | Token::Ident(_), _)) => None,
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
                        Some((Token::RightBrace, _)) => break self.bump(),
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
            client_streaming,
            output_ty,
            server_streaming,
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
                Some((Token::RightBrace, _)) => break self.bump(),
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

        let number = self.parse_int()?;

        let options = match self.peek() {
            Some((Token::LeftBracket, _)) => Some(self.parse_options_list()?),
            Some((Token::Semicolon, _)) => None,
            _ => self.unexpected_token("';' or '['")?,
        };

        let end = self.expect_eq(Token::Semicolon)?;
        let comments = self.parse_trailing_comment(leading_comments);

        Ok(ast::EnumValue {
            span: join_span(name.span.clone(), end),
            name,
            number,
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
                Some((Token::RightBrace, _)) => break self.bump(),
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

    fn parse_field_type(&mut self, terminators: &[ExpectedToken]) -> Result<(ast::Ty, Span), ()> {
        let scalar_ty = match self.peek() {
            Some((Token::Double, span)) => (ast::Ty::Double, span),
            Some((Token::Float, span)) => (ast::Ty::Float, span),
            Some((Token::Int32, span)) => (ast::Ty::Int32, span),
            Some((Token::Int64, span)) => (ast::Ty::Int64, span),
            Some((Token::Uint32, span)) => (ast::Ty::Uint32, span),
            Some((Token::Uint64, span)) => (ast::Ty::Uint64, span),
            Some((Token::Sint32, span)) => (ast::Ty::Sint32, span),
            Some((Token::Sint64, span)) => (ast::Ty::Sint64, span),
            Some((Token::Fixed32, span)) => (ast::Ty::Fixed32, span),
            Some((Token::Fixed64, span)) => (ast::Ty::Fixed64, span),
            Some((Token::Sfixed32, span)) => (ast::Ty::Sfixed32, span),
            Some((Token::Sfixed64, span)) => (ast::Ty::Sfixed64, span),
            Some((Token::Bool, span)) => (ast::Ty::Bool, span),
            Some((Token::String, span)) => (ast::Ty::String, span),
            Some((Token::Bytes, span)) => (ast::Ty::Bytes, span),
            Some((Token::Dot | Token::Ident(_), _)) => {
                let type_name = self.parse_type_name(terminators)?;
                let span = type_name.span();
                return Ok((ast::Ty::Named(type_name), span));
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
            Some((Token::LeftBracket, _)) => Some(self.parse_options_list()?),
            Some((Token::Semicolon, _)) => None,
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
                Some((Token::Semicolon, _)) => break self.bump(),
                _ => self.unexpected_token("',' or ';'")?,
            }
        };

        Ok((names, end))
    }

    fn parse_ident_string(&mut self) -> Result<ast::Ident, ()> {
        let (value, span) = self.parse_utf8_string()?;
        if !is_valid_ident(&value) {
            self.add_error(ParseError::InvalidIdentifier { span: span.clone() })
        }
        Ok(ast::Ident { value, span })
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
                    Some((Token::Max, span)) => {
                        self.bump();
                        ast::ReservedRangeEnd::Max(span)
                    }
                    _ => self.unexpected_token("an integer or 'max'")?,
                }
            }
            Some((Token::Comma | Token::Semicolon, _)) => ast::ReservedRangeEnd::None,
            _ => self.unexpected_token("'to', ',' or ';'")?,
        };

        Ok(ast::ReservedRange { start, end })
    }

    fn parse_options_list(&mut self) -> Result<ast::OptionList, ()> {
        let start = self.expect_eq(Token::LeftBracket)?;

        let mut options =
            vec![self.parse_option_body(&[ExpectedToken::COMMA, ExpectedToken::RIGHT_BRACKET])?];
        let end = loop {
            match self.peek() {
                Some((Token::Comma, _)) => {
                    self.bump();
                    options.push(self.parse_option_body(&[
                        ExpectedToken::COMMA,
                        ExpectedToken::RIGHT_BRACKET,
                    ])?);
                }
                Some((Token::RightBracket, _)) => break self.bump(),
                _ => self.unexpected_token("',' or ']'")?,
            }
        };

        Ok(ast::OptionList {
            span: join_span(start, end),
            options,
        })
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
        let mut name = vec![self.parse_option_name_part()?];

        loop {
            match self.peek() {
                Some((Token::Dot, _)) => {
                    self.bump();
                    name.push(self.parse_option_name_part()?);
                }
                Some((Token::Equals, _)) => {
                    self.bump();
                    break;
                }
                _ => self.unexpected_token("'=' or '.'")?,
            }
        }

        let value = match self.peek() {
            Some((Token::Ident(_), _)) => {
                ast::OptionValue::FullIdent(self.parse_full_ident(terminators)?)
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
            Some((Token::StringLiteral(_), _)) => ast::OptionValue::String(self.parse_string()?),
            Some((Token::BoolLiteral(value), span)) => {
                self.bump();
                ast::OptionValue::Bool(ast::Bool { value, span })
            }
            Some((Token::LeftBrace, start)) => {
                self.bump();
                let value = self.parse_text_format_message(&[ExpectedToken::RIGHT_BRACE])?;
                let end = self.expect_eq(Token::RightBrace)?;
                ast::OptionValue::Aggregate(value, join_span(start, end))
            },
            _ => self.unexpected_token("a constant")?,
        };

        Ok(ast::OptionBody { name, value })
    }

    fn parse_option_name_part(&mut self) -> Result<ast::OptionNamePart, ()> {
        match self.peek() {
            Some((Token::Ident(_), _)) => Ok(ast::OptionNamePart::Ident(self.parse_ident()?)),
            Some((Token::LeftParen, start)) => {
                self.bump();
                let ident = self.parse_full_ident(&[ExpectedToken::RIGHT_PAREN])?;
                let end = self.expect_eq(Token::RightParen)?;
                Ok(ast::OptionNamePart::Extension(ident, join_span(start, end)))
            }
            _ => self.unexpected_token("an identifier or '('"),
        }
    }

    fn parse_int_or_float(&mut self, negate: bool) -> Result<ast::OptionValue, ()> {
        match self.peek() {
            Some((Token::IntLiteral(value), span)) => {
                self.bump();
                Ok(ast::OptionValue::Int(ast::Int {
                    value,
                    span,
                    negative: negate,
                }))
            }
            Some((Token::FloatLiteral(value), span)) => {
                self.bump();
                Ok(ast::OptionValue::Float(ast::Float {
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

    fn parse_utf8_string(&mut self) -> Result<(String, Span), ()> {
        let bytes = self.parse_string()?;

        match bytes.into_utf8() {
            Ok(string) => Ok(string),
            Err(bytes) => {
                self.add_error(ParseError::InvalidUtf8String {
                    span: bytes.span.clone(),
                });
                Ok((
                    String::from_utf8_lossy(&bytes.value).into_owned(),
                    bytes.span,
                ))
            }
        }
    }

    fn parse_string(&mut self) -> Result<ast::String, ()> {
        let mut result = match self.peek() {
            Some((Token::StringLiteral(value), span)) => {
                self.bump();
                Ok(ast::String {
                    value: value.into_owned(),
                    span,
                })
            }
            _ => self.unexpected_token("a string literal")?,
        }?;

        while let Some((Token::StringLiteral(value), span)) = self.peek() {
            self.bump();
            result.value.extend(value.as_ref());
            result.span = join_span(result.span.clone(), span);
        }

        Ok(result)
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
        let mut on_new_line = false;
        if let Some((Token::Newline, _)) = self.peek_comments() {
            self.bump_comment();
            on_new_line = true;
        }

        let trailing_comment = if let Some((Token::Comment(comment), _)) = self.peek_comments() {
            self.bump_comment();

            if !on_new_line
                || matches!(
                    self.peek_comments(),
                    Some((Token::Newline | Token::RightBrace, _)) | None
                )
            {
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
                Some((Token::Comment(_) | Token::Newline, _)) => {
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

impl ExpectedToken {
    const COMMA: Self = ExpectedToken::Token(Token::Comma);
    const SEMICOLON: Self = ExpectedToken::Token(Token::Semicolon);
    const FORWARD_SLASH: Self = ExpectedToken::Token(Token::ForwardSlash);
    const LEFT_BRACE: Self = ExpectedToken::Token(Token::LeftBrace);
    const LEFT_BRACKET: Self = ExpectedToken::Token(Token::LeftBracket);
    const RIGHT_PAREN: Self = ExpectedToken::Token(Token::RightParen);
    const RIGHT_BRACKET: Self = ExpectedToken::Token(Token::RightBracket);
    const RIGHT_BRACE: Self = ExpectedToken::Token(Token::RightBrace);
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

impl Statement {
    fn span(&self) -> Span {
        match self {
            Statement::Empty(span) => span.clone(),
            Statement::Package(package) => package.span.clone(),
            Statement::Import(import) => import.span.clone(),
            Statement::Option(option) => option.span.clone(),
            Statement::Message(message) => message.span.clone(),
            Statement::Enum(enu) => enu.span.clone(),
            Statement::Service(service) => service.span.clone(),
            Statement::Extend(extend) => extend.span.clone(),
        }
    }
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

    !Path::new(s).is_absolute()
}
