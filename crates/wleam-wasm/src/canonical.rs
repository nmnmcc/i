//! Deterministic Serde encoding for cache fingerprints, including composite map
//! keys. Every value has a type tag; variable-length values are length framed.

use std::fmt;

use serde::{Serialize, ser};

#[derive(Debug)]
pub(super) struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl ser::Error for Error {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self(message.to_string())
    }
}

type Result<T> = std::result::Result<T, Error>;

pub(super) fn bytes<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>> {
    value.serialize(Encoder)
}

struct Encoder;

fn scalar(tag: u8, data: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(1 + data.len());
    output.push(tag);
    output.extend_from_slice(data);
    output
}

fn framed(output: &mut Vec<u8>, data: &[u8]) {
    output.extend_from_slice(&(data.len() as u64).to_le_bytes());
    output.extend_from_slice(data);
}

fn blob(tag: u8, data: &[u8]) -> Vec<u8> {
    let mut output = vec![tag];
    framed(&mut output, data);
    output
}

fn named(tag: u8, name: &str) -> Vec<u8> {
    blob(tag, name.as_bytes())
}

fn variant(tag: u8, name: &str, index: u32, variant: &str) -> Vec<u8> {
    let mut output = named(tag, name);
    output.extend_from_slice(&index.to_le_bytes());
    framed(&mut output, variant.as_bytes());
    output
}

struct Sequence {
    prefix: Vec<u8>,
    fields: Vec<Vec<u8>>,
}

impl Sequence {
    fn new(prefix: Vec<u8>, len: Option<usize>) -> Self {
        Self {
            prefix,
            fields: Vec::with_capacity(len.unwrap_or(0)),
        }
    }
    fn push<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        self.fields.push(bytes(value)?);
        Ok(())
    }
    fn field<T: Serialize + ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()> {
        let mut encoded = blob(0x30, key.as_bytes());
        framed(&mut encoded, &bytes(value)?);
        self.fields.push(encoded);
        Ok(())
    }
    fn finish(mut self) -> Vec<u8> {
        self.prefix
            .extend_from_slice(&(self.fields.len() as u64).to_le_bytes());
        for field in self.fields {
            framed(&mut self.prefix, &field);
        }
        self.prefix
    }
}

struct Map {
    entries: Vec<(Vec<u8>, Vec<u8>)>,
    key: Option<Vec<u8>>,
}

impl ser::Serializer for Encoder {
    type Ok = Vec<u8>;
    type Error = Error;
    type SerializeSeq = Sequence;
    type SerializeTuple = Sequence;
    type SerializeTupleStruct = Sequence;
    type SerializeTupleVariant = Sequence;
    type SerializeMap = Map;
    type SerializeStruct = Sequence;
    type SerializeStructVariant = Sequence;

    fn serialize_bool(self, value: bool) -> Result<Vec<u8>> {
        Ok(scalar(0x01, &[u8::from(value)]))
    }
    fn serialize_i8(self, value: i8) -> Result<Vec<u8>> {
        Ok(scalar(0x02, &value.to_le_bytes()))
    }
    fn serialize_i16(self, value: i16) -> Result<Vec<u8>> {
        Ok(scalar(0x03, &value.to_le_bytes()))
    }
    fn serialize_i32(self, value: i32) -> Result<Vec<u8>> {
        Ok(scalar(0x04, &value.to_le_bytes()))
    }
    fn serialize_i64(self, value: i64) -> Result<Vec<u8>> {
        Ok(scalar(0x05, &value.to_le_bytes()))
    }
    fn serialize_i128(self, value: i128) -> Result<Vec<u8>> {
        Ok(scalar(0x06, &value.to_le_bytes()))
    }
    fn serialize_u8(self, value: u8) -> Result<Vec<u8>> {
        Ok(scalar(0x07, &value.to_le_bytes()))
    }
    fn serialize_u16(self, value: u16) -> Result<Vec<u8>> {
        Ok(scalar(0x08, &value.to_le_bytes()))
    }
    fn serialize_u32(self, value: u32) -> Result<Vec<u8>> {
        Ok(scalar(0x09, &value.to_le_bytes()))
    }
    fn serialize_u64(self, value: u64) -> Result<Vec<u8>> {
        Ok(scalar(0x0a, &value.to_le_bytes()))
    }
    fn serialize_u128(self, value: u128) -> Result<Vec<u8>> {
        Ok(scalar(0x0b, &value.to_le_bytes()))
    }
    fn serialize_f32(self, value: f32) -> Result<Vec<u8>> {
        Ok(scalar(0x0c, &value.to_bits().to_le_bytes()))
    }
    fn serialize_f64(self, value: f64) -> Result<Vec<u8>> {
        Ok(scalar(0x0d, &value.to_bits().to_le_bytes()))
    }
    fn serialize_char(self, value: char) -> Result<Vec<u8>> {
        Ok(scalar(0x0e, &u32::from(value).to_le_bytes()))
    }
    fn serialize_str(self, value: &str) -> Result<Vec<u8>> {
        Ok(blob(0x0f, value.as_bytes()))
    }
    fn serialize_bytes(self, value: &[u8]) -> Result<Vec<u8>> {
        Ok(blob(0x10, value))
    }
    fn serialize_none(self) -> Result<Vec<u8>> {
        Ok(vec![0x11])
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Vec<u8>> {
        Ok(blob(0x12, &bytes(value)?))
    }
    fn serialize_unit(self) -> Result<Vec<u8>> {
        Ok(vec![0x13])
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Vec<u8>> {
        Ok(named(0x14, name))
    }
    fn serialize_unit_variant(
        self,
        name: &'static str,
        index: u32,
        case: &'static str,
    ) -> Result<Vec<u8>> {
        Ok(variant(0x15, name, index, case))
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Vec<u8>> {
        let mut output = named(0x16, name);
        framed(&mut output, &bytes(value)?);
        Ok(output)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        index: u32,
        case: &'static str,
        value: &T,
    ) -> Result<Vec<u8>> {
        let mut output = variant(0x17, name, index, case);
        framed(&mut output, &bytes(value)?);
        Ok(output)
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Sequence> {
        Ok(Sequence::new(vec![0x18], len))
    }
    fn serialize_tuple(self, len: usize) -> Result<Sequence> {
        Ok(Sequence::new(vec![0x19], Some(len)))
    }
    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Sequence> {
        Ok(Sequence::new(named(0x1a, name), Some(len)))
    }
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        index: u32,
        case: &'static str,
        len: usize,
    ) -> Result<Sequence> {
        Ok(Sequence::new(variant(0x1b, name, index, case), Some(len)))
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Map> {
        Ok(Map {
            entries: Vec::with_capacity(len.unwrap_or(0)),
            key: None,
        })
    }
    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Sequence> {
        Ok(Sequence::new(named(0x1d, name), Some(len)))
    }
    fn serialize_struct_variant(
        self,
        name: &'static str,
        index: u32,
        case: &'static str,
        len: usize,
    ) -> Result<Sequence> {
        Ok(Sequence::new(variant(0x1e, name, index, case), Some(len)))
    }
    fn is_human_readable(&self) -> bool {
        false
    }
}

macro_rules! sequence_trait {
    ($trait:ident, $method:ident) => {
        impl ser::$trait for Sequence {
            type Ok = Vec<u8>;
            type Error = Error;
            fn $method<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
                self.push(value)
            }
            fn end(self) -> Result<Vec<u8>> {
                Ok(self.finish())
            }
        }
    };
}
sequence_trait!(SerializeSeq, serialize_element);
sequence_trait!(SerializeTuple, serialize_element);
sequence_trait!(SerializeTupleStruct, serialize_field);
sequence_trait!(SerializeTupleVariant, serialize_field);

impl ser::SerializeStruct for Sequence {
    type Ok = Vec<u8>;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        self.field(key, value)
    }
    fn end(self) -> Result<Vec<u8>> {
        Ok(self.finish())
    }
}

impl ser::SerializeStructVariant for Sequence {
    type Ok = Vec<u8>;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        self.field(key, value)
    }
    fn end(self) -> Result<Vec<u8>> {
        Ok(self.finish())
    }
}

impl ser::SerializeMap for Map {
    type Ok = Vec<u8>;
    type Error = Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<()> {
        if self.key.is_some() {
            return Err(Error("map key without preceding value".into()));
        }
        self.key = Some(bytes(key)?);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        let key = self
            .key
            .take()
            .ok_or_else(|| Error("map value without key".into()))?;
        self.entries.push((key, bytes(value)?));
        Ok(())
    }
    fn end(mut self) -> Result<Vec<u8>> {
        if self.key.is_some() {
            return Err(Error("map ended before value".into()));
        }
        // Sort encoded keys, and values for duplicate keys emitted by a custom
        // serializer, so map iteration order cannot affect a fingerprint.
        self.entries.sort_unstable();
        let mut output = vec![0x1c];
        output.extend_from_slice(&(self.entries.len() as u64).to_le_bytes());
        for (key, value) in self.entries {
            framed(&mut output, &key);
            framed(&mut output, &value);
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Serialize)]
    enum Nested {
        Unit,
        Tuple(i64, String),
        Fields { parts: Vec<u32> },
    }

    #[test]
    fn composite_keys_and_nested_variants_are_order_independent() {
        let first = HashMap::from([
            (("left".to_owned(), 1_u32), Nested::Tuple(-4, "x".into())),
            (
                ("right".to_owned(), 2_u32),
                Nested::Fields {
                    parts: vec![1, 2, 3],
                },
            ),
        ]);
        let second = HashMap::from([
            (
                ("right".to_owned(), 2_u32),
                Nested::Fields {
                    parts: vec![1, 2, 3],
                },
            ),
            (("left".to_owned(), 1_u32), Nested::Tuple(-4, "x".into())),
        ]);
        assert_eq!(bytes(&first).unwrap(), bytes(&second).unwrap());
    }

    #[test]
    fn type_tags_boundaries_and_float_bits_are_distinct() {
        assert_ne!(bytes(&1_i64).unwrap(), bytes(&1_u64).unwrap());
        assert_ne!(
            bytes(&vec![1_u32, 2]).unwrap(),
            bytes(&(1_u32, 2_u32)).unwrap()
        );
        assert_ne!(bytes(&("ab", "c")).unwrap(), bytes(&("a", "bc")).unwrap());
        assert_ne!(bytes(&0.0_f64).unwrap(), bytes(&-0.0_f64).unwrap());
        assert_ne!(bytes(&Nested::Unit).unwrap(), bytes(&()).unwrap());
        assert_ne!(bytes(&None::<u32>).unwrap(), bytes(&Some(0_u32)).unwrap());
    }
}
