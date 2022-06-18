use std::ops::Range;

use logos::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pub syntax: Syntax,
    pub packages: Vec<Package>,
    pub imports: Vec<Import>,
    pub options: Vec<Option>,
    pub definitions: Vec<Definition>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Syntax {
    Proto2,
    Proto3,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Empty,
    Package(Package),
    Import(Import),
    Option(Option),
    Definition(Definition),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    Message(Message),
    Enum(Enum),
    Service(Service),
    Extension(Extension),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    pub span: Span,
    pub value: std::string::String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FullIdent {
    pub parts: Vec<Ident>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeName {
    pub leading_dot: std::option::Option<Span>,
    pub name: FullIdent,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Int {
    pub negative: bool,
    pub value: u64,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Float {
    pub value: f64,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bool {
    pub value: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct String {
    pub value: std::string::String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    FullIdent(FullIdent),
    Int(Int),
    Float(Float),
    String(String),
    Bool(Bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub kind: std::option::Option<ImportKind>,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportKind {
    Weak,
    Public,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub name: FullIdent,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Option {
    pub name: FullIdent,
    pub field_name: std::option::Option<FullIdent>,
    pub value: Constant,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub name: Ident,
    pub body: MessageBody,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub label: std::option::Option<FieldLabel>,
    pub name: Ident,
    pub ty: Ty,
    pub number: Int,
    pub options: Vec<Option>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldLabel {
    Required,
    Optional,
    Repeated,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessageBody {
    pub map_fields: Vec<MapField>,
    pub fields: Vec<Field>,
    pub enums: Vec<Enum>,
    pub messages: Vec<Message>,
    pub extensions: Vec<Extension>,
    pub extension_ranges: Vec<ReservedRange>,
    pub groups: Vec<Group>,
    pub options: Vec<Option>,
    pub oneofs: Vec<Oneof>,
    pub reserved: Vec<Reserved>,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct Oneof {
    pub name: Ident,
    pub options: Vec<Option>,
    pub fields: Vec<OneofField>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OneofField {
    pub ty: Ty,
    pub name: Ident,
    pub number: Int,
    pub options: Vec<Option>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapField {
    pub key_ty: KeyTy,
    pub ty: Ty,
    pub name: Ident,
    pub number: Int,
    pub options: Vec<Option>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Extension {
    pub extendee: TypeName,
    pub fields: Vec<ExtensionField>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExtensionField {
    Field(Field),
    Group(Group),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub label: FieldLabel,
    pub name: Ident,
    pub number: Int,
    pub body: MessageBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Reserved {
    Ranges(Vec<ReservedRange>),
    Names(Vec<Ident>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReservedRange {
    pub start: Int,
    pub end: std::option::Option<Int>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    pub name: Ident,
    pub options: Vec<Option>,
    pub values: Vec<EnumValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumValue {
    pub name: Ident,
    pub value: Int,
    pub options: Vec<Option>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Service {
    pub name: Ident,
    pub options: Vec<Option>,
    pub methods: Vec<Method>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Method {
    pub name: Ident,
    pub input_ty: TypeName,
    pub output_ty: TypeName,
    pub options: Vec<Option>,
    pub is_client_streaming: bool,
    pub is_server_streaming: bool,
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
