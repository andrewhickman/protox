//! Variants of the prost_types generated messages which include unknown options.
#![allow(clippy::all)]

use prost::Message;

#[allow(unused_imports)]
pub(crate) use prost_types::{
    enum_descriptor_proto, field_descriptor_proto, source_code_info, uninterpreted_option,
    SourceCodeInfo, UninterpretedOption,
};

use crate::options::OptionSet;

#[derive(Clone, PartialEq, Message)]
pub(crate) struct FileDescriptorSet {
    #[prost(message, repeated, tag = "1")]
    pub file: Vec<FileDescriptorProto>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct FileDescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(string, optional, tag = "2")]
    pub package: Option<String>,
    #[prost(string, repeated, tag = "3")]
    pub dependency: Vec<String>,
    #[prost(int32, repeated, packed = "false", tag = "10")]
    pub public_dependency: Vec<i32>,
    #[prost(int32, repeated, packed = "false", tag = "11")]
    pub weak_dependency: Vec<i32>,
    #[prost(message, repeated, tag = "4")]
    pub message_type: Vec<DescriptorProto>,
    #[prost(message, repeated, tag = "5")]
    pub(crate) enum_type: Vec<EnumDescriptorProto>,
    #[prost(message, repeated, tag = "6")]
    pub service: Vec<ServiceDescriptorProto>,
    #[prost(message, repeated, tag = "7")]
    pub extension: Vec<FieldDescriptorProto>,
    #[prost(message, optional, tag = "8")]
    pub options: Option<OptionSet>,
    #[prost(message, optional, tag = "9")]
    pub source_code_info: Option<SourceCodeInfo>,
    #[prost(string, optional, tag = "12")]
    pub syntax: Option<String>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct DescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(message, repeated, tag = "2")]
    pub field: Vec<FieldDescriptorProto>,
    #[prost(message, repeated, tag = "6")]
    pub extension: Vec<FieldDescriptorProto>,
    #[prost(message, repeated, tag = "3")]
    pub nested_type: Vec<DescriptorProto>,
    #[prost(message, repeated, tag = "4")]
    pub(crate) enum_type: Vec<EnumDescriptorProto>,
    #[prost(message, repeated, tag = "5")]
    pub extension_range: Vec<descriptor_proto::ExtensionRange>,
    #[prost(message, repeated, tag = "8")]
    pub oneof_decl: Vec<OneofDescriptorProto>,
    #[prost(message, optional, tag = "7")]
    pub options: Option<OptionSet>,
    #[prost(message, repeated, tag = "9")]
    pub reserved_range: Vec<descriptor_proto::ReservedRange>,
    #[prost(string, repeated, tag = "10")]
    pub reserved_name: Vec<String>,
}

pub(crate) mod descriptor_proto {
    use crate::options::OptionSet;
    use prost::Message;

    pub(crate) use prost_types::descriptor_proto::ReservedRange;

    #[derive(Clone, PartialEq, Message)]
    pub(crate) struct ExtensionRange {
        #[prost(int32, optional, tag = "1")]
        pub start: Option<i32>,
        #[prost(int32, optional, tag = "2")]
        pub end: Option<i32>,
        #[prost(message, optional, tag = "3")]
        pub options: Option<OptionSet>,
    }
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct ExtensionRangeOptions {
    #[prost(message, repeated, tag = "999")]
    pub uninterpreted_option: Vec<UninterpretedOption>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct FieldDescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(int32, optional, tag = "3")]
    pub number: Option<i32>,
    #[prost(enumeration = "field_descriptor_proto::Label", optional, tag = "4")]
    pub label: Option<i32>,
    #[prost(enumeration = "field_descriptor_proto::Type", optional, tag = "5")]
    pub r#type: Option<i32>,
    #[prost(string, optional, tag = "6")]
    pub type_name: Option<String>,
    #[prost(string, optional, tag = "2")]
    pub extendee: Option<String>,
    #[prost(string, optional, tag = "7")]
    pub default_value: Option<String>,
    #[prost(int32, optional, tag = "9")]
    pub oneof_index: Option<i32>,
    #[prost(string, optional, tag = "10")]
    pub json_name: Option<String>,
    #[prost(message, optional, tag = "8")]
    pub options: Option<OptionSet>,
    #[prost(bool, optional, tag = "17")]
    pub proto3_optional: Option<bool>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct OneofDescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(message, optional, tag = "2")]
    pub options: Option<OptionSet>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct EnumDescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(message, repeated, tag = "2")]
    pub value: Vec<EnumValueDescriptorProto>,
    #[prost(message, optional, tag = "3")]
    pub options: Option<OptionSet>,
    #[prost(message, repeated, tag = "4")]
    pub reserved_range: Vec<enum_descriptor_proto::EnumReservedRange>,
    #[prost(string, repeated, tag = "5")]
    pub reserved_name: Vec<String>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct EnumValueDescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(int32, optional, tag = "2")]
    pub number: Option<i32>,
    #[prost(message, optional, tag = "3")]
    pub options: Option<OptionSet>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct ServiceDescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(message, repeated, tag = "2")]
    pub method: Vec<MethodDescriptorProto>,
    #[prost(message, optional, tag = "3")]
    pub options: Option<OptionSet>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct MethodDescriptorProto {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(string, optional, tag = "2")]
    pub input_type: Option<String>,
    #[prost(string, optional, tag = "3")]
    pub output_type: Option<String>,
    #[prost(message, optional, tag = "4")]
    pub options: Option<OptionSet>,
    #[prost(bool, optional, tag = "5", default = "false")]
    pub client_streaming: Option<bool>,
    #[prost(bool, optional, tag = "6", default = "false")]
    pub server_streaming: Option<bool>,
}
