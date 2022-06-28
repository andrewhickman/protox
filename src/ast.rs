use std::{fmt, ops::Range, vec};

use logos::Span;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct File {
    pub syntax: Syntax,
    pub package: std::option::Option<Package>,
    pub imports: Vec<Import>,
    pub options: Vec<Option>,
    pub enums: Vec<Enum>,
    pub messages: Vec<Message>,
    pub extends: Vec<Extend>,
    pub services: Vec<Service>,
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Bool {
    pub value: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct String {
    pub value: std::string::String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Constant {
    FullIdent(FullIdent),
    Int(Int),
    Float(Float),
    String(String),
    Bool(Bool),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Import {
    pub kind: std::option::Option<ImportKind>,
    pub value: String,
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
pub(crate) struct OptionBody {
    pub name: FullIdent,
    pub field_name: std::option::Option<FullIdent>,
    pub value: Constant,
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
    pub label: std::option::Option<FieldLabel>,
    pub name: Ident,
    pub ty: Ty,
    pub number: Int,
    pub options: Vec<OptionBody>,
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
    pub fields: Vec<MessageField>,
    pub enums: Vec<Enum>,
    pub messages: Vec<Message>,
    pub extends: Vec<Extend>,
    pub extensions: Vec<Extensions>,
    pub options: Vec<Option>,
    pub oneofs: Vec<Oneof>,
    pub reserved: Vec<Reserved>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MessageField {
    Field(Field),
    Group(Group),
    Map(Map),
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
pub(crate) enum KeyTy {
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
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Oneof {
    pub name: Ident,
    pub options: Vec<Option>,
    pub fields: Vec<MessageField>,
    pub comments: Comments,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Map {
    pub key_ty: KeyTy,
    pub ty: Ty,
    pub name: Ident,
    pub number: Int,
    pub options: Vec<OptionBody>,
    pub comments: Comments,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Extend {
    pub extendee: TypeName,
    pub fields: Vec<MessageField>,
    pub comments: Comments,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Group {
    pub label: std::option::Option<FieldLabel>,
    pub name: Ident,
    pub number: Int,
    pub body: MessageBody,
    pub options: Vec<OptionBody>,
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
    pub options: Vec<OptionBody>,
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
    Max,
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
    pub value: Int,
    pub options: Vec<OptionBody>,
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
    pub is_client_streaming: bool,
    pub is_server_streaming: bool,
    pub comments: Comments,
    pub span: Span,
}

impl Default for File {
    fn default() -> Self {
        File {
            syntax: Syntax::Proto2,
            package: None,
            imports: vec![],
            options: vec![],
            enums: vec![],
            messages: vec![],
            extends: vec![],
            services: vec![],
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
