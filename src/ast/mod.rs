use std::ops::Range;

use logos::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct File {
    pub syntax: Syntax,
    pub packages: Vec<Package>,
    pub imports: Vec<Import>,
    pub options: Vec<Option>,
    pub definitions: Vec<Definition>,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Comments {
    pub leading_detached_comments: Vec<std::string::String>,
    pub leading_comment: std::option::Option<std::string::String>,
    pub trailing_comment: std::option::Option<std::string::String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Syntax {
    Proto2,
    Proto3,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Definition {
    Message(Message),
    Enum(Enum),
    Service(Service),
    Extension(Extension),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub value: std::string::String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FullIdent {
    pub parts: Vec<Ident>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeName {
    pub leading_dot: std::option::Option<Span>,
    pub name: FullIdent,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Int {
    pub negative: bool,
    pub value: u64,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Float {
    pub value: f64,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bool {
    pub value: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct String {
    pub value: std::string::String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Constant {
    FullIdent(FullIdent),
    Int(Int),
    Float(Float),
    String(String),
    Bool(Bool),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Import {
    pub kind: std::option::Option<ImportKind>,
    pub value: String,
    pub comments: Comments,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImportKind {
    Weak,
    Public,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Package {
    pub name: FullIdent,
    pub comments: Comments,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Option {
    pub name: FullIdent,
    pub field_name: std::option::Option<FullIdent>,
    pub value: Constant,
    pub comments: Comments,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub name: Ident,
    pub body: MessageBody,
    pub comments: Comments,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub label: std::option::Option<FieldLabel>,
    pub name: Ident,
    pub ty: Ty,
    pub number: Int,
    pub options: Vec<Option>,
    pub comments: Comments,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FieldLabel {
    Required,
    Optional,
    Repeated,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct MessageBody {
    pub fields: Vec<MessageField>,
    pub enums: Vec<Enum>,
    pub messages: Vec<Message>,
    pub extensions: Vec<Extension>,
    pub extension_ranges: Vec<ReservedRange>,
    pub options: Vec<Option>,
    pub oneofs: Vec<Oneof>,
    pub reserved: Vec<Reserved>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MessageField {
    Field(Field),
    Group(Group),
    Map(Map),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
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
pub enum KeyTy {
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
pub struct Oneof {
    pub name: Ident,
    pub options: Vec<Option>,
    pub fields: Vec<MessageField>,
    pub comments: Comments,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    pub key_ty: KeyTy,
    pub ty: Ty,
    pub name: Ident,
    pub number: Int,
    pub options: Vec<Option>,
    pub comments: Comments,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Extension {
    pub extendee: TypeName,
    pub fields: Vec<MessageField>,
    pub comments: Comments,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub label: std::option::Option<FieldLabel>,
    pub name: Ident,
    pub number: Int,
    pub body: MessageBody,
    pub comments: Comments,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Reserved {
    Ranges(Vec<ReservedRange>, Comments),
    Names(Vec<Ident>, Comments),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReservedRange {
    pub start: Int,
    pub end: ReservedRangeEnd,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReservedRangeEnd {
    None,
    Int(Int),
    Max,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Enum {
    pub name: Ident,
    pub options: Vec<Option>,
    pub values: Vec<EnumValue>,
    pub reserved: Vec<Reserved>,
    pub comments: Comments,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumValue {
    pub name: Ident,
    pub value: Int,
    pub options: Vec<Option>,
    pub comments: Comments,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Service {
    pub name: Ident,
    pub options: Vec<Option>,
    pub methods: Vec<Method>,
    pub comments: Comments,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Method {
    pub name: Ident,
    pub input_ty: TypeName,
    pub output_ty: TypeName,
    pub options: Vec<Option>,
    pub is_client_streaming: bool,
    pub is_server_streaming: bool,
    pub comments: Comments,
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
