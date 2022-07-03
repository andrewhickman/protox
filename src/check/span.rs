use prost_types::source_code_info::Location;

use crate::{ast, lines::LineResolver};

pub(super) struct SourceInfoPass {
    pub location: Vec<i32>,
    pub source_code_info: Vec<Location>,
    pub lines: LineResolver,
}

impl ast::Visitor for SourceInfoPass {
    fn visit_file(&mut self, file: &ast::File) {
        // name = 1;
        // package = 2;
        // dependency = 3;
        // public_dependency = 10;
        // weak_dependency = 11;
        // message_type = 4;
        // enum_type = 5;
        // service = 6;
        // extension = 7;
        // options = 8;
        // source_code_info = 9;
        // syntax = 12;

        file.visit(self)
    }

    fn visit_enum(&mut self, enu: &ast::Enum) {
        // name = 1;
        // value = 2;
        // options = 3;
        // reserved_range = 4;
        // reserved_name = 5;

        enu.visit(self)
    }

    fn visit_enum_value(&mut self, _: &ast::EnumValue) {
        // name = 1;
        // number = 2;
        // options = 3;
    }

    fn visit_message(&mut self, message: &ast::Message) {
        // name = 1;
        // field = 2;
        // extension = 6;
        // nested_type = 3;
        // enum_type = 4;
        // extension_range = 5;
        // options = 7;
        // oneof_decl = 8;
        // reserved_range = 9;
        // reserved_name = 10;

        message.body.visit(self)
    }

    fn visit_message_field(&mut self, field: &ast::MessageField) {
        field.visit(self)
    }

    fn visit_field(&mut self, _: &ast::Field) {
        // name = 1;
        // number = 3;
        // label = 4;
        // type = 5;
        // type_name = 6;
        // extendee = 2;
        // default_value = 7;
        // oneof_index = 9;
        // json_name = 10;
        // options = 8;
        // proto3_optional = 17;
    }

    fn visit_map(&mut self, _: &ast::Map) {}

    fn visit_group(&mut self, group: &ast::Group) {
        group.body.visit(self)
    }

    fn visit_oneof(&mut self, oneof: &ast::Oneof) {
        // name = 1;
        // options = 2;
        oneof.visit(self)
    }

    fn visit_extend(&mut self, extend: &ast::Extend) {
        extend.visit(self)
    }

    fn visit_service(&mut self, service: &ast::Service) {
        // name = 1;
        // method = 2;
        // options = 3;
        service.visit(self)
    }

    fn visit_method(&mut self, _: &ast::Method) {
        // name = 1;
        // input_type = 2;
        // output_type = 3;
        // options = 4;
        // client_streaming = 5;
        // server_streaming = 6
    }
}
