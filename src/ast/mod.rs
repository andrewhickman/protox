pub mod proto2;
pub mod proto3;

use logos::Span;

pub enum File {
    Proto2(proto2::File),
    Proto3(proto3::File),
}

pub struct Ident {
    pub span: Span,
    pub value: std::string::String,
}

pub struct FullIdent {
    pub parts: Vec<Ident>,
}

pub struct TypeName {
    pub leading_dot: bool,
    pub name: FullIdent,
    pub span: Span,
}

pub struct Int {
    pub negative: bool,
    pub value: u64,
    pub span: Span,
}

pub struct Float {
    pub value: f64,
    pub span: Span,
}

pub struct Bool {
    pub value: bool,
    pub span: Span,
}

pub struct String {
    pub value: std::string::String,
    pub span: Span,
}

pub enum Constant {
    FullIdent(FullIdent),
    Int(Int),
    Float(Float),
    String(String),
    BoolLiteral(Bool),
}

pub struct Import {
    pub kind: std::option::Option<ImportKind>,
    pub value: String,
}

pub enum ImportKind {
    Weak,
    Public,
}

pub struct Package {
    pub name: FullIdent,
}

pub struct Option {
    pub namespace: FullIdent,
    pub name: std::option::Option<FullIdent>,
    pub value: Constant,
}

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

pub struct Oneof {
    pub name: Ident,
    pub options: Vec<Option>,
    pub fields: Vec<OneofField>,
}

pub struct OneofField {
    pub ty: Ty,
    pub name: Ident,
    pub number: Int,
    pub options: Vec<Option>,
}

pub struct MapField {
    pub key_ty: KeyTy,
    pub ty: Ty,
    pub name: Ident,
    pub number: Int,
    pub options: Vec<Option>,
}

pub enum Reserved {
    Ranges(Vec<ReservedRange>),
    Names(Vec<Ident>),
}

pub struct ReservedRange {
    pub start: Int,
    pub end: std::option::Option<Int>,
}

pub struct Enum {
    pub name: Ident,
    pub options: Vec<Option>,
    pub values: Vec<EnumField>,
}

pub struct EnumField {
    pub name: Ident,
    pub value: Int,
    pub options: Vec<Option>,
}

pub struct Service {
    pub name: Ident,
    pub options: Vec<Option>,
    pub methods: Vec<Method>,
}

pub struct Method {
    pub input_ty: TypeName,
    pub output_ty: TypeName,
    pub options: Vec<Option>,
    pub is_client_streaming: bool,
    pub is_server_streaming: bool,
}

impl From<Ident> for FullIdent {
    fn from(value: Ident) -> Self {
        FullIdent { parts: vec![value] }
    }
}

impl From<Vec<Ident>> for FullIdent {
    fn from(parts: Vec<Ident>) -> Self {
        FullIdent { parts }
    }
}