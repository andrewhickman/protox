use std::{borrow::Cow, fmt, ops::Range, vec};

use logos::Span;

use crate::{case::to_pascal_case, index_to_i32, join_span};

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
    pub kind: std::option::Option<(ImportKind, Span)>,
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

impl Constant {
    pub fn span(&self) -> Span {
        match self {
            Constant::FullIdent(ident) => ident.span(),
            Constant::Int(int) => int.span.clone(),
            Constant::Float(float) => float.span.clone(),
            Constant::String(string) => string.span.clone(),
            Constant::Bool(b) => b.span.clone(),
        }
    }
}

impl File {
    pub fn public_imports(&self) -> impl Iterator<Item = (i32, &'_ Import)> {
        self.imports
            .iter()
            .enumerate()
            .filter_map(|(index, import)| match &import.kind {
                Some((ImportKind::Public, _)) => Some((index_to_i32(index), import)),
                _ => None,
            })
    }

    pub fn weak_imports(&self) -> impl Iterator<Item = (i32, &'_ Import)> {
        self.imports
            .iter()
            .enumerate()
            .filter_map(|(index, import)| match &import.kind {
                Some((ImportKind::Weak, _)) => Some((index_to_i32(index), import)),
                _ => None,
            })
    }

    pub fn extends(&self) -> impl Iterator<Item = &'_ Extend> {
        self.items.iter().filter_map(|item| {
            if let FileItem::Extend(extend) = item {
                Some(extend)
            } else {
                None
            }
        })
    }

    pub fn enums(&self) -> impl Iterator<Item = &'_ Enum> {
        self.items.iter().filter_map(|item| {
            if let FileItem::Enum(enu) = item {
                Some(enu)
            } else {
                None
            }
        })
    }

    pub fn services(&self) -> impl Iterator<Item = &'_ Service> {
        self.items.iter().filter_map(|item| {
            if let FileItem::Service(service) = item {
                Some(service)
            } else {
                None
            }
        })
    }
}

impl MessageBody {
    pub fn extends(&self) -> impl Iterator<Item = &'_ Extend> {
        self.items.iter().filter_map(|item| {
            if let MessageItem::Extend(extend) = item {
                Some(extend)
            } else {
                None
            }
        })
    }

    pub fn enums(&self) -> impl Iterator<Item = &'_ Enum> {
        self.items.iter().filter_map(|item| {
            if let MessageItem::Enum(enu) = item {
                Some(enu)
            } else {
                None
            }
        })
    }

    pub fn reserved_ranges(&self) -> impl Iterator<Item = (&'_ Reserved, &'_ [ReservedRange])> {
        self.reserved
            .iter()
            .filter_map(|reserved| match &reserved.kind {
                ReservedKind::Ranges(ranges) => Some((reserved, ranges.as_slice())),
                _ => None,
            })
    }

    pub fn reserved_names(&self) -> impl Iterator<Item = (&'_ Reserved, &'_ [Ident])> {
        self.reserved
            .iter()
            .filter_map(|reserved| match &reserved.kind {
                ReservedKind::Names(names) => Some((reserved, names.as_slice())),
                _ => None,
            })
    }
}

impl Enum {
    pub fn reserved_ranges(&self) -> impl Iterator<Item = (&'_ Reserved, &'_ [ReservedRange])> {
        self.reserved
            .iter()
            .filter_map(|reserved| match &reserved.kind {
                ReservedKind::Ranges(ranges) => Some((reserved, ranges.as_slice())),
                _ => None,
            })
    }

    pub fn reserved_names(&self) -> impl Iterator<Item = (&'_ Reserved, &'_ [Ident])> {
        self.reserved
            .iter()
            .filter_map(|reserved| match &reserved.kind {
                ReservedKind::Names(names) => Some((reserved, names.as_slice())),
                _ => None,
            })
    }
}

impl Field {
    pub fn default_value(&self) -> std::option::Option<&OptionBody> {
        self.options.as_ref().and_then(|options| {
            options
                .options
                .iter()
                .find(|o| o.name.parts.len() == 1 && o.name.parts[0].value == "default")
        })
    }

    pub fn is_map(&self) -> bool {
        matches!(&self.kind, FieldKind::Map { .. })
    }

    pub fn is_group(&self) -> bool {
        matches!(&self.kind, FieldKind::Group { .. })
    }

    pub fn map_message_name(&self) -> std::string::String {
        to_pascal_case(&self.name.value) + "Entry"
    }

    pub fn field_name(&self) -> Cow<'_, str> {
        if self.is_group() {
            Cow::Owned(self.name.value.to_ascii_lowercase())
        } else {
            Cow::Borrowed(self.name.value.as_str())
        }
    }
}

impl Field {
    pub fn synthetic_oneof_name(&self) -> std::string::String {
        format!("_{}", &self.name.value)
    }
}

impl OptionBody {
    pub fn span(&self) -> Span {
        join_span(self.name.span(), self.value.span())
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

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::FullIdent(ident) => ident.fmt(f),
            Constant::Int(int) => int.fmt(f),
            Constant::Float(float) => float.fmt(f),
            Constant::String(string) => string.fmt(f),
            Constant::Bool(bool) => bool.fmt(f),
        }
    }
}

impl fmt::Display for Int {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl fmt::Display for Float {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl fmt::Display for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl fmt::Display for Bool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}
