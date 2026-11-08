//! Canonical interface encoding that distinguishes compiler structs from user
//! maps. Metadata fields may be omitted from structs, never from map entries:
//! labels and exported names such as `location`, `start`, and `end` are semantic.

use std::{collections::BTreeMap, fmt};

use serde::{Serialize, ser};

#[derive(Debug)]
pub(crate) struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for Error {}
impl ser::Error for Error {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self(message.to_string())
    }
}

type Result<T> = std::result::Result<T, Error>;

/// Type tags keep user maps distinct from Serde structs and enum variants.
/// Map entries are sorted before generic identifiers are alpha-renamed.
#[derive(Serialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum Value {
    Atom(Vec<u8>),
    Sequence(Vec<u8>, Vec<Value>),
    Map(Vec<(Value, Value)>),
    Struct {
        name: &'static str,
        variant: Option<&'static str>,
        fields: BTreeMap<&'static str, Value>,
    },
}

pub(crate) fn bytes<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>> {
    let mut value = value.serialize(Encoder)?;
    normalise(&mut value, &mut BTreeMap::new());
    serde_json::to_vec(&value).map_err(ser::Error::custom)
}

fn normalise(value: &mut Value, variables: &mut BTreeMap<u64, u64>) {
    // Inference links do not change the public type. Only inspect actual Type
    // and TypeVar variants; a user map containing these names stays untouched.
    if let Value::Struct {
        name: "Type",
        variant: Some("Var"),
        fields,
    } = value
        && let Some(Value::Struct {
            name: "TypeVar",
            variant: Some("Link"),
            fields,
        }) = fields.get("type_")
        && let Some(linked) = fields.get("type_")
    {
        *value = linked.clone();
        normalise(value, variables);
        return;
    }
    match value {
        Value::Struct {
            name,
            variant,
            fields,
        } => {
            if *name == "TypeVar"
                && matches!(variant, Some("Generic" | "Unbound"))
                && let Some(id) = fields.get_mut("id")
            {
                normalise_id(id, variables);
            }
            if *name == "TypeVariantConstructors"
                && let Some(Value::Sequence(_, ids)) = fields.get_mut("type_parameters_ids")
            {
                for id in ids {
                    normalise_id(id, variables);
                }
            }
            for child in fields.values_mut() {
                normalise(child, variables);
            }
        }
        Value::Map(entries) => {
            for (key, child) in entries {
                normalise(key, variables);
                normalise(child, variables);
            }
        }
        Value::Sequence(_, values) => {
            for child in values {
                normalise(child, variables);
            }
        }
        Value::Atom(_) => {}
    }
}

fn normalise_id(value: &mut Value, variables: &mut BTreeMap<u64, u64>) {
    let Value::Atom(bytes) = value else { return };
    if bytes.len() != 9 || bytes[0] != 0x0a {
        return;
    }
    let original = u64::from_le_bytes(bytes[1..].try_into().expect("u64 is eight bytes"));
    let next = variables.len() as u64;
    let canonical = *variables.entry(original).or_insert(next);
    bytes[1..].copy_from_slice(&canonical.to_le_bytes());
}

struct Encoder;

fn atom(tag: u8, data: &[u8]) -> Value {
    let mut bytes = vec![tag];
    bytes.extend_from_slice(data);
    Value::Atom(bytes)
}

fn prefix(tag: u8, name: &str) -> Vec<u8> {
    let mut bytes = vec![tag];
    bytes.extend_from_slice(name.as_bytes());
    bytes
}

fn variant_prefix(tag: u8, name: &str, variant: &str) -> Vec<u8> {
    let mut bytes = prefix(tag, name);
    bytes.push(0);
    bytes.extend_from_slice(variant.as_bytes());
    bytes
}

struct Sequence {
    prefix: Vec<u8>,
    values: Vec<Value>,
}

impl Sequence {
    fn new(prefix: Vec<u8>, len: Option<usize>) -> Self {
        Self {
            prefix,
            values: Vec::with_capacity(len.unwrap_or(0)),
        }
    }
    fn push<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        self.values.push(value.serialize(Encoder)?);
        Ok(())
    }
    fn finish(self) -> Value {
        Value::Sequence(self.prefix, self.values)
    }
}

struct Fields {
    name: &'static str,
    variant: Option<&'static str>,
    fields: BTreeMap<&'static str, Value>,
}

impl Fields {
    fn new(name: &'static str, variant: Option<&'static str>) -> Self {
        Self {
            name,
            variant,
            fields: BTreeMap::new(),
        }
    }
    fn field<T: Serialize + ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()> {
        // SrcSpan is identified by its Rust type, not by an object's field
        // names. In particular, FieldMap { fields: { start: 0, end: 1 } } is
        // retained and changing label positions invalidates imported calls.
        if self.name == "SrcSpan"
            || matches!(
                key,
                "location"
                    | "definition_location"
                    | "src_path"
                    | "line_numbers"
                    | "documentation"
                    | "warnings"
                    | "references"
                    | "contains_echo"
                    | "arguments_start_position"
                    | "operator_start"
            )
        {
            return Ok(());
        }
        self.fields.insert(key, value.serialize(Encoder)?);
        Ok(())
    }
    fn finish(self) -> Value {
        Value::Struct {
            name: self.name,
            variant: self.variant,
            fields: self.fields,
        }
    }
}

struct Map {
    entries: Vec<(Value, Value)>,
    key: Option<Value>,
}

macro_rules! numeric {
    ($($method:ident($type:ty) => $tag:literal),* $(,)?) => {
        $(fn $method(self, value: $type) -> Result<Value> {
            Ok(atom($tag, &value.to_le_bytes()))
        })*
    };
}

impl ser::Serializer for Encoder {
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = Sequence;
    type SerializeTuple = Sequence;
    type SerializeTupleStruct = Sequence;
    type SerializeTupleVariant = Sequence;
    type SerializeMap = Map;
    type SerializeStruct = Fields;
    type SerializeStructVariant = Fields;

    numeric! {
        serialize_i8(i8) => 0x02, serialize_i16(i16) => 0x03,
        serialize_i32(i32) => 0x04, serialize_i64(i64) => 0x05,
        serialize_i128(i128) => 0x06, serialize_u8(u8) => 0x07,
        serialize_u16(u16) => 0x08, serialize_u32(u32) => 0x09,
        serialize_u64(u64) => 0x0a, serialize_u128(u128) => 0x0b,
    }
    fn serialize_bool(self, value: bool) -> Result<Value> {
        Ok(atom(0x01, &[u8::from(value)]))
    }
    fn serialize_f32(self, value: f32) -> Result<Value> {
        Ok(atom(0x0c, &value.to_bits().to_le_bytes()))
    }
    fn serialize_f64(self, value: f64) -> Result<Value> {
        Ok(atom(0x0d, &value.to_bits().to_le_bytes()))
    }
    fn serialize_char(self, value: char) -> Result<Value> {
        Ok(atom(0x0e, &u32::from(value).to_le_bytes()))
    }
    fn serialize_str(self, value: &str) -> Result<Value> {
        Ok(atom(0x0f, value.as_bytes()))
    }
    fn serialize_bytes(self, value: &[u8]) -> Result<Value> {
        Ok(atom(0x10, value))
    }
    fn serialize_none(self) -> Result<Value> {
        Ok(atom(0x11, &[]))
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Value> {
        Ok(Value::Sequence(vec![0x12], vec![value.serialize(Encoder)?]))
    }
    fn serialize_unit(self) -> Result<Value> {
        Ok(atom(0x13, &[]))
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Value> {
        Ok(Value::Atom(prefix(0x14, name)))
    }
    fn serialize_unit_variant(
        self,
        name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<Value> {
        Ok(Value::Atom(variant_prefix(0x15, name, variant)))
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Value> {
        Ok(Value::Sequence(
            prefix(0x16, name),
            vec![value.serialize(Encoder)?],
        ))
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value> {
        Ok(Value::Sequence(
            variant_prefix(0x17, name, variant),
            vec![value.serialize(Encoder)?],
        ))
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Sequence> {
        Ok(Sequence::new(vec![0x18], len))
    }
    fn serialize_tuple(self, len: usize) -> Result<Sequence> {
        Ok(Sequence::new(vec![0x19], Some(len)))
    }
    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Sequence> {
        Ok(Sequence::new(prefix(0x1a, name), Some(len)))
    }
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Sequence> {
        Ok(Sequence::new(
            variant_prefix(0x1b, name, variant),
            Some(len),
        ))
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Map> {
        Ok(Map {
            entries: Vec::with_capacity(len.unwrap_or(0)),
            key: None,
        })
    }
    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Fields> {
        Ok(Fields::new(name, None))
    }
    fn serialize_struct_variant(
        self,
        name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Fields> {
        Ok(Fields::new(name, Some(variant)))
    }
    fn is_human_readable(&self) -> bool {
        false
    }
}

macro_rules! sequence_trait {
    ($trait:ident, $method:ident) => {
        impl ser::$trait for Sequence {
            type Ok = Value;
            type Error = Error;
            fn $method<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
                self.push(value)
            }
            fn end(self) -> Result<Value> {
                Ok(self.finish())
            }
        }
    };
}
sequence_trait!(SerializeSeq, serialize_element);
sequence_trait!(SerializeTuple, serialize_element);
sequence_trait!(SerializeTupleStruct, serialize_field);
sequence_trait!(SerializeTupleVariant, serialize_field);

macro_rules! struct_trait {
    ($trait:ident) => {
        impl ser::$trait for Fields {
            type Ok = Value;
            type Error = Error;
            fn serialize_field<T: Serialize + ?Sized>(
                &mut self,
                key: &'static str,
                value: &T,
            ) -> Result<()> {
                self.field(key, value)
            }
            fn end(self) -> Result<Value> {
                Ok(self.finish())
            }
        }
    };
}
struct_trait!(SerializeStruct);
struct_trait!(SerializeStructVariant);

impl ser::SerializeMap for Map {
    type Ok = Value;
    type Error = Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<()> {
        if self.key.is_some() {
            return Err(Error("map key without value".into()));
        }
        self.key = Some(key.serialize(Encoder)?);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        let key = self
            .key
            .take()
            .ok_or_else(|| Error("map value without key".into()))?;
        self.entries.push((key, value.serialize(Encoder)?));
        Ok(())
    }
    fn end(mut self) -> Result<Value> {
        if self.key.is_some() {
            return Err(Error("map ended before value".into()));
        }
        self.entries.sort_unstable();
        Ok(Value::Map(self.entries))
    }
}
