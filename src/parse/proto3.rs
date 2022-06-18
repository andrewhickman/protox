use super::{lex::Token, Parser};
use crate::ast::proto3 as ast3;

impl<'a> Parser<'a> {
    pub fn parse_proto3_file(&mut self) -> Result<ast3::File, ()> {
        let mut packages = Vec::new();
        let mut imports = Vec::new();
        let mut options = Vec::new();
        let mut definitions = Vec::new();

        loop {
            match self.parse_proto3_statement() {
                Ok(Some(ast3::Statement::Empty)) => continue,
                Ok(Some(ast3::Statement::Package(package))) => packages.push(package),
                Ok(Some(ast3::Statement::Import(import))) => imports.push(import),
                Ok(Some(ast3::Statement::Option(option))) => options.push(option),
                Ok(Some(ast3::Statement::Definition(definition))) => definitions.push(definition),
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

        Ok(ast3::File {
            packages,
            imports,
            options,
            definitions,
        })
    }

    pub fn parse_proto3_statement(&mut self) -> Result<Option<ast3::Statement>, ()> {
        match self.peek() {
            Some((Token::Semicolon, _)) => {
                self.bump();
                Ok(Some(ast3::Statement::Empty))
            }
            Some((Token::Import, _)) => Ok(Some(ast3::Statement::Import(self.parse_import()?))),
            Some((Token::Package, _)) => Ok(Some(ast3::Statement::Package(self.parse_package()?))),
            Some((Token::Option, _)) => Ok(Some(ast3::Statement::Option(self.parse_option()?))),
            Some((Token::Extend, _)) => Ok(Some(ast3::Statement::Definition(
                ast3::Definition::Extension(self.parse_proto3_extension()?),
            ))),
            Some((Token::Message, _)) => Ok(Some(ast3::Statement::Definition(
                ast3::Definition::Message(self.parse_proto3_message()?),
            ))),
            Some((Token::Enum, _)) => Ok(Some(ast3::Statement::Definition(
                ast3::Definition::Enum(self.parse_enum()?),
            ))),
            Some((Token::Service, _)) => Ok(Some(ast3::Statement::Definition(
                ast3::Definition::Service(self.parse_service()?),
            ))),
            None => Ok(None),
            _ => self.unexpected_token(
                "'enum', 'extend', 'import', 'message', 'option', 'service', 'package' or ';'",
            ),
        }
    }

    pub fn parse_proto3_extension(&mut self) -> Result<ast3::Extension, ()> {
        todo!()
    }

    pub fn parse_proto3_message(&mut self) -> Result<ast3::Message, ()> {
        todo!()
    }
}
