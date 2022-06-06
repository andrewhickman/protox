mod proto2;
mod proto3;

#[derive(Debug, Copy, Clone)]
struct Span {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy)]
struct Spanned<T> {
    span: Span,
    value: T,
}

pub enum File {
    Proto2(proto2::File),
    Proto3(proto3::File),
}

pub struct Ident {
    span: Span,
    value: String,
}

pub struct FullIdent {
    parts: Vec<Ident>,
}

pub struct TypeName {
    leading_dot: bool,
    name: FullIdent,
    span: Span,
}

pub struct IntLiteral {
    negative: bool,
    value: u64,
    span: Span,
}

pub struct FloatLiteral {
    value: f64,
    span: Span,
}

pub struct BoolLiteral {
    value: bool,
    span: Span,
}

pub struct StringLiteral {
    value: String,
    span: Span,
}

pub enum Constant {
    FullIdent(FullIdent),
    Int(IntLiteral),
    Float(FloatLiteral),
    String(StringLiteral),
    BoolLiteral(BoolLiteral),
}

pub struct Import {
    kind: std::option::Option<ImportKind>,
    value: StringLiteral,
}

pub enum ImportKind {
    Weak,
    Public,
}

pub struct Package {
    name: FullIdent,
}

pub struct Option {
    name: FullIdent,
    value: Constant,
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
    name: Ident,
    options: Vec<Option>,
    fields: Vec<OneofField>,
}

pub struct OneofField {
    ty: Ty,
    name: Ident,
    number: IntLiteral,
    options: Vec<Option>,
}

pub struct MapField {
    key_ty: KeyTy,
    ty: Ty,
    name: Ident,
    number: IntLiteral,
    options: Vec<Option>,
}

pub enum Reserved {
    Ranges(Vec<ReservedRange>),
    Names(Vec<Ident>),
}

pub struct ReservedRange {
    start: IntLiteral,
    end: std::option::Option<IntLiteral>,
}

pub struct EnumDefinition {
    name: Ident,
    options: Vec<Option>,
    values: Vec<EnumField>,
}

pub struct EnumField {
    name: Ident,
    value: IntLiteral,
    options: Vec<Option>,
}
