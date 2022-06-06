use super::*;

pub struct File {
    imports: Vec<Import>,
    package: std::option::Option<Package>,
    options: Vec<Option>,
    definitions: Vec<Definition>,
}

pub enum Definition {
    Message(MessageDefinition),
    Enum(EnumDefinition),
    Service(ServiceDefinition),
    Extension(ExtensionDefinition),
}

pub struct MessageDefinition {
    name: Ident,
    body: MessageBody,
}

pub struct MessageBody {
    fields: Vec<FieldDefinition>,
    enums: Vec<EnumDefinition>,
    messages: Vec<MessageDefinition>,
    extensions: Vec<ExtensionDefinition>,
    extensionRanges: Vec<ReservedRange>,
    groups: Vec<GroupDefinition>,
    options: Vec<Option>,
    oneofs: Vec<OneofDefinition>,
    reserved: Vec<Reserved>,
}

pub struct FieldDefinition {
    label: FieldLabel,
    name: Ident,
    ty: Ty,
    number: IntLiteral,
    options: Vec<Option>,
}

pub enum FieldLabel {
    Required,
    Optional,
    Repeated,
}

pub struct ExtensionDefinition {
    extendee: TypeName,
    fields: Vec<ExtensionField>,
}

pub enum ExtensionField {
    Field(FieldDefinition),
    Group(GroupDefinition),
}

pub struct GroupDefinition {
    label: FieldLabel,
    name: Ident,
    number: IntLiteral,
    body: MessageBody,
}
