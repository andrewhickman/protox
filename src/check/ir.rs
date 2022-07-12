use std::borrow::Cow;

use logos::Span;

use crate::{
    ast::{self, MessageBody},
    index_to_i32,
};

/// A protobuf file structure, with synthetic oneofs, groups and map messages expanded.
#[derive(Debug)]
pub(crate) struct File<'a> {
    pub ast: &'a ast::File,
    pub messages: Vec<Message<'a>>,
}

#[derive(Debug)]
pub(crate) struct Message<'a> {
    pub ast: MessageSource<'a>,
    pub fields: Vec<Field<'a>>,
    pub messages: Vec<Message<'a>>,
    pub oneofs: Vec<Oneof<'a>>,
}

#[derive(Debug)]
pub(crate) enum MessageSource<'a> {
    Message(&'a ast::Message),
    Group(&'a ast::Field, &'a MessageBody),
    Map(&'a ast::Field),
}

#[derive(Debug)]
pub(crate) struct Field<'a> {
    pub ast: FieldSource<'a>,
    pub oneof_index: Option<i32>,
    pub is_synthetic_oneof: bool,
}

#[derive(Debug)]
pub(crate) enum FieldSource<'a> {
    Field(&'a ast::Field),
    MapKey(&'a ast::Ty, Span),
    MapValue(&'a ast::Ty, Span),
}

#[derive(Debug)]
pub(crate) struct Oneof<'a> {
    pub ast: OneofSource<'a>,
}

#[derive(Debug)]
pub(crate) enum OneofSource<'a> {
    Oneof(&'a ast::Oneof),
    Field(&'a ast::Field),
}

impl<'a> File<'a> {
    pub(crate) fn build(ast: &'a ast::File) -> Self {
        let mut messages = Vec::new();

        for item in &ast.items {
            match item {
                ast::FileItem::Message(message) => {
                    build_message(ast.syntax, message, &mut messages)
                }
                ast::FileItem::Extend(extend) => build_extend(ast.syntax, extend, &mut messages),
                ast::FileItem::Enum(_) | ast::FileItem::Service(_) => continue,
            }
        }

        File { ast, messages }
    }
}

impl<'a> MessageSource<'a> {
    pub fn name(&self) -> Cow<'_, str> {
        match self {
            MessageSource::Message(message) => Cow::Borrowed(message.name.value.as_str()),
            MessageSource::Group(group, _) => Cow::Borrowed(group.name.value.as_str()),
            MessageSource::Map(map) => Cow::Owned(map.map_message_name()),
        }
    }

    pub fn name_span(&self) -> Span {
        match self {
            MessageSource::Message(message) => message.name.span.clone(),
            MessageSource::Group(group, _) => group.name.span.clone(),
            MessageSource::Map(map) => map.name.span.clone(),
        }
    }

    pub fn body(&self) -> Option<&'a ast::MessageBody> {
        match self {
            MessageSource::Message(message) => Some(&message.body),
            MessageSource::Group(_, body) => Some(body),
            MessageSource::Map(_) => None,
        }
    }
}

impl<'a> FieldSource<'a> {
    pub fn name(&self) -> Cow<'_, str> {
        match self {
            FieldSource::Field(field) => field.field_name(),
            FieldSource::MapKey(_, _) => Cow::Borrowed("key"),
            FieldSource::MapValue(_, _) => Cow::Borrowed("value"),
        }
    }

    pub fn name_span(&self) -> Span {
        match self {
            FieldSource::Field(field) => field.name.span.clone(),
            FieldSource::MapKey(_, key_span) => key_span.clone(),
            FieldSource::MapValue(_, value_span) => value_span.clone(),
        }
    }

    pub fn number(&self) -> ast::Int {
        match self {
            FieldSource::Field(field) => field.number.clone(),
            FieldSource::MapKey(_, key_span) => ast::Int {
                negative: false,
                value: 1,
                span: key_span.clone()
            },
            FieldSource::MapValue(_, value_span) => ast::Int {
                negative: false,
                value: 2,
                span: value_span.clone(),
            },
        }
    }

    pub fn ty(&self) -> ast::Ty {
        match self {
            FieldSource::Field(field) => match &field.kind {
                ast::FieldKind::Normal { ty, .. } => ty.clone(),
                ast::FieldKind::Group { .. } => ast::Ty::Named(ast::TypeName {
                    leading_dot: None,
                    name: ast::FullIdent::from(field.name.clone()),
                }),
                ast::FieldKind::Map { .. } => ast::Ty::Named(ast::TypeName {
                    leading_dot: None,
                    name: ast::FullIdent::from(ast::Ident::new(field.map_message_name(), field.name.span.clone())),
                }),
            },
            FieldSource::MapKey(ty, _) => (*ty).clone(),
            FieldSource::MapValue(ty, _) => (*ty).clone(),
        }
    }

    pub fn label(&self) -> Option<ast::FieldLabel> {
        match self {
            FieldSource::Field(field) => field.label.clone().map(|(l, _)| l),
            FieldSource::MapKey(_, _) | FieldSource::MapValue(_, _) => Some(ast::FieldLabel::Optional),
        }
    }
}

fn build_message<'a>(syntax: ast::Syntax, ast: &'a ast::Message, messages: &mut Vec<Message<'a>>) {
    let (fields, nested_messages, oneofs) = build_message_body(syntax, &ast.body);
    messages.push(Message {
        ast: MessageSource::Message(ast),
        fields,
        messages: nested_messages,
        oneofs,
    })
}

fn build_message_body(
    syntax: ast::Syntax,
    ast: &ast::MessageBody,
) -> (Vec<Field>, Vec<Message>, Vec<Oneof>) {
    let mut fields = Vec::new();
    let mut messages = Vec::new();
    let mut oneofs = Vec::new();

    for oneof in ast.oneofs() {
        // Real oneofs must be ordered before any synthetic oneofs generated by fields
        oneofs.push(Oneof {
            ast: OneofSource::Oneof(oneof),
        });
    }

    let mut real_oneof_index = 0;
    for field in &ast.items {
        match field {
            ast::MessageItem::Field(field) => {
                build_field(syntax, field, &mut fields, &mut messages, &mut oneofs, None)
            }
            ast::MessageItem::Message(message) => build_message(syntax, message, &mut messages),
            ast::MessageItem::Extend(extend) => build_extend(syntax, extend, &mut messages),
            ast::MessageItem::Oneof(oneof) => {
                for field in &oneof.fields {
                    build_field(
                        syntax,
                        field,
                        &mut fields,
                        &mut messages,
                        &mut oneofs,
                        Some(real_oneof_index),
                    )
                }
                real_oneof_index += 1;
            }
            ast::MessageItem::Enum(_) => continue,
        }
    }

    (fields, messages, oneofs)
}

fn build_field<'a>(
    syntax: ast::Syntax,
    field: &'a ast::Field,
    fields: &mut Vec<Field<'a>>,
    messages: &mut Vec<Message<'a>>,
    oneofs: &mut Vec<Oneof<'a>>,
    mut oneof_index: Option<i32>,
) {
    let is_synthetic_oneof = match &field.kind {
        ast::FieldKind::Normal { .. } => {
            if oneof_index.is_none()
                && syntax != ast::Syntax::Proto2
                && matches!(field.label, Some((ast::FieldLabel::Optional, _)))
            {
                oneof_index = Some(index_to_i32(oneofs.len()));
                oneofs.push(Oneof {
                    ast: OneofSource::Field(field),
                });
                true
            } else {
                false
            }
        }
        ast::FieldKind::Group { body, .. } => {
            let (nested_fields, nested_messages, oneofs) = build_message_body(syntax, body);
            messages.push(Message {
                ast: MessageSource::Group(field, body),
                fields: nested_fields,
                messages: nested_messages,
                oneofs,
            });
            false
        }
        ast::FieldKind::Map {
            key_ty,
            key_ty_span,
            value_ty,
            value_ty_span,
            ..
        } => {
            messages.push(Message {
                ast: MessageSource::Map(field),
                fields: vec![
                    Field {
                        ast: FieldSource::MapKey(key_ty, key_ty_span.clone()),
                        oneof_index: None,
                        is_synthetic_oneof: false,
                    },
                    Field {
                        ast: FieldSource::MapValue(value_ty, value_ty_span.clone()),
                        oneof_index: None,
                        is_synthetic_oneof: false,
                    },
                ],
                messages: Vec::new(),
                oneofs: Vec::new(),
            });
            false
        }
    };

    fields.push(Field {
        ast: FieldSource::Field(field),
        oneof_index,
        is_synthetic_oneof,
    });
}

fn build_extend<'a>(syntax: ast::Syntax, ast: &'a ast::Extend, messages: &mut Vec<Message<'a>>) {
    for field in &ast.fields {
        if let ast::FieldKind::Group { body, .. } = &field.kind {
            let (fields, nested_messages, oneofs) = build_message_body(syntax, body);
            messages.push(Message {
                ast: MessageSource::Group(field, body),
                fields,
                messages: nested_messages,
                oneofs,
            })
        }
    }
}
