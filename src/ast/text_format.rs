use logos::Span;

use super::{Ident, FullIdent};


pub(crate) struct Message {
    pub fields: Vec<Field>,
    pub span: Span,
}

pub(crate) struct Field {
    pub name: FieldName,
    pub value: FieldValue,
}

pub(crate) enum FieldName {
    Ident(Ident),
    Extension(FullIdent),
    Any(FullIdent, FullIdent),
}

pub(crate) enum FieldValue {
    Message(Message),
    MessageList(Vec<Message>),
    Scalar(Scalar),
    ScalarList(Vec<Scalar>),
}

pub(crate) enum Scalar {

}