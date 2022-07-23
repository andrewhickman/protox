use std::{
    convert::TryFrom,
    fmt::{self, Write},
    ops::Range,
    vec,
};

use logos::Span;

use crate::{join_span, types::field_descriptor_proto};

pub(crate) mod text_format;

#[derive(Default, Clone, Debug, PartialEq)]
pub(crate) struct File {
    pub span: Span,
    pub syntax: Syntax,
    pub syntax_span: std::option::Option<(Span, Comments)>,
    pub package: std::option::Option<Package>,
    pub imports: Vec<Import>,
    pub options: Vec<Option>,
    pub items: Vec<FileItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum FileItem {
    Enum(Enum),
    Message(Message),
    Extend(Extend),
    Service(Service),
}

#[derive(Clone, Default, Debug, PartialEq)]
pub(crate) struct Comments {
    pub leading_detached_comments: Vec<std::string::String>,
    pub leading_comment: std::option::Option<std::string::String>,
    pub trailing_comment: std::option::Option<std::string::String>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum Syntax {
    Proto2,
    Proto3,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Ident {
    pub value: std::string::String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FullIdent {
    pub parts: Vec<Ident>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TypeName {
    pub leading_dot: std::option::Option<Span>,
    pub name: FullIdent,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Int {
    pub negative: bool,
    pub value: u64,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Float {
    pub value: f64,
    pub span: Span,
}

#[derive(Clone, PartialEq)]
pub(crate) struct String {
    pub value: Vec<u8>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum OptionValue {
    Ident {
        negative: bool,
        ident: Ident,
        span: Span,
    },
    Int(Int),
    Float(Float),
    String(String),
    Aggregate(text_format::Message, Span),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Import {
    pub kind: std::option::Option<(ImportKind, Span)>,
    pub value: std::string::String,
    pub value_span: Span,
    pub comments: Comments,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ImportKind {
    Weak,
    Public,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Package {
    pub name: FullIdent,
    pub comments: Comments,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Option {
    pub body: OptionBody,
    pub comments: Comments,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum OptionNamePart {
    Ident(Ident),
    Extension(FullIdent, Span),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OptionBody {
    pub name: Vec<OptionNamePart>,
    pub value: OptionValue,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub(crate) struct OptionList {
    pub options: Vec<OptionBody>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Message {
    pub name: Ident,
    pub body: MessageBody,
    pub comments: Comments,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Field {
    pub label: std::option::Option<(FieldLabel, Span)>,
    pub name: Ident,
    pub kind: FieldKind,
    pub number: Int,
    pub options: std::option::Option<OptionList>,
    pub comments: Comments,
    pub span: Span,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum FieldLabel {
    Optional = 1,
    Required = 2,
    Repeated = 3,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct MessageBody {
    pub items: Vec<MessageItem>,
    pub extensions: Vec<Extensions>,
    pub options: Vec<Option>,
    pub reserved: Vec<Reserved>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MessageItem {
    Field(Field),
    Enum(Enum),
    Message(Message),
    Extend(Extend),
    Oneof(Oneof),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum FieldKind {
    Normal {
        ty: Ty,
        ty_span: Span,
    },
    Group {
        ty_span: Span,
        body: MessageBody,
    },
    Map {
        ty_span: Span,
        key_ty: Ty,
        key_ty_span: Span,
        value_ty: Ty,
        value_ty_span: Span,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Ty {
    Double,
    Float,
    Int32,
    Int64,
    Uint32,
    Uint64,
    Sint32,
    Sint64,
    Fixed32,
    Fixed64,
    Sfixed32,
    Sfixed64,
    Bool,
    String,
    Bytes,
    Named(TypeName),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Oneof {
    pub name: Ident,
    pub options: Vec<Option>,
    pub fields: Vec<Field>,
    pub comments: Comments,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Extend {
    pub extendee: TypeName,
    pub fields: Vec<Field>,
    pub comments: Comments,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Reserved {
    pub kind: ReservedKind,
    pub comments: Comments,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Extensions {
    pub ranges: Vec<ReservedRange>,
    pub options: std::option::Option<OptionList>,
    pub comments: Comments,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ReservedKind {
    Ranges(Vec<ReservedRange>),
    Names(Vec<Ident>),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ReservedRange {
    pub start: Int,
    pub end: ReservedRangeEnd,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ReservedRangeEnd {
    None,
    Int(Int),
    Max(Span),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Enum {
    pub name: Ident,
    pub options: Vec<Option>,
    pub values: Vec<EnumValue>,
    pub reserved: Vec<Reserved>,
    pub comments: Comments,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EnumValue {
    pub name: Ident,
    pub number: Int,
    pub options: std::option::Option<OptionList>,
    pub comments: Comments,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Service {
    pub name: Ident,
    pub options: Vec<Option>,
    pub methods: Vec<Method>,
    pub comments: Comments,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Method {
    pub name: Ident,
    pub input_ty: TypeName,
    pub output_ty: TypeName,
    pub options: Vec<Option>,
    pub client_streaming: std::option::Option<Span>,
    pub server_streaming: std::option::Option<Span>,
    pub comments: Comments,
    pub span: Span,
}

impl Default for Syntax {
    fn default() -> Self {
        Syntax::Proto2
    }
}

impl Int {
    pub fn as_i32(&self) -> std::option::Option<i32> {
        if self.negative {
            i32::try_from(self.value).ok()?.checked_neg()
        } else {
            i32::try_from(self.value).ok()
        }
    }

    pub fn as_i64(&self) -> std::option::Option<i64> {
        if self.negative {
            i64::try_from(self.value).ok()?.checked_neg()
        } else {
            i64::try_from(self.value).ok()
        }
    }
}

impl String {
    pub fn into_utf8(self) -> Result<(std::string::String, Span), Self> {
        match std::string::String::from_utf8(self.value) {
            Ok(string) => Ok((string, self.span)),
            Err(err) => Err(String {
                value: err.into_bytes(),
                span: self.span,
            }),
        }
    }
}

impl Ident {
    pub fn new(value: impl Into<std::string::String>, span: Range<usize>) -> Self {
        Ident {
            span,
            value: value.into(),
        }
    }
}

impl FullIdent {
    pub fn span(&self) -> Span {
        self.parts.first().unwrap().span.start..self.parts.last().unwrap().span.end
    }
}

impl TypeName {
    pub fn span(&self) -> Span {
        if let Some(leading_dot) = &self.leading_dot {
            join_span(leading_dot.clone(), self.name.span())
        } else {
            self.name.span()
        }
    }
}

impl Ty {
    pub fn proto_ty(&self) -> std::option::Option<field_descriptor_proto::Type> {
        match self {
            Ty::Double => Some(field_descriptor_proto::Type::Double),
            Ty::Float => Some(field_descriptor_proto::Type::Float),
            Ty::Int32 => Some(field_descriptor_proto::Type::Int32),
            Ty::Int64 => Some(field_descriptor_proto::Type::Int64),
            Ty::Uint32 => Some(field_descriptor_proto::Type::Uint32),
            Ty::Uint64 => Some(field_descriptor_proto::Type::Uint64),
            Ty::Sint32 => Some(field_descriptor_proto::Type::Sint32),
            Ty::Sint64 => Some(field_descriptor_proto::Type::Sint64),
            Ty::Fixed32 => Some(field_descriptor_proto::Type::Fixed32),
            Ty::Fixed64 => Some(field_descriptor_proto::Type::Fixed64),
            Ty::Sfixed32 => Some(field_descriptor_proto::Type::Sfixed32),
            Ty::Sfixed64 => Some(field_descriptor_proto::Type::Sfixed64),
            Ty::Bool => Some(field_descriptor_proto::Type::Bool),
            Ty::String => Some(field_descriptor_proto::Type::String),
            Ty::Bytes => Some(field_descriptor_proto::Type::Bytes),
            Ty::Named(_) => None,
        }
    }

    pub fn ty_name(&self) -> std::option::Option<std::string::String> {
        match self {
            Ty::Named(name) => Some(name.to_string()),
            _ => None,
        }
    }
}

impl OptionNamePart {
    pub fn span(&self) -> Span {
        match self {
            OptionNamePart::Ident(ident) => ident.span.clone(),
            OptionNamePart::Extension(_, span) => span.clone(),
        }
    }
}

impl OptionBody {
    pub fn name_span(&self) -> Span {
        debug_assert!(!self.name.is_empty());
        join_span(
            self.name.first().unwrap().span(),
            self.name.last().unwrap().span(),
        )
    }

    pub fn span(&self) -> Span {
        join_span(self.name_span(), self.value.span())
    }
}

impl OptionValue {
    pub fn span(&self) -> Span {
        match self {
            OptionValue::Ident { span, .. } => span.clone(),
            OptionValue::Int(int) => int.span.clone(),
            OptionValue::Float(float) => float.span.clone(),
            OptionValue::String(string) => string.span.clone(),
            OptionValue::Aggregate(_, span) => span.clone(),
        }
    }
}

impl ReservedRange {
    pub fn start_span(&self) -> Span {
        self.start.span.clone()
    }

    pub fn end_span(&self) -> Span {
        match &self.end {
            ReservedRangeEnd::None => self.start.span.clone(),
            ReservedRangeEnd::Int(end) => end.span.clone(),
            ReservedRangeEnd::Max(end_span) => end_span.clone(),
        }
    }

    pub fn span(&self) -> Span {
        join_span(self.start_span(), self.end_span())
    }
}

impl From<Ident> for FullIdent {
    fn from(value: Ident) -> Self {
        FullIdent { parts: vec![value] }
    }
}

impl From<Vec<Ident>> for FullIdent {
    fn from(parts: Vec<Ident>) -> Self {
        debug_assert!(!parts.is_empty());
        FullIdent { parts }
    }
}

impl fmt::Display for Syntax {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Syntax::Proto2 => write!(f, "proto2"),
            Syntax::Proto3 => write!(f, "proto3"),
        }
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl fmt::Display for FullIdent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.parts[0])?;
        for part in &self.parts[1..] {
            write!(f, ".{}", part)?;
        }
        Ok(())
    }
}

impl fmt::Display for TypeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.leading_dot.is_some() {
            write!(f, ".")?;
        }
        write!(f, "{}", self.name)
    }
}

impl fmt::Display for OptionValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptionValue::Ident {
                negative, ident, ..
            } => {
                if *negative {
                    write!(f, "-{}", ident)
                } else {
                    write!(f, "{}", ident)
                }
            }
            OptionValue::Int(int) => int.fmt(f),
            OptionValue::Float(float) => float.fmt(f),
            OptionValue::String(string) => string.fmt(f),
            OptionValue::Aggregate(message, _) => message.fmt(f),
        }
    }
}

impl fmt::Display for Int {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.negative {
            write!(f, "-{}", self.value)
        } else {
            write!(f, "{}", self.value)
        }
    }
}

impl fmt::Display for Float {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.value.fract() == 0.0 {
            write!(f, "{:.1}", self.value)
        } else {
            write!(f, "{}", self.value)
        }
    }
}

impl fmt::Debug for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self.to_string();
        f.debug_struct("String")
            .field("value", &value)
            .field("span", &self.span)
            .finish()
    }
}

impl fmt::Display for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &ch in &self.value {
            match ch {
                b'\t' => f.write_str("\\t")?,
                b'\r' => f.write_str("\\r")?,
                b'\n' => f.write_str("\\n")?,
                b'\\' => f.write_str("\\\\")?,
                b'\'' => f.write_str("\\'")?,
                b'"' => f.write_str("\\\"")?,
                b'\x20'..=b'\x7e' => f.write_char(ch as char)?,
                _ => {
                    write!(f, "\\{:03o}", ch)?;
                }
            }
        }

        Ok(())
    }
}
