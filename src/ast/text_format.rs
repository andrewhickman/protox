use std::fmt::{self, Write};

use logos::Span;

use super::{Float, FullIdent, Ident, Int, String};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Message {
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Field {
    pub name: FieldName,
    pub value: FieldValue,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum FieldName {
    Ident(Ident),
    Extension(FullIdent, Span),
    Any(FullIdent, FullIdent, Span),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum FieldValue {
    Message(Message, Span),
    MessageList(Vec<(Message, Span)>, Span),
    Scalar(Scalar),
    ScalarList(Vec<Scalar>, Span),
}

#[derive(Clone, Debug, PartialEq)]
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

impl FieldValue {
    pub fn span(&self) -> Span {
        match self {
            FieldValue::Message(_, span)
            | FieldValue::MessageList(_, span)
            | FieldValue::ScalarList(_, span) => span.clone(),
            FieldValue::Scalar(scalar) => scalar.span(),
        }
    }
}

impl Scalar {
    pub fn span(&self) -> Span {
        match self {
            Scalar::String(s) => s.span.clone(),
            Scalar::Float(f) => f.span.clone(),
            Scalar::Ident { span, .. } => span.clone(),
            Scalar::Int(i) => i.span.clone(),
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let delimiter = if f.alternate() { "\n" } else { "," };

        let mut first = true;
        for field in &self.fields {
            if !first {
                delimiter.fmt(f)?;
            }
            field.fmt(f)?;
            first = false;
        }
        Ok(())
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.name, self.value)
    }
}

impl fmt::Display for FieldName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldName::Ident(ident) => ident.fmt(f),
            FieldName::Extension(ext, _) => write!(f, "[{}]", ext),
            FieldName::Any(domain, ty, _) => write!(f, "[{}/{}]", domain, ty),
        }
    }
}

impl fmt::Display for FieldValue {
    fn fmt(&self, mut f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::Message(message, _) => {
                if f.alternate() {
                    writeln!(f, "{{")?;
                    writeln!(indented(&mut f), "{:#}", message)?;
                    write!(f, "}}")?;
                } else {
                    write!(f, "{{{}}}", message)?;
                }
                Ok(())
            }
            FieldValue::MessageList(list, _) => {
                write!(f, "[")?;
                let mut first = true;
                for (message, _) in list {
                    if first {
                        message.fmt(f)?;
                    } else if f.alternate() {
                        write!(f, ", {:#}", message)?;
                    } else {
                        write!(f, ",{}", message)?;
                    }
                    first = false;
                }
                write!(f, "]")?;
                Ok(())
            }
            FieldValue::Scalar(scalar) => scalar.fmt(f),
            FieldValue::ScalarList(list, _) => {
                write!(f, "[")?;
                let mut first = true;
                for scalar in list {
                    if first {
                        scalar.fmt(f)?;
                    } else if f.alternate() {
                        write!(f, ", {:#}", scalar)?;
                    } else {
                        write!(f, ",{}", scalar)?;
                    }
                    first = false;
                }
                write!(f, "]")?;
                Ok(())
            }
        }
    }
}

impl fmt::Display for Scalar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scalar::String(string) => write!(f, "\"{}\"", string),
            Scalar::Float(float) => write!(f, "{}", float),
            Scalar::Ident {
                negative, ident, ..
            } => {
                if *negative {
                    write!(f, "-")?;
                }
                write!(f, "{}", ident)?;
                Ok(())
            }
            Scalar::Int(int) => write!(f, "{}", int),
        }
    }
}

struct Indented<W> {
    writer: W,
    on_newline: bool,
}

impl<W> fmt::Write for Indented<W>
where
    W: fmt::Write,
{
    fn write_str(&mut self, mut s: &str) -> fmt::Result {
        while !s.is_empty() {
            if self.on_newline {
                self.writer.write_str("  ")?;
            }

            let split = match s.find('\n') {
                Some(pos) => {
                    self.on_newline = true;
                    pos + 1
                }
                None => {
                    self.on_newline = false;
                    s.len()
                }
            };
            self.writer.write_str(&s[..split])?;
            s = &s[split..];
        }

        Ok(())
    }
}

fn indented<W>(writer: W) -> Indented<W> {
    Indented {
        writer,
        on_newline: true,
    }
}
