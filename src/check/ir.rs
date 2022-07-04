use std::cmp::Ordering;

use crate::{ast, index_to_i32};

/// A protobuf file structure, with synthetic oneofs, groups and map messages expanded.
pub(crate) struct File<'a> {
    pub ast: &'a ast::File,
    pub messages: Vec<Message<'a>>,
}

pub(crate) struct Message<'a> {
    pub ast: MessageSource<'a>,
    pub fields: Vec<Field<'a>>,
    pub messages: Vec<Message<'a>>,
    pub oneofs: Vec<Oneof<'a>>,
}

pub(crate) enum MessageSource<'a> {
    Message(&'a ast::Message),
    Group(&'a ast::Group),
    Map(&'a ast::Map),
}

pub(crate) struct Field<'a> {
    pub ast: FieldSource<'a>,
    pub oneof_index: Option<i32>,
}

pub(crate) enum FieldSource<'a> {
    Field(&'a ast::Field),
    Group(&'a ast::Group),
    Map(&'a ast::Map),
    MapKey(&'a ast::Map),
    MapValue(&'a ast::Map),
}

pub(crate) struct Oneof<'a> {
    pub ast: OneofSource<'a>,
}

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

    for field in &ast.items {
        match field {
            ast::MessageItem::Field(field) => {
                build_message_field(syntax, field, &mut fields, &mut messages, &mut oneofs, None)
            }
            ast::MessageItem::Message(message) => build_message(syntax, message, &mut messages),
            ast::MessageItem::Extend(extend) => build_extend(syntax, extend, &mut messages),
            ast::MessageItem::Enum(_) => continue,
        }
    }

    oneofs.sort_by(|l, r| match (&l.ast, &r.ast) {
        (OneofSource::Oneof(_), OneofSource::Field(_)) => Ordering::Less,
        (OneofSource::Field(_), OneofSource::Oneof(_)) => Ordering::Greater,
        (OneofSource::Oneof(_), OneofSource::Oneof(_))
        | (OneofSource::Field(_), OneofSource::Field(_)) => Ordering::Equal,
    });

    (fields, messages, oneofs)
}

fn build_message_field<'a>(
    syntax: ast::Syntax,
    message_field: &'a ast::MessageField,
    fields: &mut Vec<Field<'a>>,
    messages: &mut Vec<Message<'a>>,
    oneofs: &mut Vec<Oneof<'a>>,
    oneof_index: Option<i32>,
) {
    match message_field {
        ast::MessageField::Field(field) => {
            if oneof_index.is_none()
                && syntax != ast::Syntax::Proto2
                && field.label == Some(ast::FieldLabel::Optional)
            {
                let oneof_index = Some(index_to_i32(oneofs.len()));
                fields.push(Field {
                    ast: FieldSource::Field(field),
                    oneof_index,
                });
                oneofs.push(Oneof {
                    ast: OneofSource::Field(field),
                });
            } else {
                fields.push(Field {
                    ast: FieldSource::Field(field),
                    oneof_index,
                });
            }
        }
        ast::MessageField::Group(group) => {
            fields.push(Field {
                ast: FieldSource::Group(group),
                oneof_index,
            });
            let (nested_fields, nested_messages, oneofs) = build_message_body(syntax, &group.body);
            messages.push(Message {
                ast: MessageSource::Group(group),
                fields: nested_fields,
                messages: nested_messages,
                oneofs,
            })
        }
        ast::MessageField::Map(map) => {
            fields.push(Field {
                ast: FieldSource::Map(map),
                oneof_index: None,
            });
            messages.push(Message {
                ast: MessageSource::Map(map),
                fields: vec![
                    Field {
                        ast: FieldSource::MapKey(map),
                        oneof_index: None,
                    },
                    Field {
                        ast: FieldSource::MapValue(map),
                        oneof_index: None,
                    },
                ],
                messages: Vec::new(),
                oneofs: Vec::new(),
            });
        }
        ast::MessageField::Oneof(oneof) => {
            let oneof_index = Some(index_to_i32(oneofs.len()));
            for field in &oneof.fields {
                build_message_field(syntax, field, fields, messages, oneofs, oneof_index)
            }
            oneofs.push(Oneof {
                ast: OneofSource::Oneof(oneof),
            })
        }
    }
}

fn build_extend<'a>(syntax: ast::Syntax, ast: &'a ast::Extend, messages: &mut Vec<Message<'a>>) {
    for field in &ast.fields {
        if let ast::MessageField::Group(group) = field {
            {
                let (fields, nested_messages, oneofs) = build_message_body(syntax, &group.body);
                messages.push(Message {
                    ast: MessageSource::Group(group),
                    fields,
                    messages: nested_messages,
                    oneofs,
                })
            };
        }
    }
}
