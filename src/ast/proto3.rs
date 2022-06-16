use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    imports: Vec<Import>,
    package: std::option::Option<Package>,
    options: Vec<Option>,
    messages: Vec<Message>,
    definitions: Vec<Definition>,
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
