//! Versioned, copying host boundary for Core Wasm modules.
//!
//! The garbage-collected representation is private. Only temporary linear-memory
//! buffers and opaque host references cross the public boundary.

use ilang_core::type_::{self, Type};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Abi {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    #[serde(default)]
    pub tests: Vec<String>,
    #[serde(default)]
    pub exports: BTreeMap<String, Signature>,
    #[serde(default)]
    pub imports: BTreeMap<String, BTreeMap<String, Signature>>,
}

impl Default for Abi {
    fn default() -> Self {
        Self {
            version: 1,
            entry: None,
            tests: Vec::new(),
            exports: BTreeMap::new(),
            imports: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbiType {
    Nil,
    Bool,
    Float,
    Int,
    String,
    BitArray,
    #[serde(rename = "externref")]
    ExternRef,
}

impl AbiType {
    pub fn flat(self) -> &'static [&'static str] {
        match self {
            Self::Nil => &[],
            Self::Bool => &["i32"],
            Self::Float => &["f64"],
            Self::Int => &["i32", "i32", "i32"],
            Self::String | Self::BitArray => &["i32", "i32"],
            Self::ExternRef => &["externref"],
        }
    }

    fn lift(self) -> &'static str {
        match self {
            Self::Nil => "ref.null eq",
            Self::Bool => "call $abi_bool_lift",
            Self::Float => "call $abi_float_lift",
            Self::Int => "call $abi_int_lift",
            Self::String => "call $abi_string_lift",
            Self::BitArray => "call $abi_bits_lift",
            Self::ExternRef => "call $abi_host_lift",
        }
    }

    fn lower(self) -> &'static str {
        match self {
            Self::Nil => "drop",
            Self::Bool => "call $bool_get",
            Self::Float => "call $float_get",
            Self::Int => "call $abi_int_lower",
            Self::String => "call $abi_string_lower",
            Self::BitArray => "call $abi_bits_lower",
            Self::ExternRef => "call $abi_host_lower",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    pub params: Vec<AbiType>,
    pub result: AbiType,
}

impl Signature {
    pub fn from_type(ty: &Arc<Type>, external_types: &BTreeSet<String>) -> Result<Self, String> {
        let ty = type_::collapse_links(ty.clone());
        let Type::Fn { arguments, return_ } = ty.as_ref() else {
            return Err("a host export must name a function".into());
        };
        Ok(Self {
            params: arguments
                .iter()
                .map(|arg| type_of_with_externals(arg, external_types))
                .collect::<Result<_, _>>()?,
            result: type_of_with_externals(return_, external_types)?,
        })
    }
}

pub fn type_of(ty: &Arc<Type>) -> Result<AbiType, String> {
    type_of_with_externals(ty, &BTreeSet::new())
}

pub fn type_of_with_externals(
    ty: &Arc<Type>,
    external_types: &BTreeSet<String>,
) -> Result<AbiType, String> {
    let ty = type_::collapse_links(ty.clone());
    match ty.as_ref() {
        Type::Named {
            module,
            name,
            arguments,
            ..
        } => {
            if external_types.contains(&format!("{module}.{name}")) {
                return if arguments.iter().all(is_concrete) {
                    Ok(AbiType::ExternRef)
                } else {
                    Err("generic host functions are not supported; use a concrete signature".into())
                };
            }
            if module == "i" && arguments.is_empty() {
                return match name.as_str() {
                    "Nil" => Ok(AbiType::Nil),
                    "Bool" => Ok(AbiType::Bool),
                    "Float" => Ok(AbiType::Float),
                    "Int" => Ok(AbiType::Int),
                    "String" => Ok(AbiType::String),
                    "BitArray" => Ok(AbiType::BitArray),
                    _ => Err(format!(
                        "{name} cannot cross the host ABI; encode it explicitly"
                    )),
                };
            }
            Err(format!(
                "{module}.{name} is not a declared external type; encode it explicitly"
            ))
        }
        Type::Var { .. } => {
            Err("generic host functions are not supported; use a concrete signature".into())
        }
        Type::Fn { .. } => Err("function callbacks cannot cross the synchronous host ABI".into()),
        Type::Tuple { .. } => Err(
            "collections and custom data cannot cross the host ABI; encode them explicitly".into(),
        ),
    }
}

fn is_concrete(ty: &Arc<Type>) -> bool {
    match type_::collapse_links(ty.clone()).as_ref() {
        Type::Named { arguments, .. } => arguments.iter().all(is_concrete),
        Type::Fn { arguments, return_ } => {
            arguments.iter().all(is_concrete) && is_concrete(return_)
        }
        Type::Tuple { elements } => elements.iter().all(is_concrete),
        Type::Var { .. } => false,
    }
}

/// Escape an arbitrary UTF-8 string as a WAT string literal.
pub fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for byte in text.bytes() {
        match byte {
            b' '..=b'~' if byte != b'"' && byte != b'\\' => out.push(char::from(byte)),
            _ => out.push_str(&format!("\\{byte:02x}")),
        }
    }
    out.push('"');
    out
}

fn ident(name: &str) -> String {
    if name.starts_with('$') {
        name.to_owned()
    } else {
        format!("${name}")
    }
}

fn declarations(kind: &str, prefix: &str, ty: AbiType) -> String {
    ty.flat()
        .iter()
        .enumerate()
        .map(|(i, ty)| format!("({kind} ${prefix}_{i} {ty})\n"))
        .collect()
}

fn get_flat(prefix: &str, ty: AbiType) -> String {
    (0..ty.flat().len())
        .map(|i| format!("local.get ${prefix}_{i}\n"))
        .collect()
}

fn set_flat(prefix: &str, ty: AbiType) -> String {
    (0..ty.flat().len())
        .rev()
        .map(|i| format!("local.set ${prefix}_{i}\n"))
        .collect()
}

fn free_flat(prefix: &str, ty: AbiType) -> String {
    match ty {
        AbiType::Int => format!(
            "local.get ${prefix}_1\nlocal.get ${prefix}_2\ni32.const 4\ni32.mul\ncall $free\n"
        ),
        AbiType::String => format!("local.get ${prefix}_0\nlocal.get ${prefix}_1\ncall $free\n"),
        AbiType::BitArray => format!(
            "local.get ${prefix}_0\nlocal.get ${prefix}_1\ni64.extend_i32_u\ni64.const 7\ni64.add\ni64.const 3\ni64.shr_u\ni32.wrap_i64\ncall $free\n"
        ),
        _ => String::new(),
    }
}

/// Produce a raw import and an internal `(env, args) -> value` wrapper.
pub fn import_wrapper(
    internal_wat_name: &str,
    module: &str,
    name: &str,
    signature: &Signature,
    raw_id: &str,
) -> (String, String) {
    let raw = ident(raw_id);
    let mut import = format!("(import {} {} (func {raw}", quote(module), quote(name));
    for ty in &signature.params {
        for flat in ty.flat() {
            import.push_str(&format!(" (param {flat})"));
        }
    }
    for flat in signature.result.flat() {
        import.push_str(&format!(" (result {flat})"));
    }
    import.push_str("))\n");

    let mut body = format!(
        "(func {} (type $call) (param $env eqref) (param $args eqref) (result eqref)\n(local $boxed_result eqref)\n",
        ident(internal_wat_name)
    );
    for (i, ty) in signature.params.iter().enumerate() {
        body.push_str(&declarations("local", &format!("a{i}"), *ty));
    }
    body.push_str(&declarations("local", "r", signature.result));
    for (i, ty) in signature.params.iter().enumerate() {
        body.push_str(&format!(
            "local.get $args\nref.cast (ref $values)\ni32.const {i}\narray.get $values\n{}\n",
            ty.lower()
        ));
        body.push_str(&set_flat(&format!("a{i}"), *ty));
    }
    for (i, ty) in signature.params.iter().enumerate() {
        body.push_str(&get_flat(&format!("a{i}"), *ty));
    }
    body.push_str(&format!("call {raw}\n"));
    body.push_str(&set_flat("r", signature.result));
    for (i, ty) in signature.params.iter().enumerate() {
        body.push_str(&free_flat(&format!("a{i}"), *ty));
    }
    body.push_str(&get_flat("r", signature.result));
    body.push_str(signature.result.lift());
    body.push_str("\nlocal.set $boxed_result\n");
    body.push_str(&free_flat("r", signature.result));
    body.push_str("local.get $boxed_result\n)\n");
    (import, body)
}

/// Produce a public, flattened export calling an internal i function.
pub fn export_wrapper(export: &str, target_wat: &str, signature: &Signature) -> String {
    let mut out = format!("(func (export {})\n", quote(export));
    for (i, ty) in signature.params.iter().enumerate() {
        out.push_str(&declarations("param", &format!("a{i}"), *ty));
    }
    for ty in signature.result.flat() {
        out.push_str(&format!("(result {ty})\n"));
    }
    out.push_str("ref.null eq\n");
    for (i, ty) in signature.params.iter().enumerate() {
        out.push_str(&get_flat(&format!("a{i}"), *ty));
        out.push_str(ty.lift());
        out.push('\n');
    }
    out.push_str(&format!(
        "array.new_fixed $values {}\ncall {}\n{}\n)\n",
        signature.params.len(),
        ident(target_wat),
        signature.result.lower()
    ));
    out
}

pub fn type_wat() -> &'static str {
    "(type $hostbox (struct (field externref) (field i64)))"
}

/// Bridge helpers keep GC references out of user-controlled linear memory.
pub fn runtime_wat() -> &'static str {
    r#"
    ;; A host call owns a temporary arena. Its freelist is isolated from the
    ;; surrounding call so restoring a frame also reclaims allocations made on
    ;; paths that trapped before ordinary buffer cleanup. Nested host calls keep
    ;; the outer call's buffers intact.
    (global $abi_frame_top (mut i32) (i32.const 0))
    (func (export "__ilang_abi_mark") (result i32)
      (local $mark i32) (local $end i64) (local $pages i64)
      global.get $heap local.set $mark
      local.get $mark i64.extend_i32_u i64.const 16 i64.add local.set $end
      local.get $end i64.const 4294967295 i64.gt_u if unreachable end
      local.get $end i64.const 65535 i64.add i64.const 16 i64.shr_u local.set $pages
      local.get $pages memory.size i64.extend_i32_u i64.gt_u
      if
        local.get $pages memory.size i64.extend_i32_u i64.sub i32.wrap_i64 memory.grow
        i32.const -1 i32.eq if unreachable end
      end
      local.get $mark global.get $free_head i32.store
      local.get $mark global.get $abi_frame_top i32.store offset=4
      local.get $mark global.set $abi_frame_top
      local.get $end i32.wrap_i64 global.set $heap
      i32.const 0 global.set $free_head
      local.get $mark)
    (func (export "__ilang_abi_reset") (param $mark i32)
      local.get $mark i32.eqz if unreachable end
      local.get $mark global.get $abi_frame_top i32.ne if unreachable end
      local.get $mark i32.load global.set $free_head
      local.get $mark i32.load offset=4 global.set $abi_frame_top
      local.get $mark global.set $heap)

    (func $abi_check_range (param $ptr i32) (param $len i32)
      local.get $ptr i64.extend_i32_u
      local.get $len i64.extend_i32_u i64.add
      memory.size i64.extend_i32_u i64.const 16 i64.shl
      i64.gt_u if unreachable end)

    (func $abi_bool_lift (param $v i32) (result eqref)
      local.get $v i32.const 1 i32.gt_u if unreachable end
      local.get $v call $bool_new)
    (func $abi_float_lift (param $v f64) (result eqref)
      local.get $v local.get $v f64.ne
      local.get $v f64.abs f64.const inf f64.eq i32.or
      if unreachable end
      local.get $v call $float_new)

    (func $abi_host_lift (param $v externref) (result eqref)
      local.get $v local.get $v call $host_reference_id struct.new $hostbox)
    (func $abi_host_lower (param $v eqref) (result externref)
      local.get $v ref.cast (ref $hostbox) struct.get $hostbox 0)
    (func $host_id (param $v eqref) (result i64)
      local.get $v ref.cast (ref $hostbox) struct.get $hostbox 1)
    (func $host_equal (param $a eqref) (param $b eqref) (result i32)
      local.get $a call $host_id local.get $b call $host_id i64.eq)

    (func $abi_int_lift (param $sign i32) (param $ptr i32) (param $count i32) (result eqref)
      (local $limbs (ref $limbs)) (local $i i32)
      local.get $count i32.const 1073741823 i32.gt_u if unreachable end
      local.get $ptr local.get $count i32.const 4 i32.mul call $abi_check_range
      local.get $count i32.eqz
      if
        local.get $sign if unreachable end
      else
        local.get $sign i32.const 1 i32.ne
        local.get $sign i32.const -1 i32.ne i32.and if unreachable end
        local.get $ptr local.get $count i32.const 1 i32.sub i32.const 4 i32.mul i32.add
        i32.load i32.eqz if unreachable end
      end
      local.get $count array.new_default $limbs local.set $limbs
      block $done loop $copy
        local.get $i local.get $count i32.ge_u br_if $done
        local.get $limbs local.get $i
        local.get $ptr local.get $i i32.const 4 i32.mul i32.add i32.load
        array.set $limbs
        local.get $i i32.const 1 i32.add local.set $i br $copy
      end end
      local.get $sign local.get $limbs call $int_pack)

    (func $abi_int_lower (param $v eqref) (result i32 i32 i32)
      (local $limbs (ref $limbs)) (local $count i32) (local $ptr i32) (local $i i32)
      local.get $v call $int_limbs local.tee $limbs array.len local.set $count
      local.get $count i32.const 1073741823 i32.gt_u if unreachable end
      local.get $count i32.const 4 i32.mul call $alloc local.set $ptr
      block $done loop $copy
        local.get $i local.get $count i32.ge_u br_if $done
        local.get $ptr local.get $i i32.const 4 i32.mul i32.add
        local.get $limbs local.get $i array.get $limbs i32.store
        local.get $i i32.const 1 i32.add local.set $i br $copy
      end end
      local.get $v call $int_sign local.get $ptr local.get $count)

    (func $abi_utf8_valid (param $ptr i32) (param $len i32) (result i32)
      (local $i i32) (local $b i32) (local $b1 i32) (local $n i32) (local $j i32)
      local.get $ptr local.get $len call $abi_check_range
      block $invalid
        loop $next
          local.get $i local.get $len i32.ge_u if i32.const 1 return end
          local.get $ptr local.get $i i32.add i32.load8_u local.set $b
          local.get $b i32.const 128 i32.lt_u
          if
            local.get $i i32.const 1 i32.add local.set $i br $next
          end
          local.get $b i32.const 194 i32.lt_u br_if $invalid
          local.get $b i32.const 223 i32.le_u
          if i32.const 2 local.set $n
          else
            local.get $b i32.const 239 i32.le_u
            if i32.const 3 local.set $n
            else
              local.get $b i32.const 244 i32.gt_u br_if $invalid
              i32.const 4 local.set $n
            end
          end
          local.get $len local.get $i i32.sub local.get $n i32.lt_u br_if $invalid
          local.get $ptr local.get $i i32.add i32.load8_u offset=1 local.set $b1
          local.get $b i32.const 224 i32.eq local.get $b1 i32.const 160 i32.lt_u i32.and br_if $invalid
          local.get $b i32.const 237 i32.eq local.get $b1 i32.const 160 i32.ge_u i32.and br_if $invalid
          local.get $b i32.const 240 i32.eq local.get $b1 i32.const 144 i32.lt_u i32.and br_if $invalid
          local.get $b i32.const 244 i32.eq local.get $b1 i32.const 144 i32.ge_u i32.and br_if $invalid
          i32.const 1 local.set $j
          block $continued loop $continuation
            local.get $j local.get $n i32.ge_u br_if $continued
            local.get $ptr local.get $i i32.add local.get $j i32.add i32.load8_u
            i32.const 192 i32.and i32.const 128 i32.ne br_if $invalid
            local.get $j i32.const 1 i32.add local.set $j br $continuation
          end end
          local.get $i local.get $n i32.add local.set $i br $next
        end
      end
      i32.const 0)

    (func $abi_string_lift (param $ptr i32) (param $len i32) (result eqref)
      local.get $ptr local.get $len call $abi_utf8_valid i32.eqz if unreachable end
      local.get $ptr local.get $len call $string_from_mem)

    (func $abi_string_lower (param $v eqref) (result i32 i32)
      (local $bytes (ref $bytes)) (local $ptr i32) (local $len i32) (local $i i32)
      local.get $v ref.cast (ref $obj) struct.get $obj 4 ref.as_non_null local.tee $bytes
      array.len local.set $len
      local.get $len call $alloc local.set $ptr
      block $done loop $copy
        local.get $i local.get $len i32.ge_u br_if $done
        local.get $ptr local.get $i i32.add
        local.get $bytes local.get $i array.get_u $bytes i32.store8
        local.get $i i32.const 1 i32.add local.set $i br $copy
      end end
      local.get $ptr local.get $len)

    (func $abi_bits_lift (param $ptr i32) (param $bits i32) (result eqref)
      (local $bytes (ref $bytes)) (local $len i32) (local $i i32)
      local.get $bits i64.extend_i32_u i64.const 7 i64.add i64.const 3 i64.shr_u i32.wrap_i64 local.set $len
      local.get $ptr local.get $len call $abi_check_range
      local.get $len array.new_default $bytes local.set $bytes
      block $done loop $copy
        local.get $i local.get $len i32.ge_u br_if $done
        local.get $bytes local.get $i
        local.get $ptr local.get $i i32.add i32.load8_u array.set $bytes
        local.get $i i32.const 1 i32.add local.set $i br $copy
      end end
      local.get $bits i32.const 7 i32.and
      if
        local.get $bytes local.get $len i32.const 1 i32.sub
        local.get $bytes local.get $len i32.const 1 i32.sub array.get_u $bytes
        i32.const 255 i32.const 8 local.get $bits i32.const 7 i32.and i32.sub i32.shl i32.and
        array.set $bytes
      end
      i32.const 9 i32.const 0 local.get $bits i64.extend_i32_u f64.const 0
      local.get $bytes ref.null $values ref.null $limbs ref.null $call struct.new $obj)

    (func $abi_bits_lower (param $v eqref) (result i32 i32)
      (local $obj (ref $obj)) (local $bytes (ref $bytes))
      (local $bits i32) (local $len i32) (local $offset i64) (local $position i64)
      (local $ptr i32) (local $i i32) (local $value i32)
      local.get $v ref.cast (ref $obj) local.set $obj
      local.get $obj struct.get $obj 2 i64.const 4294967295 i64.gt_u if unreachable end
      local.get $obj struct.get $obj 2 i32.wrap_i64 local.set $bits
      local.get $obj struct.get $obj 1 i64.extend_i32_u local.set $offset
      local.get $obj struct.get $obj 4 ref.as_non_null local.set $bytes
      local.get $bits i64.extend_i32_u i64.const 7 i64.add i64.const 3 i64.shr_u i32.wrap_i64 local.set $len
      local.get $len call $alloc local.tee $ptr i32.const 0 local.get $len memory.fill
      block $done loop $copy
        local.get $i local.get $bits i32.ge_u br_if $done
        local.get $offset local.get $i i64.extend_i32_u i64.add local.set $position
        local.get $bytes local.get $position i64.const 3 i64.shr_u i32.wrap_i64 array.get_u $bytes
        i32.const 7 local.get $position i32.wrap_i64 i32.const 7 i32.and i32.sub i32.shr_u
        i32.const 1 i32.and
        if
          local.get $ptr local.get $i i32.const 3 i32.shr_u i32.add
          local.get $ptr local.get $i i32.const 3 i32.shr_u i32.add i32.load8_u
          i32.const 1 i32.const 7 local.get $i i32.const 7 i32.and i32.sub i32.shl i32.or
          i32.store8
        end
        local.get $i i32.const 1 i32.add local.set $i br $copy
      end end
      local.get $ptr local.get $bits)
    "#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_uses_the_version_one_wire_names() {
        let mut abi = Abi::default();
        abi.exports.insert(
            "identity".into(),
            Signature {
                params: vec![AbiType::ExternRef, AbiType::BitArray],
                result: AbiType::Int,
            },
        );
        let encoded = serde_json::to_value(&abi).expect("serialize ABI");
        assert_eq!(encoded["version"], 1);
        assert_eq!(
            encoded["exports"]["identity"]["params"],
            serde_json::json!(["externref", "bit_array"])
        );
        assert_eq!(
            serde_json::from_value::<Abi>(encoded).expect("decode ABI"),
            abi
        );
    }

    #[test]
    fn exported_names_cannot_escape_wat_strings() {
        assert_eq!(quote("a\"\\\n🙂"), "\"a\\22\\5c\\0a\\f0\\9f\\99\\82\"");
    }

    #[test]
    fn mixed_multivalue_import_and_export_wrappers_validate() {
        // Exercise every result arity against a signature containing all input
        // arities, including zero-width Nil. This catches stack ordering and
        // ownership-cleanup instructions accidentally consuming a result.
        let types = [
            AbiType::Nil,
            AbiType::Bool,
            AbiType::Float,
            AbiType::Int,
            AbiType::String,
            AbiType::BitArray,
            AbiType::ExternRef,
        ];
        let mut imports = String::new();
        let mut functions = String::new();
        for (i, result) in types.iter().copied().enumerate() {
            let signature = Signature {
                params: types.to_vec(),
                result,
            };
            let (import, function) = import_wrapper(
                &format!("$fn{i}"),
                "example",
                &format!("f{i}"),
                &signature,
                &format!("$raw{i}"),
            );
            imports.push_str(&import);
            functions.push_str(&function);
            functions.push_str(&export_wrapper(
                &format!("f{i}"),
                &format!("$fn{i}"),
                &signature,
            ));
        }
        let stubs = r#"
            (type $values (array (mut eqref)))
            (type $call (func (param eqref eqref) (result eqref)))
        "#;
        let helpers = r#"
            (func $free (param i32 i32))
            (func $abi_bool_lift (param i32) (result eqref) ref.null eq)
            (func $bool_get (param eqref) (result i32) i32.const 0)
            (func $abi_float_lift (param f64) (result eqref) ref.null eq)
            (func $float_get (param eqref) (result f64) f64.const 0)
            (func $abi_int_lift (param i32 i32 i32) (result eqref) ref.null eq)
            (func $abi_int_lower (param eqref) (result i32 i32 i32) i32.const 0 i32.const 0 i32.const 0)
            (func $abi_string_lift (param i32 i32) (result eqref) ref.null eq)
            (func $abi_string_lower (param eqref) (result i32 i32) i32.const 0 i32.const 0)
            (func $abi_bits_lift (param i32 i32) (result eqref) ref.null eq)
            (func $abi_bits_lower (param eqref) (result i32 i32) i32.const 0 i32.const 0)
            (func $abi_host_lift (param externref) (result eqref) ref.null eq)
            (func $abi_host_lower (param eqref) (result externref) ref.null extern)
        "#;
        let bytes = wat::parse_str(format!("(module {stubs} {imports} {helpers} {functions})"))
            .expect("ABI wrappers encode");
        wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::all())
            .validate_all(&bytes)
            .expect("all flattened wrapper signatures validate");
    }
}
