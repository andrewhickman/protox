#![allow(unused)]

use std::collections::btree_map::{self, BTreeMap};
use std::slice;

use bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::{DecodeError, Message};

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct OptionSet {
    fields: BTreeMap<u32, Value>,
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
}

impl OptionSet {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Message for OptionSet {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
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
            }
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
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
            }
        }
        len
    }

    fn clear(&mut self) {
        unimplemented!()
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
        unimplemented!()
    }
}
