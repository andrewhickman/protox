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
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub name: Ident,
    pub fields: Vec<Field>,
    pub map_fields: Vec<MapField>,
    pub enums: Vec<Enum>,
    pub messages: Vec<Message>,
    pub options: Vec<Option>,
    pub oneofs: Vec<Oneof>,
    pub reserved: Vec<Reserved>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub repeated: bool,
    pub name: Ident,
    pub ty: Ty,
    pub number: Int,
    pub options: Vec<Option>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Extension {
    pub extendee: TypeName,
    pub fields: Vec<Field>,
}
