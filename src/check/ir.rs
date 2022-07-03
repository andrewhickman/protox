use crate::ast;

/// A protobuf file structure, with synthetic oneofs, groups and map messages expanded.
pub(crate) struct File<'a> {
    ast: &'a ast::File,
    messages: Vec<Message<'a>>,
    extends: Vec<Extend<'a>>
}

pub(crate) struct Message<'a> {
    ast: MessageAst<'a>,
    oneofs: Vec<Oneof<'a>>
}

enum MessageAst<'a> {
    Message(&'a ast::Message),
    Group(&'a ast::Group),
    Map(&'a ast::Map),
}

pub(crate) struct Oneof<'a> {
    ast: Option<&'a ast::Oneof>,
}

pub(crate) struct Extend<'a> {
    ast: &'a ast::Extend,
    index: usize,
}
