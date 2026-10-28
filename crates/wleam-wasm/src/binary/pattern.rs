//! Pattern matching and bit-array construction emitted directly as Wasm instructions.

use num_traits::{Signed, ToPrimitive};
use wasm_encoder::{Instruction as I, ValType};

use super::{Backend, Context, EQREF, Label, heap};
use crate::{
    Error,
    ir::{BitKind, BitOptions, BitSegment, Expr, Pattern},
};

impl Backend<'_> {
    pub(super) fn pattern(
        &mut self,
        pattern: &Pattern,
        value: u32,
        fail: Label,
        context: &mut Context,
    ) -> Result<(), Error> {
        match pattern {
            Pattern::Discard => {}
            Pattern::Variable(name) => {
                let local = context.variable(name);
                context.emit(I::LocalGet(value));
                context.emit(I::LocalSet(local));
            }
            Pattern::Alias { name, pattern } => {
                let local = context.variable(name);
                context.emit(I::LocalGet(value));
                context.emit(I::LocalSet(local));
                self.pattern(pattern, value, fail, context)?;
            }
            Pattern::Nil => {
                context.emit(I::LocalGet(value));
                context.emit(I::RefIsNull);
                reject_false(context, fail);
            }
            Pattern::Bool(boolean) => {
                self.pattern_tag(value, 1, fail, context)?;
                context.emit(I::LocalGet(value));
                self.call("bool_get", context)?;
                context.emit(I::I32Const(i32::from(*boolean)));
                context.emit(I::I32Eq);
                reject_false(context, fail);
            }
            Pattern::Int(integer) => {
                context.emit(I::LocalGet(value));
                self.integer(integer, context)?;
                self.call("value_eq", context)?;
                reject_false(context, fail);
            }
            Pattern::Float(float) => {
                context.emit(I::LocalGet(value));
                context.emit(I::F64Const((*float).into()));
                self.call("float_new", context)?;
                self.call("value_eq", context)?;
                reject_false(context, fail);
            }
            Pattern::String(string) => {
                context.emit(I::LocalGet(value));
                self.string(string, context)?;
                self.call("value_eq", context)?;
                reject_false(context, fail);
            }
            Pattern::Value(expression) => {
                context.emit(I::LocalGet(value));
                self.expression(expression, context, false)?;
                self.call("value_eq", context)?;
                reject_false(context, fail);
            }
            Pattern::Tuple(fields) => {
                self.pattern_tag(value, 4, fail, context)?;
                self.pattern_fields(fields, value, fail, context)?;
            }
            Pattern::Record { tag, arguments } => {
                self.pattern_tag(value, 5, fail, context)?;
                context.emit(I::LocalGet(value));
                context.emit(I::RefCastNonNull(heap(self.types.obj)));
                context.emit(I::StructGet {
                    struct_type_index: self.types.obj,
                    field_index: 1,
                });
                context.emit(I::I32Const(*tag as i32));
                context.emit(I::I32Eq);
                reject_false(context, fail);
                self.pattern_fields(arguments, value, fail, context)?;
            }
            Pattern::List { elements, tail } => {
                let rest = context.local(EQREF);
                context.emit(I::LocalGet(value));
                context.emit(I::LocalSet(rest));
                for pattern in elements {
                    context.emit(I::LocalGet(rest));
                    self.call("list_is_empty", context)?;
                    context.br_if(fail);
                    let element = context.local(EQREF);
                    context.emit(I::LocalGet(rest));
                    self.call("list_head", context)?;
                    context.emit(I::LocalSet(element));
                    self.pattern(pattern, element, fail, context)?;
                    context.emit(I::LocalGet(rest));
                    self.call("list_tail", context)?;
                    context.emit(I::LocalSet(rest));
                }
                if let Some(tail) = tail {
                    self.pattern(tail, rest, fail, context)?;
                } else {
                    context.emit(I::LocalGet(rest));
                    self.call("list_is_empty", context)?;
                    reject_false(context, fail);
                }
            }
            Pattern::StringPrefix { prefix, left, rest } => {
                let literal = context.local(EQREF);
                self.string(prefix, context)?;
                context.emit(I::LocalSet(literal));
                context.emit(I::LocalGet(value));
                context.emit(I::LocalGet(literal));
                self.call("string_has_prefix", context)?;
                reject_false(context, fail);
                if let Some(name) = left {
                    let local = context.variable(name);
                    context.emit(I::LocalGet(literal));
                    context.emit(I::LocalSet(local));
                }
                if let Some(name) = rest {
                    let local = context.variable(name);
                    context.emit(I::LocalGet(value));
                    context.emit(I::I32Const(prefix.len() as i32));
                    self.call("string_suffix", context)?;
                    context.emit(I::LocalSet(local));
                }
            }
            Pattern::BitArray(segments) => self.bit_pattern(segments, value, fail, context)?,
        }
        Ok(())
    }

    fn pattern_tag(
        &self,
        value: u32,
        tag: i32,
        fail: Label,
        context: &mut Context,
    ) -> Result<(), Error> {
        context.emit(I::LocalGet(value));
        self.call("value_tag", context)?;
        context.emit(I::I32Const(tag));
        context.emit(I::I32Eq);
        reject_false(context, fail);
        Ok(())
    }

    fn pattern_fields(
        &mut self,
        fields: &[Pattern],
        value: u32,
        fail: Label,
        context: &mut Context,
    ) -> Result<(), Error> {
        for (index, pattern) in fields.iter().enumerate() {
            let local = context.local(EQREF);
            context.emit(I::LocalGet(value));
            context.emit(I::I32Const(index as i32));
            self.call("field", context)?;
            context.emit(I::LocalSet(local));
            self.pattern(pattern, local, fail, context)?;
        }
        Ok(())
    }

    /// An explicit size uses the checked runtime conversion; the caller emits
    /// the default width itself when this returns false.
    fn bit_width(
        &mut self,
        options: &BitOptions,
        clamp_negative: bool,
        context: &mut Context,
    ) -> Result<bool, Error> {
        let Some(size) = &options.size else {
            return Ok(false);
        };
        self.expression(size, context, false)?;
        context.emit(I::I64Const(i64::from(options.unit)));
        context.emit(I::I32Const(i32::from(clamp_negative)));
        self.call("bits_width", context)?;
        Ok(true)
    }

    pub(super) fn bit_array(
        &mut self,
        segments: &[BitSegment<Expr>],
        context: &mut Context,
    ) -> Result<(), Error> {
        if let Some((bytes, length)) = constant_bits(segments) {
            context.emit(I::I32Const(9));
            context.emit(I::I32Const(0));
            context.emit(I::I64Const(length as i64));
            context.emit(I::F64Const(0.0.into()));
            self.byte_array(&bytes, context)?;
            context.emit(I::RefNull(heap(self.types.values)));
            context.emit(I::RefNull(heap(self.types.limbs)));
            context.emit(I::RefNull(heap(self.types.call)));
            context.emit(I::StructNew(self.types.obj));
            return Ok(());
        }
        let output = context.local(EQREF);
        self.call("bits_empty", context)?;
        context.emit(I::LocalSet(output));
        for segment in segments {
            let value = context.local(EQREF);
            self.expression(&segment.value, context, false)?;
            context.emit(I::LocalSet(value));
            context.emit(I::LocalGet(output));
            context.emit(I::LocalGet(value));
            let endian = i32::from(segment.options.little_endian);
            match segment.options.kind {
                BitKind::Int | BitKind::Float => {
                    if !self.bit_width(&segment.options, true, context)? {
                        context.emit(I::I64Const(if segment.options.kind == BitKind::Int {
                            8
                        } else {
                            64
                        }));
                    }
                    context.emit(I::I32Const(endian));
                    self.call(
                        if segment.options.kind == BitKind::Int {
                            "bits_from_int"
                        } else {
                            "bits_from_float"
                        },
                        context,
                    )?;
                }
                BitKind::Bits | BitKind::Bytes => {
                    context.emit(I::I64Const(0));
                    if !self.bit_width(&segment.options, true, context)? {
                        context.emit(I::LocalGet(value));
                        self.call("bits_len64", context)?;
                    }
                    self.call("bits_slice", context)?;
                }
                kind => {
                    let (encoding, codepoint) = utf_kind(kind);
                    context.emit(I::I32Const(encoding));
                    context.emit(I::I32Const(endian));
                    context.emit(I::I32Const(i32::from(codepoint)));
                    self.call("bits_from_utf", context)?;
                }
            }
            self.call("bits_concat", context)?;
            context.emit(I::LocalSet(output));
        }
        context.emit(I::LocalGet(output));
        Ok(())
    }

    fn bit_pattern(
        &mut self,
        segments: &[BitSegment<Pattern>],
        source: u32,
        fail: Label,
        context: &mut Context,
    ) -> Result<(), Error> {
        let offset = context.local(ValType::I64);
        let width = context.local(ValType::I64);
        let extracted = context.local(EQREF);
        context.emit(I::I64Const(0));
        context.emit(I::LocalSet(offset));
        for segment in segments {
            let endian = i32::from(segment.options.little_endian);
            let signed = i32::from(segment.options.signed);
            match segment.options.kind {
                kind @ (BitKind::Utf8 | BitKind::Utf16 | BitKind::Utf32) => {
                    let (encoding, _) = utf_kind(kind);
                    if matches!(segment.value, Pattern::Discard) {
                        self.read_utf(
                            source, offset, width, extracted, encoding, endian, fail, context,
                        )?;
                    } else {
                        let mut value_pattern = &segment.value;
                        let mut aliases = Vec::new();
                        while let Pattern::Alias { name, pattern } = value_pattern {
                            aliases.push(name);
                            value_pattern = pattern;
                        }
                        let literal = context.local(EQREF);
                        match value_pattern {
                            Pattern::String(value) => self.string(value, context)?,
                            Pattern::Value(expression) => {
                                self.expression(expression, context, false)?
                            }
                            _ => {
                                return Err(Error::Codegen(
                                    "a UTF string pattern must be a literal".into(),
                                ));
                            }
                        }
                        context.emit(I::LocalSet(literal));
                        let encoded = context.local(EQREF);
                        context.emit(I::LocalGet(literal));
                        context.emit(I::I32Const(encoding));
                        context.emit(I::I32Const(endian));
                        context.emit(I::I32Const(0));
                        self.call("bits_from_utf", context)?;
                        context.emit(I::LocalSet(encoded));
                        context.emit(I::LocalGet(encoded));
                        self.call("bits_len64", context)?;
                        context.emit(I::LocalSet(width));
                        self.check_bit_range(source, offset, width, fail, context)?;
                        context.emit(I::LocalGet(source));
                        context.emit(I::LocalGet(offset));
                        context.emit(I::LocalGet(width));
                        self.call("bits_slice", context)?;
                        context.emit(I::LocalGet(encoded));
                        self.call("bits_match_equal", context)?;
                        reject_false(context, fail);
                        for name in aliases {
                            let local = context.variable(name);
                            context.emit(I::LocalGet(literal));
                            context.emit(I::LocalSet(local));
                        }
                    }
                }
                kind @ (BitKind::Utf8Codepoint
                | BitKind::Utf16Codepoint
                | BitKind::Utf32Codepoint) => {
                    let (encoding, _) = utf_kind(kind);
                    self.read_utf(
                        source, offset, width, extracted, encoding, endian, fail, context,
                    )?;
                    self.pattern(&segment.value, extracted, fail, context)?;
                }
                kind => {
                    if !self.bit_width(&segment.options, false, context)? {
                        match kind {
                            BitKind::Int => context.emit(I::I64Const(8)),
                            BitKind::Float => context.emit(I::I64Const(64)),
                            _ => {
                                context.emit(I::LocalGet(source));
                                self.call("bits_len64", context)?;
                                context.emit(I::LocalGet(offset));
                                context.emit(I::I64Sub);
                            }
                        }
                    }
                    context.emit(I::LocalSet(width));
                    self.check_bit_range(source, offset, width, fail, context)?;
                    if matches!(kind, BitKind::Bits | BitKind::Bytes)
                        && segment.options.size.is_none()
                        && segment.options.unit > 1
                    {
                        context.emit(I::LocalGet(width));
                        context.emit(I::I64Const(i64::from(segment.options.unit)));
                        context.emit(I::I64RemU);
                        context.emit(I::I64Const(0));
                        context.emit(I::I64Ne);
                        context.br_if(fail);
                    }
                    if kind == BitKind::Float {
                        context.emit(I::LocalGet(source));
                        context.emit(I::LocalGet(offset));
                        context.emit(I::LocalGet(width));
                        context.emit(I::I32Const(endian));
                        self.call("bits_float_finite", context)?;
                        reject_false(context, fail);
                    }
                    context.emit(I::LocalGet(source));
                    context.emit(I::LocalGet(offset));
                    context.emit(I::LocalGet(width));
                    match kind {
                        BitKind::Int => {
                            context.emit(I::I32Const(signed));
                            context.emit(I::I32Const(endian));
                            self.call("bits_read_int", context)?;
                        }
                        BitKind::Float => {
                            context.emit(I::I32Const(endian));
                            self.call("bits_read_float", context)?;
                        }
                        _ => self.call("bits_slice", context)?,
                    }
                    context.emit(I::LocalSet(extracted));
                    self.pattern(&segment.value, extracted, fail, context)?;
                }
            }
            context.emit(I::LocalGet(offset));
            context.emit(I::LocalGet(width));
            context.emit(I::I64Add);
            context.emit(I::LocalSet(offset));
        }
        context.emit(I::LocalGet(offset));
        context.emit(I::LocalGet(source));
        self.call("bits_len64", context)?;
        context.emit(I::I64Ne);
        context.br_if(fail);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn read_utf(
        &self,
        source: u32,
        offset: u32,
        width: u32,
        extracted: u32,
        encoding: i32,
        endian: i32,
        fail: Label,
        context: &mut Context,
    ) -> Result<(), Error> {
        let success = context.local(ValType::I32);
        context.emit(I::LocalGet(source));
        context.emit(I::LocalGet(offset));
        context.emit(I::I32Const(encoding));
        context.emit(I::I32Const(endian));
        self.call("bits_read_utf", context)?;
        context.emit(I::LocalSet(success));
        context.emit(I::LocalSet(width));
        context.emit(I::LocalSet(extracted));
        context.emit(I::LocalGet(success));
        reject_false(context, fail);
        Ok(())
    }

    fn check_bit_range(
        &self,
        source: u32,
        offset: u32,
        width: u32,
        fail: Label,
        context: &mut Context,
    ) -> Result<(), Error> {
        context.emit(I::LocalGet(source));
        context.emit(I::LocalGet(offset));
        context.emit(I::LocalGet(width));
        self.call("bits_has", context)?;
        reject_false(context, fail);
        Ok(())
    }
}

fn reject_false(context: &mut Context, fail: Label) {
    context.emit(I::I32Eqz);
    context.br_if(fail);
}

fn utf_kind(kind: BitKind) -> (i32, bool) {
    match kind {
        BitKind::Utf8 => (8, false),
        BitKind::Utf16 => (16, false),
        BitKind::Utf32 => (32, false),
        BitKind::Utf8Codepoint => (8, true),
        BitKind::Utf16Codepoint => (16, true),
        BitKind::Utf32Codepoint => (32, true),
        _ => unreachable!("only Unicode segment kinds have encodings"),
    }
}

/// Compile constant segments into bytes without creating runtime intermediate
/// arrays. A fixed budget prevents large literals from causing unbounded work.
fn constant_bits(segments: &[BitSegment<Expr>]) -> Option<(Vec<u8>, usize)> {
    const MAX_BITS: usize = 16 * 1024 * 1024;
    let mut bits = PackedBits::default();
    for segment in segments {
        let width = match &segment.options.size {
            Some(size) => match size.as_ref() {
                Expr::Int(integer) => {
                    let width = if integer.is_negative() {
                        0
                    } else {
                        integer.to_usize()?
                    };
                    width.checked_mul(usize::from(segment.options.unit))?
                }
                _ => return None,
            },
            None => match segment.options.kind {
                BitKind::Int => 8,
                BitKind::Float => 64,
                _ => 0,
            },
        };
        if width > MAX_BITS || bits.length.checked_add(width)? > MAX_BITS {
            return None;
        }
        match (&segment.value, segment.options.kind) {
            (Expr::Int(integer), BitKind::Int) => {
                // Calculate two's-complement bytes once, avoiding a temporary
                // BigInt shift and allocation for every individual literal bit.
                let bytes = integer.to_signed_bytes_le();
                let extension = if integer.is_negative() { 255 } else { 0 };
                let read = |at: usize| {
                    bytes.get(at / 8).copied().unwrap_or(extension) & (1 << (at % 8)) != 0
                };
                if segment.options.little_endian {
                    for start in (0..width).step_by(8) {
                        for bit in (start..(start + 8).min(width)).rev() {
                            bits.push(read(bit));
                        }
                    }
                } else {
                    for bit in (0..width).rev() {
                        bits.push(read(bit));
                    }
                }
            }
            (Expr::Float(number), BitKind::Float) => {
                let bytes = match width {
                    64 => {
                        if segment.options.little_endian {
                            number.to_le_bytes().to_vec()
                        } else {
                            number.to_be_bytes().to_vec()
                        }
                    }
                    32 => {
                        if segment.options.little_endian {
                            (*number as f32).to_le_bytes().to_vec()
                        } else {
                            (*number as f32).to_be_bytes().to_vec()
                        }
                    }
                    _ => return None,
                };
                bits.append_bytes(&bytes);
            }
            (Expr::String(value), kind @ (BitKind::Utf8 | BitKind::Utf16 | BitKind::Utf32)) => {
                let bytes: Vec<u8> = match kind {
                    BitKind::Utf8 => value.as_bytes().to_vec(),
                    BitKind::Utf16 => value
                        .encode_utf16()
                        .flat_map(|unit| {
                            if segment.options.little_endian {
                                unit.to_le_bytes()
                            } else {
                                unit.to_be_bytes()
                            }
                        })
                        .collect(),
                    BitKind::Utf32 => value
                        .chars()
                        .flat_map(|c| {
                            if segment.options.little_endian {
                                (c as u32).to_le_bytes()
                            } else {
                                (c as u32).to_be_bytes()
                            }
                        })
                        .collect(),
                    _ => unreachable!(),
                };
                if bits.length.checked_add(bytes.len().checked_mul(8)?)? > MAX_BITS {
                    return None;
                }
                bits.append_bytes(&bytes);
            }
            _ => return None,
        }
    }
    Some((bits.bytes, bits.length))
}

#[derive(Default)]
struct PackedBits {
    bytes: Vec<u8>,
    length: usize,
}

impl PackedBits {
    fn push(&mut self, bit: bool) {
        if self.length.is_multiple_of(8) {
            self.bytes.push(0);
        }
        if bit {
            self.bytes[self.length / 8] |= 1 << (7 - self.length % 8);
        }
        self.length += 1;
    }

    fn append_bytes(&mut self, bytes: &[u8]) {
        if self.length.is_multiple_of(8) {
            self.bytes.extend_from_slice(bytes);
            self.length += bytes.len() * 8;
        } else {
            for byte in bytes {
                for bit in (0..8).rev() {
                    self.push(byte & (1 << bit) != 0);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(value: i32, width: i32, little_endian: bool) -> BitSegment<Expr> {
        BitSegment {
            value: Expr::Int(value.into()),
            options: BitOptions {
                size: Some(Box::new(Expr::Int(width.into()))),
                little_endian,
                ..Default::default()
            },
        }
    }

    #[test]
    fn literal_segments_preserve_partial_bytes_and_signed_endianness() {
        assert_eq!(
            constant_bits(&[int(-1, 3, false), int(2, 5, false)]),
            Some((vec![0xe2], 8))
        );
        assert_eq!(
            constant_bits(&[int(0x1234, 12, true)]),
            Some((vec![0x34, 0x20], 12))
        );
        assert_eq!(
            constant_bits(&[int(255, -1, false), int(3, 2, false)]),
            Some((vec![0xc0], 2))
        );
    }

    #[test]
    fn literal_unicode_encodes_utf16_surrogate_pairs_and_utf32_little_endian() {
        let segment = |kind| BitSegment {
            value: Expr::String("🌍".into()),
            options: BitOptions {
                kind,
                little_endian: true,
                ..Default::default()
            },
        };
        assert_eq!(
            constant_bits(&[segment(BitKind::Utf16)]),
            Some((vec![0x3c, 0xd8, 0x0d, 0xdf], 32))
        );
        assert_eq!(
            constant_bits(&[segment(BitKind::Utf32)]),
            Some((vec![0x0d, 0xf3, 0x01, 0], 32))
        );
    }

    #[test]
    fn direct_patterns_execute_checked_language_cases() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("src")).unwrap();
        std::fs::write(
            directory.path().join("gleam.toml"),
            "name = \"patterns\"\nversion = \"1.0.0\"\ntarget = \"wasm\"\n",
        )
        .unwrap();
        std::fs::write(directory.path().join("src/patterns.gleam"), r#"
type Packet { Packet(value: Int, name: String) }
fn identity(value) { value }
fn sized(bits, size) {
  case bits { <<_:size(size), _:bits>> -> True _ -> False }
}
pub fn main() {
  let assert #(True, Nil, 1.5, 12345678901234567890, "hello") = identity(#(True, Nil, 1.5, 12345678901234567890, "hello"))
  let assert Packet(value: number, name: "packet") = Packet(42, "packet")
  assert number == 42
  let assert [first, second, ..tail] as whole = identity([1, 2, 3, 4])
  assert #(first, second, tail, whole) == #(1, 2, [3, 4], [1, 2, 3, 4])
  let assert "pre" as prefix <> suffix = identity("prefix")
  assert #(prefix, suffix) == #("pre", "fix")
  let choice = case identity(#(2, 11)) { #(1, n) | #(2, n) if n > 10 -> n _ -> 0 }
  assert choice == 11
  let assert <<_:utf8, tail:bytes>> = identity(<<"🌍!":utf8>>)
  assert tail == <<"!":utf8>>
  let assert <<"🌍":utf16-little, tail:bits>> = identity(<<"🌍":utf16-little, 5:3>>)
  assert tail == <<5:3>>
  let assert <<"🌍":utf32-little>> = identity(<<"🌍":utf32-little>>)
  let assert <<first:bytes-unit(2)-size(3), rest:bits>> = identity(<<63:6, 2:2>>)
  assert first == <<63:6>>
  assert rest == <<2:2>>
  let assert <<little:12-little, signed:4-signed>> = identity(<<0x1234:12-little, -2:4>>)
  assert #(little, signed) == #(0x234, -2)
  assert <<255:size(identity(-1)), 2:2>> == <<2:2>>
  assert sized(<<1>>, identity(-1)) == False
  assert sized(<<1>>, identity(9999999999999999999999999999999999)) == False
  let negative = case identity(<<255>>) { <<_:utf8>> -> False _ -> True }
  assert negative
  Nil
}
"#).unwrap();
        let checked = wleam_frontend::check_project(directory.path(), &Default::default()).unwrap();
        let program = crate::lower::lower(&checked.modules, &["patterns.main".into()]).unwrap();
        let artifact = super::super::compile(
            &program,
            &crate::Options {
                entry: "patterns.main".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let binary = directory.path().join("patterns.wasm");
        std::fs::write(&binary, artifact.wasm).unwrap();
        let node = std::env::var_os("NODE").unwrap_or_else(|| "node".into());
        let result = std::process::Command::new(node).arg("--input-type=module").arg("-e").arg(r#"
import fs from 'node:fs';
const module = new WebAssembly.Module(fs.readFileSync(process.argv[1]));
let instance;
instance = new WebAssembly.Instance(module, {'wleam:host/v1': {
 write() {}, random(){ return 0; }, reference_id(){ return 1n; },
 panic(ptr,len){ throw Error(new TextDecoder().decode(new Uint8Array(instance.exports.memory.buffer,ptr,len))); }
}});
instance.exports._start();
"#).arg(&binary).output().expect("Node.js from Devenv is required for WasmGC execution");
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
