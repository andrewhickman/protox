use std::collections::btree_map::{self, BTreeMap};
use std::mem;

use bytes::{Buf, BufMut};
use prost::encoding::{DecodeContext, WireType};
use prost::{DecodeError, Message};

use crate::tag;
use crate::types::UninterpretedOption;

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct OptionSet {
    fields: BTreeMap<u32, Value>,
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
            uninterpreted_options,
        }
    }

    pub fn take_uninterpreted(&mut self) -> Vec<UninterpretedOption> {
        mem::take(&mut self.uninterpreted_options)
    }

    pub fn get_message_mut(&mut self, number: i32) -> &mut OptionSet {
        match self
            .fields
            .entry(number as u32)
            .or_insert_with(|| Value::Message(OptionSet::new()))
        {
            Value::Message(message) => message,
            _ => panic!("type mismatch"),
        }
    }

    pub fn get(&self, key: i32) -> Option<&Value> {
        self.fields.get(&(key as u32))
    }

    pub fn set(&mut self, key: i32, value: Value) -> Result<(), ()> {
        match self.fields.entry(key as u32) {
            btree_map::Entry::Vacant(entry) => {
                entry.insert(value);
                Ok(())
            }
            btree_map::Entry::Occupied(_) => Err(()),
        }
    }

    pub fn set_repeated(&mut self, key: i32, value: Value) {
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

                entry.insert(value);
            }
            btree_map::Entry::Occupied(mut entry) => match (entry.get_mut(), value) {
                (Value::RepeatedFloat(list), Value::Float(value)) => list.push(value),
                (Value::RepeatedDouble(list), Value::Double(value)) => list.push(value),
                (Value::RepeatedBool(list), Value::Bool(value)) => list.push(value),
                (Value::RepeatedInt32(list), Value::Int32(value)) => list.push(value),
                (Value::RepeatedInt64(list), Value::Int64(value)) => list.push(value),
                (Value::RepeatedUint32(list), Value::Uint32(value)) => list.push(value),
                (Value::RepeatedUint64(list), Value::Uint64(value)) => list.push(value),
                (Value::RepeatedFixed32(list), Value::Fixed32(value)) => list.push(value),
                (Value::RepeatedFixed64(list), Value::Fixed64(value)) => list.push(value),
                (Value::RepeatedSint32(list), Value::Sint32(value)) => list.push(value),
                (Value::RepeatedSint64(list), Value::Sint64(value)) => list.push(value),
                (Value::RepeatedSfixed32(list), Value::Sfixed32(value)) => list.push(value),
                (Value::RepeatedSfixed64(list), Value::Sfixed64(value)) => list.push(value),
                (Value::RepeatedString(list), Value::String(value)) => list.push(value),
                (Value::RepeatedBytes(list), Value::Bytes(value)) => list.push(value),
                (Value::RepeatedMessage(list), Value::Message(value)) => list.push(value),
                (Value::RepeatedGroup(list), Value::Group(value)) => list.push(value),
                (Value::RepeatedFloat(list), Value::RepeatedFloat(values)) => list.extend(values),
                (Value::RepeatedDouble(list), Value::RepeatedDouble(values)) => list.extend(values),
                (Value::RepeatedBool(list), Value::RepeatedBool(values)) => list.extend(values),
                (Value::RepeatedInt32(list), Value::RepeatedInt32(values)) => list.extend(values),
                (Value::RepeatedInt64(list), Value::RepeatedInt64(values)) => list.extend(values),
                (Value::RepeatedUint32(list), Value::RepeatedUint32(values)) => list.extend(values),
                (Value::RepeatedUint64(list), Value::RepeatedUint64(values)) => list.extend(values),
                (Value::RepeatedFixed32(list), Value::RepeatedFixed32(values)) => {
                    list.extend(values)
                }
                (Value::RepeatedFixed64(list), Value::RepeatedFixed64(values)) => {
                    list.extend(values)
                }
                (Value::RepeatedSint32(list), Value::RepeatedSint32(values)) => list.extend(values),
                (Value::RepeatedSint64(list), Value::RepeatedSint64(values)) => list.extend(values),
                (Value::RepeatedSfixed32(list), Value::RepeatedSfixed32(values)) => {
                    list.extend(values)
                }
                (Value::RepeatedSfixed64(list), Value::RepeatedSfixed64(values)) => {
                    list.extend(values)
                }
                (Value::RepeatedString(list), Value::RepeatedString(values)) => list.extend(values),
                (Value::RepeatedBytes(list), Value::RepeatedBytes(values)) => list.extend(values),
                (Value::RepeatedMessage(list), Value::RepeatedMessage(values)) => {
                    list.extend(values)
                }
                (Value::RepeatedGroup(list), Value::RepeatedGroup(values)) => list.extend(values),
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
            tag::UNINTERPRETED_OPTION as u32,
            &self.uninterpreted_options,
            buf,
        );

        for (&tag, field) in &self.fields {
            match field {
                Value::Float(value) => prost::encoding::float::encode(tag, value, buf),
                Value::Double(value) => prost::encoding::double::encode(tag, value, buf),
                Value::Bool(value) => prost::encoding::bool::encode(tag, value, buf),
                Value::Int32(value) => prost::encoding::int32::encode(tag, value, buf),
                Value::Int64(value) => prost::encoding::int64::encode(tag, value, buf),
                Value::Uint32(value) => prost::encoding::uint32::encode(tag, value, buf),
                Value::Uint64(value) => prost::encoding::uint64::encode(tag, value, buf),
                Value::Fixed32(value) => prost::encoding::fixed32::encode(tag, value, buf),
                Value::Fixed64(value) => prost::encoding::fixed64::encode(tag, value, buf),
                Value::Sint32(value) => prost::encoding::sint32::encode(tag, value, buf),
                Value::Sint64(value) => prost::encoding::sint64::encode(tag, value, buf),
                Value::Sfixed32(value) => prost::encoding::sfixed32::encode(tag, value, buf),
                Value::Sfixed64(value) => prost::encoding::sfixed64::encode(tag, value, buf),
                Value::String(value) => prost::encoding::string::encode(tag, value, buf),
                Value::Bytes(value) => prost::encoding::bytes::encode(tag, value, buf),
                Value::Message(value) => prost::encoding::message::encode(tag, value, buf),
                Value::Group(value) => prost::encoding::group::encode(tag, value, buf),
                Value::RepeatedFloat(values) => {
                    prost::encoding::float::encode_repeated(tag, values, buf)
                }
                Value::RepeatedDouble(values) => {
                    prost::encoding::double::encode_repeated(tag, values, buf)
                }
                Value::RepeatedBool(values) => {
                    prost::encoding::bool::encode_repeated(tag, values, buf)
                }
                Value::RepeatedInt32(values) => {
                    prost::encoding::int32::encode_repeated(tag, values, buf)
                }
                Value::RepeatedInt64(values) => {
                    prost::encoding::int64::encode_repeated(tag, values, buf)
                }
                Value::RepeatedUint32(values) => {
                    prost::encoding::uint32::encode_repeated(tag, values, buf)
                }
                Value::RepeatedUint64(values) => {
                    prost::encoding::uint64::encode_repeated(tag, values, buf)
                }
                Value::RepeatedFixed32(values) => {
                    prost::encoding::fixed32::encode_repeated(tag, values, buf)
                }
                Value::RepeatedFixed64(values) => {
                    prost::encoding::fixed64::encode_repeated(tag, values, buf)
                }
                Value::RepeatedSint32(values) => {
                    prost::encoding::sint32::encode_repeated(tag, values, buf)
                }
                Value::RepeatedSint64(values) => {
                    prost::encoding::sint64::encode_repeated(tag, values, buf)
                }
                Value::RepeatedSfixed32(values) => {
                    prost::encoding::sfixed32::encode_repeated(tag, values, buf)
                }
                Value::RepeatedSfixed64(values) => {
                    prost::encoding::sfixed64::encode_repeated(tag, values, buf)
                }
                Value::RepeatedString(values) => {
                    prost::encoding::string::encode_repeated(tag, values, buf)
                }
                Value::RepeatedBytes(values) => {
                    prost::encoding::bytes::encode_repeated(tag, values, buf)
                }
                Value::RepeatedMessage(values) => {
                    prost::encoding::message::encode_repeated(tag, values, buf)
                }
                Value::RepeatedGroup(values) => {
                    prost::encoding::group::encode_repeated(tag, values, buf)
                }
            }
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;

        len += prost::encoding::message::encoded_len_repeated(
            tag::UNINTERPRETED_OPTION as u32,
            &self.uninterpreted_options,
        );

        for (&tag, field) in &self.fields {
            match field {
                Value::Float(value) => len += prost::encoding::float::encoded_len(tag, value),
                Value::Double(value) => len += prost::encoding::double::encoded_len(tag, value),
                Value::Bool(value) => len += prost::encoding::bool::encoded_len(tag, value),
                Value::Int32(value) => len += prost::encoding::int32::encoded_len(tag, value),
                Value::Int64(value) => len += prost::encoding::int64::encoded_len(tag, value),
                Value::Uint32(value) => len += prost::encoding::uint32::encoded_len(tag, value),
                Value::Uint64(value) => len += prost::encoding::uint64::encoded_len(tag, value),
                Value::Fixed32(value) => len += prost::encoding::fixed32::encoded_len(tag, value),
                Value::Fixed64(value) => len += prost::encoding::fixed64::encoded_len(tag, value),
                Value::Sint32(value) => len += prost::encoding::sint32::encoded_len(tag, value),
                Value::Sint64(value) => len += prost::encoding::sint64::encoded_len(tag, value),
                Value::Sfixed32(value) => len += prost::encoding::sfixed32::encoded_len(tag, value),
                Value::Sfixed64(value) => len += prost::encoding::sfixed64::encoded_len(tag, value),
                Value::String(value) => len += prost::encoding::string::encoded_len(tag, value),
                Value::Bytes(value) => len += prost::encoding::bytes::encoded_len(tag, value),
                Value::Message(value) => len += prost::encoding::message::encoded_len(tag, value),
                Value::Group(value) => len += prost::encoding::group::encoded_len(tag, value),
                Value::RepeatedFloat(values) => {
                    len += prost::encoding::float::encoded_len_repeated(tag, values)
                }
                Value::RepeatedDouble(values) => {
                    len += prost::encoding::double::encoded_len_repeated(tag, values)
                }
                Value::RepeatedBool(values) => {
                    len += prost::encoding::bool::encoded_len_repeated(tag, values)
                }
                Value::RepeatedInt32(values) => {
                    len += prost::encoding::int32::encoded_len_repeated(tag, values)
                }
                Value::RepeatedInt64(values) => {
                    len += prost::encoding::int64::encoded_len_repeated(tag, values)
                }
                Value::RepeatedUint32(values) => {
                    len += prost::encoding::uint32::encoded_len_repeated(tag, values)
                }
                Value::RepeatedUint64(values) => {
                    len += prost::encoding::uint64::encoded_len_repeated(tag, values)
                }
                Value::RepeatedFixed32(values) => {
                    len += prost::encoding::fixed32::encoded_len_repeated(tag, values)
                }
                Value::RepeatedFixed64(values) => {
                    len += prost::encoding::fixed64::encoded_len_repeated(tag, values)
                }
                Value::RepeatedSint32(values) => {
                    len += prost::encoding::sint32::encoded_len_repeated(tag, values)
                }
                Value::RepeatedSint64(values) => {
                    len += prost::encoding::sint64::encoded_len_repeated(tag, values)
                }
                Value::RepeatedSfixed32(values) => {
                    len += prost::encoding::sfixed32::encoded_len_repeated(tag, values)
                }
                Value::RepeatedSfixed64(values) => {
                    len += prost::encoding::sfixed64::encoded_len_repeated(tag, values)
                }
                Value::RepeatedString(values) => {
                    len += prost::encoding::string::encoded_len_repeated(tag, values)
                }
                Value::RepeatedBytes(values) => {
                    len += prost::encoding::bytes::encoded_len_repeated(tag, values)
                }
                Value::RepeatedMessage(values) => {
                    len += prost::encoding::message::encoded_len_repeated(tag, values)
                }
                Value::RepeatedGroup(values) => {
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
