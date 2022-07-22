use std::collections::btree_map::{self, BTreeMap};
use std::mem;

use bytes::{Buf, BufMut};
use logos::Span;
use prost::encoding::{DecodeContext, WireType};
use prost::{DecodeError, Message};

use crate::tag;
use crate::types::UninterpretedOption;

#[allow(unused)]
pub(crate) const FILE_JAVA_PACKAGE: i32 = 1;
#[allow(unused)]
pub(crate) const FILE_JAVA_OUTER_CLASSNAME: i32 = 8;
#[allow(unused)]
pub(crate) const FILE_JAVA_MULTIPLE_FILES: i32 = 10;
#[allow(unused)]
pub(crate) const FILE_JAVA_GENERATE_EQUALS_AND_HASH: i32 = 20;
#[allow(unused)]
pub(crate) const FILE_JAVA_STRING_CHECK_UTF8: i32 = 27;
#[allow(unused)]
pub(crate) const FILE_OPTIMIZE_FOR: i32 = 9;
#[allow(unused)]
pub(crate) const FILE_GO_PACKAGE: i32 = 11;
#[allow(unused)]
pub(crate) const FILE_CC_GENERIC_SERVICES: i32 = 16;
#[allow(unused)]
pub(crate) const FILE_JAVA_GENERIC_SERVICES: i32 = 17;
#[allow(unused)]
pub(crate) const FILE_PY_GENERIC_SERVICES: i32 = 18;
#[allow(unused)]
pub(crate) const FILE_PHP_GENERIC_SERVICES: i32 = 42;
#[allow(unused)]
pub(crate) const FILE_DEPRECATED: i32 = 23;
#[allow(unused)]
pub(crate) const FILE_CC_ENABLE_ARENAS: i32 = 31;
#[allow(unused)]
pub(crate) const FILE_OBJC_CLASS_PREFIX: i32 = 36;
#[allow(unused)]
pub(crate) const FILE_CSHARP_NAMESPACE: i32 = 37;
#[allow(unused)]
pub(crate) const FILE_SWIFT_PREFIX: i32 = 39;
#[allow(unused)]
pub(crate) const FILE_PHP_CLASS_PREFIX: i32 = 40;
#[allow(unused)]
pub(crate) const FILE_PHP_NAMESPACE: i32 = 41;
#[allow(unused)]
pub(crate) const FILE_PHP_METADATA_NAMESPACE: i32 = 44;
#[allow(unused)]
pub(crate) const FILE_RUBY_PACKAGE: i32 = 45;
#[allow(unused)]
pub(crate) const FILE_UNINTERPRETED_OPTION: i32 = 999;

#[allow(unused)]
pub(crate) const MESSAGE_MESSAGE_SET_WIRE_FORMAT: i32 = 1;
#[allow(unused)]
pub(crate) const MESSAGE_NO_STANDARD_DESCRIPTOR_ACCESSOR: i32 = 2;
#[allow(unused)]
pub(crate) const MESSAGE_DEPRECATED: i32 = 3;
#[allow(unused)]
pub(crate) const MESSAGE_MAP_ENTRY: i32 = 7;
#[allow(unused)]
pub(crate) const MESSAGE_UNINTERPRETED_OPTION: i32 = 999;

#[allow(unused)]
pub(crate) const FIELD_CTYPE: i32 = 1;
#[allow(unused)]
pub(crate) const FIELD_PACKED: i32 = 2;
#[allow(unused)]
pub(crate) const FIELD_JSTYPE: i32 = 6;
#[allow(unused)]
pub(crate) const FIELD_LAZY: i32 = 5;
#[allow(unused)]
pub(crate) const FIELD_DEPRECATED: i32 = 3;
#[allow(unused)]
pub(crate) const FIELD_WEAK: i32 = 10;
#[allow(unused)]
pub(crate) const FIELD_UNINTERPRETED_OPTION: i32 = 999;

#[allow(unused)]
pub(crate) const ENUM_ALLOW_ALIAS: i32 = 2;
#[allow(unused)]
pub(crate) const ENUM_DEPRECATED: i32 = 3;
#[allow(unused)]
pub(crate) const ENUM_UNINTERPRETED_OPTION: i32 = 999;

#[allow(unused)]
pub(crate) const ENUM_VALUE_DEPRECATED: i32 = 1;
#[allow(unused)]
pub(crate) const ENUM_VALUE_UNINTERPRETED_OPTION: i32 = 999;

#[allow(unused)]
pub(crate) const SERVICE_DEPRECATED: i32 = 33;
#[allow(unused)]
pub(crate) const SERVICE_UNINTERPRETED_OPTION: i32 = 999;

#[allow(unused)]
pub(crate) const METHOD_DEPRECATED: i32 = 33;
#[allow(unused)]
pub(crate) const METHOD_IDEMPOTENCY_LEVEL: i32 = 34;
#[allow(unused)]
pub(crate) const METHOD_UNINTERPRETED_OPTION: i32 = 999;

pub(crate) const UNINTERPRETED_OPTION: i32 = tag::UNINTERPRETED_OPTION;

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct OptionSet {
    fields: BTreeMap<u32, (Value, Span)>,
    uninterpreted_options: Vec<UninterpretedOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Float(f32),
    Double(f64),
    Bool(bool),
    Int32(i32),
    Int64(i64),
    Uint32(u32),
    Uint64(u64),
    Fixed32(u32),
    Fixed64(u64),
    Sint32(i32),
    Sint64(i64),
    Sfixed32(i32),
    Sfixed64(i64),
    String(String),
    Bytes(Vec<u8>),
    Message(OptionSet),
    Group(OptionSet),
    RepeatedFloat(Vec<f32>),
    RepeatedDouble(Vec<f64>),
    RepeatedBool(Vec<bool>),
    RepeatedInt32(Vec<i32>),
    RepeatedInt64(Vec<i64>),
    RepeatedUint32(Vec<u32>),
    RepeatedUint64(Vec<u64>),
    RepeatedFixed32(Vec<u32>),
    RepeatedFixed64(Vec<u64>),
    RepeatedSint32(Vec<i32>),
    RepeatedSint64(Vec<i64>),
    RepeatedSfixed32(Vec<i32>),
    RepeatedSfixed64(Vec<i64>),
    RepeatedString(Vec<String>),
    RepeatedBytes(Vec<Vec<u8>>),
    RepeatedMessage(Vec<OptionSet>),
    RepeatedGroup(Vec<OptionSet>),
}

impl OptionSet {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn uninterpreted(uninterpreted_options: Vec<UninterpretedOption>) -> Self {
        OptionSet {
            fields: BTreeMap::new(),
            uninterpreted_options: Vec::new(),
        }
    }

    pub fn take_uninterpreted(&mut self) -> Vec<UninterpretedOption> {
        mem::take(&mut self.uninterpreted_options)
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn get_message_mut(&mut self, number: i32, span: Span) -> &mut OptionSet {
        match self
            .fields
            .entry(number as u32)
            .or_insert_with(|| (Value::Message(OptionSet::new()), span))
        {
            (Value::Message(message), _) => message,
            _ => panic!("type mismatch"),
        }
    }

    pub fn set(&mut self, key: i32, value: Value) -> Result<(), Span> {
        match self.fields.entry(key as u32) {
            btree_map::Entry::Vacant(entry) => {
                entry.insert((value, Span::default()));
                Ok(())
            }
            btree_map::Entry::Occupied(entry) => Err(entry.get().1.clone()),
        }
    }

    pub fn set_repeated(&mut self, key: i32, value: Value, span: Span) {
        match self.fields.entry(key as u32) {
            btree_map::Entry::Vacant(entry) => {
                let value = match value {
                    Value::Float(value) => Value::RepeatedFloat(vec![value]),
                    Value::Double(value) => Value::RepeatedDouble(vec![value]),
                    Value::Bool(value) => Value::RepeatedBool(vec![value]),
                    Value::Int32(value) => Value::RepeatedInt32(vec![value]),
                    Value::Int64(value) => Value::RepeatedInt64(vec![value]),
                    Value::Uint32(value) => Value::RepeatedUint32(vec![value]),
                    Value::Uint64(value) => Value::RepeatedUint64(vec![value]),
                    Value::Fixed32(value) => Value::RepeatedFixed32(vec![value]),
                    Value::Fixed64(value) => Value::RepeatedFixed64(vec![value]),
                    Value::Sint32(value) => Value::RepeatedSint32(vec![value]),
                    Value::Sint64(value) => Value::RepeatedSint64(vec![value]),
                    Value::Sfixed32(value) => Value::RepeatedSfixed32(vec![value]),
                    Value::Sfixed64(value) => Value::RepeatedSfixed64(vec![value]),
                    Value::String(value) => Value::RepeatedString(vec![value]),
                    Value::Bytes(value) => Value::RepeatedBytes(vec![value]),
                    Value::Message(value) => Value::RepeatedMessage(vec![value]),
                    Value::Group(value) => Value::RepeatedGroup(vec![value]),
                    Value::RepeatedFloat(_)
                    | Value::RepeatedDouble(_)
                    | Value::RepeatedBool(_)
                    | Value::RepeatedInt32(_)
                    | Value::RepeatedInt64(_)
                    | Value::RepeatedUint32(_)
                    | Value::RepeatedUint64(_)
                    | Value::RepeatedFixed32(_)
                    | Value::RepeatedFixed64(_)
                    | Value::RepeatedSint32(_)
                    | Value::RepeatedSint64(_)
                    | Value::RepeatedSfixed32(_)
                    | Value::RepeatedSfixed64(_)
                    | Value::RepeatedString(_)
                    | Value::RepeatedBytes(_)
                    | Value::RepeatedMessage(_)
                    | Value::RepeatedGroup(_) => value,
                };

                entry.insert((value, span));
            }
            btree_map::Entry::Occupied(mut entry) => match (entry.get_mut(), value) {
                ((Value::RepeatedFloat(list), _), Value::Float(value)) => list.push(value),
                ((Value::RepeatedDouble(list), _), Value::Double(value)) => list.push(value),
                ((Value::RepeatedBool(list), _), Value::Bool(value)) => list.push(value),
                ((Value::RepeatedInt32(list), _), Value::Int32(value)) => list.push(value),
                ((Value::RepeatedInt64(list), _), Value::Int64(value)) => list.push(value),
                ((Value::RepeatedUint32(list), _), Value::Uint32(value)) => list.push(value),
                ((Value::RepeatedUint64(list), _), Value::Uint64(value)) => list.push(value),
                ((Value::RepeatedFixed32(list), _), Value::Fixed32(value)) => list.push(value),
                ((Value::RepeatedFixed64(list), _), Value::Fixed64(value)) => list.push(value),
                ((Value::RepeatedSint32(list), _), Value::Sint32(value)) => list.push(value),
                ((Value::RepeatedSint64(list), _), Value::Sint64(value)) => list.push(value),
                ((Value::RepeatedSfixed32(list), _), Value::Sfixed32(value)) => list.push(value),
                ((Value::RepeatedSfixed64(list), _), Value::Sfixed64(value)) => list.push(value),
                ((Value::RepeatedString(list), _), Value::String(value)) => list.push(value),
                ((Value::RepeatedBytes(list), _), Value::Bytes(value)) => list.push(value),
                ((Value::RepeatedMessage(list), _), Value::Message(value)) => list.push(value),
                ((Value::RepeatedGroup(list), _), Value::Group(value)) => list.push(value),
                ((Value::RepeatedFloat(list), _), Value::RepeatedFloat(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedDouble(list), _), Value::RepeatedDouble(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedBool(list), _), Value::RepeatedBool(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedInt32(list), _), Value::RepeatedInt32(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedInt64(list), _), Value::RepeatedInt64(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedUint32(list), _), Value::RepeatedUint32(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedUint64(list), _), Value::RepeatedUint64(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedFixed32(list), _), Value::RepeatedFixed32(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedFixed64(list), _), Value::RepeatedFixed64(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedSint32(list), _), Value::RepeatedSint32(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedSint64(list), _), Value::RepeatedSint64(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedSfixed32(list), _), Value::RepeatedSfixed32(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedSfixed64(list), _), Value::RepeatedSfixed64(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedString(list), _), Value::RepeatedString(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedBytes(list), _), Value::RepeatedBytes(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedMessage(list), _), Value::RepeatedMessage(values)) => {
                    list.extend(values)
                }
                ((Value::RepeatedGroup(list), _), Value::RepeatedGroup(values)) => {
                    list.extend(values)
                }
                _ => panic!("mismatched types"),
            },
        }
    }
}

impl Message for OptionSet {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        prost::encoding::message::encode_repeated(
            UNINTERPRETED_OPTION as u32,
            &self.uninterpreted_options,
            buf,
        );

        for (&tag, field) in &self.fields {
            match field {
                (Value::Float(value), _) => prost::encoding::float::encode(tag, value, buf),
                (Value::Double(value), _) => prost::encoding::double::encode(tag, value, buf),
                (Value::Bool(value), _) => prost::encoding::bool::encode(tag, value, buf),
                (Value::Int32(value), _) => prost::encoding::int32::encode(tag, value, buf),
                (Value::Int64(value), _) => prost::encoding::int64::encode(tag, value, buf),
                (Value::Uint32(value), _) => prost::encoding::uint32::encode(tag, value, buf),
                (Value::Uint64(value), _) => prost::encoding::uint64::encode(tag, value, buf),
                (Value::Fixed32(value), _) => prost::encoding::fixed32::encode(tag, value, buf),
                (Value::Fixed64(value), _) => prost::encoding::fixed64::encode(tag, value, buf),
                (Value::Sint32(value), _) => prost::encoding::sint32::encode(tag, value, buf),
                (Value::Sint64(value), _) => prost::encoding::sint64::encode(tag, value, buf),
                (Value::Sfixed32(value), _) => prost::encoding::sfixed32::encode(tag, value, buf),
                (Value::Sfixed64(value), _) => prost::encoding::sfixed64::encode(tag, value, buf),
                (Value::String(value), _) => prost::encoding::string::encode(tag, value, buf),
                (Value::Bytes(value), _) => prost::encoding::bytes::encode(tag, value, buf),
                (Value::Message(value), _) => prost::encoding::message::encode(tag, value, buf),
                (Value::Group(value), _) => prost::encoding::group::encode(tag, value, buf),
                (Value::RepeatedFloat(values), _) => {
                    prost::encoding::float::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedDouble(values), _) => {
                    prost::encoding::double::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedBool(values), _) => {
                    prost::encoding::bool::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedInt32(values), _) => {
                    prost::encoding::int32::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedInt64(values), _) => {
                    prost::encoding::int64::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedUint32(values), _) => {
                    prost::encoding::uint32::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedUint64(values), _) => {
                    prost::encoding::uint64::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedFixed32(values), _) => {
                    prost::encoding::fixed32::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedFixed64(values), _) => {
                    prost::encoding::fixed64::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedSint32(values), _) => {
                    prost::encoding::sint32::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedSint64(values), _) => {
                    prost::encoding::sint64::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedSfixed32(values), _) => {
                    prost::encoding::sfixed32::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedSfixed64(values), _) => {
                    prost::encoding::sfixed64::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedString(values), _) => {
                    prost::encoding::string::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedBytes(values), _) => {
                    prost::encoding::bytes::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedMessage(values), _) => {
                    prost::encoding::message::encode_repeated(tag, values, buf)
                }
                (Value::RepeatedGroup(values), _) => {
                    prost::encoding::group::encode_repeated(tag, values, buf)
                }
            }
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;

        len += prost::encoding::message::encoded_len_repeated(
            UNINTERPRETED_OPTION as u32,
            &self.uninterpreted_options,
        );

        for (&tag, field) in &self.fields {
            match field {
                (Value::Float(value), _) => len += prost::encoding::float::encoded_len(tag, value),
                (Value::Double(value), _) => {
                    len += prost::encoding::double::encoded_len(tag, value)
                }
                (Value::Bool(value), _) => len += prost::encoding::bool::encoded_len(tag, value),
                (Value::Int32(value), _) => len += prost::encoding::int32::encoded_len(tag, value),
                (Value::Int64(value), _) => len += prost::encoding::int64::encoded_len(tag, value),
                (Value::Uint32(value), _) => {
                    len += prost::encoding::uint32::encoded_len(tag, value)
                }
                (Value::Uint64(value), _) => {
                    len += prost::encoding::uint64::encoded_len(tag, value)
                }
                (Value::Fixed32(value), _) => {
                    len += prost::encoding::fixed32::encoded_len(tag, value)
                }
                (Value::Fixed64(value), _) => {
                    len += prost::encoding::fixed64::encoded_len(tag, value)
                }
                (Value::Sint32(value), _) => {
                    len += prost::encoding::sint32::encoded_len(tag, value)
                }
                (Value::Sint64(value), _) => {
                    len += prost::encoding::sint64::encoded_len(tag, value)
                }
                (Value::Sfixed32(value), _) => {
                    len += prost::encoding::sfixed32::encoded_len(tag, value)
                }
                (Value::Sfixed64(value), _) => {
                    len += prost::encoding::sfixed64::encoded_len(tag, value)
                }
                (Value::String(value), _) => {
                    len += prost::encoding::string::encoded_len(tag, value)
                }
                (Value::Bytes(value), _) => len += prost::encoding::bytes::encoded_len(tag, value),
                (Value::Message(value), _) => {
                    len += prost::encoding::message::encoded_len(tag, value)
                }
                (Value::Group(value), _) => len += prost::encoding::group::encoded_len(tag, value),
                (Value::RepeatedFloat(values), _) => {
                    len += prost::encoding::float::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedDouble(values), _) => {
                    len += prost::encoding::double::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedBool(values), _) => {
                    len += prost::encoding::bool::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedInt32(values), _) => {
                    len += prost::encoding::int32::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedInt64(values), _) => {
                    len += prost::encoding::int64::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedUint32(values), _) => {
                    len += prost::encoding::uint32::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedUint64(values), _) => {
                    len += prost::encoding::uint64::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedFixed32(values), _) => {
                    len += prost::encoding::fixed32::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedFixed64(values), _) => {
                    len += prost::encoding::fixed64::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedSint32(values), _) => {
                    len += prost::encoding::sint32::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedSint64(values), _) => {
                    len += prost::encoding::sint64::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedSfixed32(values), _) => {
                    len += prost::encoding::sfixed32::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedSfixed64(values), _) => {
                    len += prost::encoding::sfixed64::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedString(values), _) => {
                    len += prost::encoding::string::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedBytes(values), _) => {
                    len += prost::encoding::bytes::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedMessage(values), _) => {
                    len += prost::encoding::message::encoded_len_repeated(tag, values)
                }
                (Value::RepeatedGroup(values), _) => {
                    len += prost::encoding::group::encoded_len_repeated(tag, values)
                }
            }
        }
        len
    }

    fn clear(&mut self) {
        todo!("need this to parse extension options from bytes")
    }

    fn merge_field<B>(
        &mut self,
        _: u32,
        _: WireType,
        _: &mut B,
        _: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        todo!("need this to parse extension options from bytes")
    }
}
