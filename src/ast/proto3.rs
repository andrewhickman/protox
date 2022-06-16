use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    package: std::option::Option<Package>,
    imports: Vec<Import>,
    options: Vec<Option>,
    definitions: Vec<Definition>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
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
    name: Ident,
    body: MessageBody,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessageBody {
    fields: Vec<Field>,
    map_fields: Vec<MapField>,
    enums: Vec<Enum>,
    messages: Vec<Message>,
    options: Vec<Option>,
    oneofs: Vec<Oneof>,
    reserved: Vec<Reserved>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    repeated: bool,
    name: Ident,
    ty: Ty,
    number: Int,
    options: Vec<Option>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Extension {
    extendee: TypeName,
    fields: Vec<Field>,
}
