use crate::ast;

pub(crate) trait Visitor {
    fn visit_file(&mut self, file: &ast::File) {
        file.visit(self)
    }

    fn visit_enum(&mut self, enu: &ast::Enum) {
        enu.visit(self)
    }

    fn visit_enum_value(&mut self, _: &ast::EnumValue) {}

    fn visit_message(&mut self, message: &ast::Message) {
        message.body.visit(self)
    }

    fn visit_message_field(&mut self, field: &ast::MessageField) {
        field.visit(self)
    }

    fn visit_field(&mut self, _: &ast::Field) {}

    fn visit_map(&mut self, _: &ast::Map) {}

    fn visit_group(&mut self, group: &ast::Group) {
        group.body.visit(self)
    }

    fn visit_oneof(&mut self, oneof: &ast::Oneof) {
        oneof.visit(self)
    }

    fn visit_extend(&mut self, extend: &ast::Extend) {
        extend.visit(self)
    }

    fn visit_service(&mut self, service: &ast::Service) {
        service.visit(self)
    }

    fn visit_method(&mut self, _: &ast::Method) {}
}

impl ast::File {
    pub fn visit<V: Visitor + ?Sized>(&self, visitor: &mut V) {
        for item in &self.items {
            match item {
                ast::FileItem::Enum(e) => visitor.visit_enum(e),
                ast::FileItem::Message(m) => visitor.visit_message(m),
                ast::FileItem::Extend(e) => visitor.visit_extend(e),
                ast::FileItem::Service(s) => visitor.visit_service(s),
            }
        }
    }
}

impl ast::Enum {
    pub fn visit<V: Visitor + ?Sized>(&self, visitor: &mut V) {
        for value in &self.values {
            visitor.visit_enum_value(value)
        }
    }
}

impl ast::MessageBody {
    pub fn visit<V: Visitor + ?Sized>(&self, visitor: &mut V) {
        for item in &self.items {
            match item {
                ast::MessageItem::Field(f) => visitor.visit_message_field(f),
                ast::MessageItem::Enum(e) => visitor.visit_enum(e),
                ast::MessageItem::Message(m) => visitor.visit_message(m),
                ast::MessageItem::Extend(e) => visitor.visit_extend(e),
            }
        }
    }
}

impl ast::MessageField {
    pub fn visit<V: Visitor + ?Sized>(&self, visitor: &mut V) {
        match self {
            ast::MessageField::Field(f) => visitor.visit_field(f),
            ast::MessageField::Group(g) => visitor.visit_group(g),
            ast::MessageField::Map(m) => visitor.visit_map(m),
            ast::MessageField::Oneof(o) => visitor.visit_oneof(o),
        }
    }
}

impl ast::Oneof {
    pub fn visit<V: Visitor + ?Sized>(&self, visitor: &mut V) {
        for field in &self.fields {
            visitor.visit_message_field(field)
        }
    }
}

impl ast::Extend {
    pub fn visit<V: Visitor + ?Sized>(&self, visitor: &mut V) {
        for field in &self.fields {
            visitor.visit_message_field(field)
        }
    }
}

impl ast::Service {
    pub fn visit<V: Visitor + ?Sized>(&self, visitor: &mut V) {
        for method in &self.methods {
            visitor.visit_method(method)
        }
    }
}
