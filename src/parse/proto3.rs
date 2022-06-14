use super::Parser;
use crate::ast;

impl<'a> Parser<'a> {
    pub fn parse_proto3_file(&mut self) -> Result<ast::proto3::File, ()> {
        todo!()
    }
}