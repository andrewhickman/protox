use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    imports: Vec<Import>,
    package: std::option::Option<Package>,
    options: Vec<Option>,
    definitions: Vec<Definition>,
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
    name: Ident,
    body: MessageBody,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessageBody {
    map_fields: Vec<MapField>,
    fields: Vec<Field>,
    enums: Vec<Enum>,
    messages: Vec<Message>,
    extensions: Vec<Extension>,
    extension_ranges: Vec<ReservedRange>,
    groups: Vec<Group>,
    options: Vec<Option>,
    oneofs: Vec<Oneof>,
    reserved: Vec<Reserved>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    label: FieldLabel,
    name: Ident,
    ty: Ty,
    number: Int,
    options: Vec<Option>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldLabel {
    Required,
    Optional,
    Repeated,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Extension {
    extendee: TypeName,
    fields: Vec<ExtensionField>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExtensionField {
    Field(Field),
    Group(Group),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    label: FieldLabel,
    name: Ident,
    number: Int,
    body: MessageBody,
}
