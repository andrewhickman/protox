use logos::Span;

use super::{Float, FullIdent, Ident, Int, String};

pub(crate) struct Message {
    pub fields: Vec<Field>,
}

pub(crate) struct Field {
    pub name: FieldName,
    pub value: FieldValue,
}

pub(crate) enum FieldName {
    Ident(Ident),
    Extension(FullIdent, Span),
    Any(FullIdent, FullIdent, Span),
}

pub(crate) enum FieldValue {
    Message(Message, Span),
    MessageList(Vec<(Message, Span)>, Span),
    Scalar(Scalar),
    ScalarList(Vec<Scalar>, Span),
}

pub(crate) enum Scalar {
    String(String),
    Float(Float),
    Ident {
        negative: bool,
        ident: Ident,
        span: Span,
    },
    Int(Int),
}

impl FieldName {
    pub fn span(&self) -> Span {
        match self {
            FieldName::Ident(ident) => ident.span.clone(),
            FieldName::Extension(_, span) => span.clone(),
            FieldName::Any(_, _, span) => span.clone(),
        }
    }
}
