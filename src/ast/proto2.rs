use super::*;

pub struct File {
    imports: Vec<Import>,
    package: std::option::Option<Package>,
    options: Vec<Option>,
    definitions: Vec<Definition>,
}

pub enum Definition {
    Message(Message),
    Enum(Enum),
    Service(Service),
    Extension(Extension),
}

pub struct Message {
    name: Ident,
    body: MessageBody,
}

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

pub struct Field {
    label: FieldLabel,
    name: Ident,
    ty: Ty,
    number: Int,
    options: Vec<Option>,
}

pub enum FieldLabel {
    Required,
    Optional,
    Repeated,
}

pub struct Extension {
    extendee: TypeName,
    fields: Vec<ExtensionField>,
}

pub enum ExtensionField {
    Field(Field),
    Group(Group),
}

pub struct Group {
    label: FieldLabel,
    name: Ident,
    number: Int,
    body: MessageBody,
}
