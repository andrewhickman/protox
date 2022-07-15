use super::*;

impl<'a> Parser<'a> {
    pub(super) fn parse_text_format_message(
        &mut self,
        terminators: &[ExpectedToken],
    ) -> Result<ast::text_format::Message, ()> {
        self.lexer.extras.text_format_mode = true;
        let result = self.parse_text_format_message_inner(terminators);
        self.lexer.extras.text_format_mode = false;
        result
    }

    fn parse_text_format_message_inner(
        &mut self,
        terminators: &[ExpectedToken],
    ) -> Result<ast::text_format::Message, ()> {
        let mut fields = Vec::new();

        loop {
            match self.peek() {
                Some((Token::Ident(_) | Token::LeftBracket, _)) => {
                    fields.push(self.parse_text_format_field()?)
                }
                Some((tok, _)) if terminators.iter().any(|e| e.matches(&tok)) => break,
                None => break,
                _ => self.unexpected_token(fmt_expected(
                    once(ExpectedToken::Ident)
                        .chain(terminators.iter().cloned())
                        .chain(once(ExpectedToken::LEFT_BRACKET)),
                ))?,
            }
        }

        Ok(ast::text_format::Message { fields })
    }

    #[cfg(test)]
    pub(super) fn parse_text_format_message_test(&mut self) -> Result<ast::text_format::Message, ()> {
        self.parse_text_format_message(&[])
    }

    fn parse_text_format_field(&mut self) -> Result<ast::text_format::Field, ()> {
        let name = self.parse_text_format_field_name()?;

        let colon = match self.peek() {
            Some((Token::Colon, _)) => Some(self.bump()),
            Some((tok, _)) if is_text_format_field_value_start_token(&tok) => None,
            _ => self.unexpected_token("':' or a message value")?,
        };

        let value = self.parse_text_format_field_value()?;

        if colon.is_none()
            && matches!(
                value,
                ast::text_format::FieldValue::Scalar(_)
                    | ast::text_format::FieldValue::ScalarList(..)
            )
        {
            self.add_error(ParseError::MissingColonForScalarTextFormatField {
                field_name: name.span(),
            });
        }

        let end = match self.peek() {
            Some((Token::Comma | Token::Semicolon, _)) => self.bump(),
            _ => value.span(),
        };

        Ok(ast::text_format::Field {
            value,
            span: join_span(name.span(), end),
            name,
        })
    }

    fn parse_text_format_field_name(&mut self) -> Result<ast::text_format::FieldName, ()> {
        match self.peek() {
            Some((Token::Ident(_), _)) => {
                Ok(ast::text_format::FieldName::Ident(self.parse_ident()?))
            }
            Some((Token::LeftBracket, start)) => {
                self.bump();

                let name_or_domain = self.parse_full_ident(&[
                    ExpectedToken::RIGHT_BRACKET,
                    ExpectedToken::FORWARD_SLASH,
                ])?;
                match self.peek() {
                    Some((Token::RightBracket, end)) => {
                        self.bump();
                        Ok(ast::text_format::FieldName::Extension(
                            name_or_domain,
                            join_span(start, end),
                        ))
                    }
                    Some((Token::ForwardSlash, _)) => {
                        let type_name = self.parse_full_ident(&[ExpectedToken::RIGHT_BRACKET])?;
                        let end = self.expect_eq(Token::RightBracket)?;
                        Ok(ast::text_format::FieldName::Any(
                            name_or_domain,
                            type_name,
                            join_span(start, end),
                        ))
                    }
                    _ => self.unexpected_token("']' or '/'")?,
                }
            }
            _ => self.unexpected_token("an identifier or '['")?,
        }
    }

    fn parse_text_format_field_value(&mut self) -> Result<ast::text_format::FieldValue, ()> {
        match self.peek() {
            Some((
                Token::Minus
                | Token::Ident(_)
                | Token::StringLiteral(_)
                | Token::FloatLiteral(_)
                | Token::IntLiteral(_),
                _,
            )) => Ok(ast::text_format::FieldValue::Scalar(
                self.parse_text_format_scalar_value()?,
            )),
            Some((Token::LeftBrace | Token::LeftAngleBracket, _)) => {
                let (message, span) = self.parse_text_format_message_value()?;
                Ok(ast::text_format::FieldValue::Message(message, span))
            }
            Some((Token::LeftBracket, start)) => {
                self.bump();
                match self.peek() {
                    Some((
                        Token::Minus
                        | Token::Ident(_)
                        | Token::StringLiteral(_)
                        | Token::FloatLiteral(_)
                        | Token::IntLiteral(_),
                        _,
                    )) => {
                        let list = self.parse_text_format_scalar_list()?;
                        let end = self.expect_eq(Token::RightBracket)?;
                        Ok(ast::text_format::FieldValue::ScalarList(
                            list,
                            join_span(start, end),
                        ))
                    }
                    Some((Token::LeftBrace | Token::LeftAngleBracket, _)) => {
                        let list = self.parse_text_format_message_list()?;
                        let end = self.expect_eq(Token::RightBracket)?;
                        Ok(ast::text_format::FieldValue::MessageList(
                            list,
                            join_span(start, end),
                        ))
                    }
                    _ => self.unexpected_token("an identifier, string, number or message")?,
                }
            }
            _ => self.unexpected_token("an identifier, string, number, message or array")?,
        }
    }

    fn parse_text_format_scalar_value(&mut self) -> Result<ast::text_format::Scalar, ()> {
        let (negative, start) = match self.peek() {
            Some((Token::Minus, _)) => (true, self.bump()),
            Some((
                Token::Ident(_)
                | Token::StringLiteral(_)
                | Token::FloatLiteral(_)
                | Token::IntLiteral(_),
                span,
            )) => (false, span),
            _ => self.unexpected_token("an identifier, string or number")?,
        };

        match self.peek() {
            Some((Token::StringLiteral(_), _)) => {
                if negative {
                    let _: Result<(), ()> = self.unexpected_token("an identifier or number");
                }

                Ok(ast::text_format::Scalar::String(self.parse_string()?))
            }
            Some((Token::Ident(_), _)) => {
                let ident = self.parse_ident()?;
                Ok(ast::text_format::Scalar::Ident {
                    negative,
                    span: join_span(start, ident.span.clone()),
                    ident,
                })
            }
            Some((Token::IntLiteral(value), end)) => {
                self.bump();

                Ok(ast::text_format::Scalar::Int(ast::Int {
                    negative,
                    value,
                    span: join_span(start, end),
                }))
            }
            Some((Token::FloatLiteral(value), end)) => {
                self.bump();

                Ok(ast::text_format::Scalar::Float(ast::Float {
                    value: if negative { -value } else { value },
                    span: join_span(start, end),
                }))
            }
            _ => self.unexpected_token("an identifier, string or number")?,
        }
    }

    fn parse_text_format_message_value(&mut self) -> Result<(ast::text_format::Message, Span), ()> {
        let (delimiter, start) = match self.peek() {
            Some((Token::LeftBrace, _)) => (Token::RightBrace, self.bump()),
            Some((Token::LeftAngleBracket, _)) => (Token::RightAngleBracket, self.bump()),
            _ => self.unexpected_token("'{' or '<'")?,
        };

        let message = self.parse_text_format_message(&[ExpectedToken::Token(delimiter.clone())])?;

        let end = self.expect_eq(delimiter)?;

        Ok((message, join_span(start, end)))
    }

    fn parse_text_format_scalar_list(&mut self) -> Result<Vec<ast::text_format::Scalar>, ()> {
        let mut values = vec![self.parse_text_format_scalar_value()?];

        loop {
            match self.peek() {
                Some((Token::Comma, _)) => {
                    self.bump();
                    values.push(self.parse_text_format_scalar_value()?);
                }
                Some((Token::RightBracket, _)) => break,
                _ => self.unexpected_token("',' or ']'")?,
            }
        }

        Ok(values)
    }

    fn parse_text_format_message_list(
        &mut self,
    ) -> Result<Vec<(ast::text_format::Message, Span)>, ()> {
        let mut values = vec![self.parse_text_format_message_value()?];

        loop {
            match self.peek() {
                Some((Token::Comma, _)) => {
                    self.bump();
                    values.push(self.parse_text_format_message_value()?);
                }
                Some((Token::RightBracket, _)) => break,
                _ => self.unexpected_token("',' or ']'")?,
            }
        }

        Ok(values)
    }
}

fn is_text_format_field_value_start_token(tok: &Token) -> bool {
    matches!(
        tok,
        Token::LeftBrace
            | Token::LeftAngleBracket
            | Token::Minus
            | Token::Ident(_)
            | Token::StringLiteral(_)
            | Token::FloatLiteral(_)
            | Token::IntLiteral(_)
    )
}
