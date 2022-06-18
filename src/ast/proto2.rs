use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pub packages: Vec<Package>,
    pub imports: Vec<Import>,
    pub options: Vec<Option>,
    pub definitions: Vec<Definition>,
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
pub struct Message {
    pub name: Ident,
    pub body: MessageBody,
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
pub struct Field {
    pub label: FieldLabel,
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
