//! Direct instruction emission for the public copying ABI and runtime metadata.

use std::collections::{BTreeMap, BTreeSet};

use wasm_encoder::{BlockType, Function, HeapType, Instruction as I, RefType, ValType};

use super::Types;
use crate::{
    Error,
    abi::{AbiType, Signature},
    ir::Program,
};

pub(super) fn flattened_types(ty: AbiType) -> Vec<ValType> {
    match ty {
        AbiType::Nil => vec![],
        AbiType::Bool => vec![ValType::I32],
        AbiType::Float => vec![ValType::F64],
        AbiType::Int => vec![ValType::I32; 3],
        AbiType::String | AbiType::BitArray => vec![ValType::I32; 2],
        AbiType::ExternRef => vec![ValType::EXTERNREF],
    }
}

fn lift(
    function: &mut Function,
    ty: AbiType,
    lookup: &impl Fn(&str) -> Result<u32, Error>,
) -> Result<(), Error> {
    let name = match ty {
        AbiType::Nil => {
            function.instruction(&I::RefNull(RefType::EQREF.heap_type));
            return Ok(());
        }
        AbiType::Bool => "abi_bool_lift",
        AbiType::Float => "abi_float_lift",
        AbiType::Int => "abi_int_lift",
        AbiType::String => "abi_string_lift",
        AbiType::BitArray => "abi_bits_lift",
        AbiType::ExternRef => "abi_host_lift",
    };
    function.instruction(&I::Call(lookup(name)?));
    Ok(())
}

fn lower(
    function: &mut Function,
    ty: AbiType,
    lookup: &impl Fn(&str) -> Result<u32, Error>,
) -> Result<(), Error> {
    let name = match ty {
        AbiType::Nil => {
            function.instruction(&I::Drop);
            return Ok(());
        }
        AbiType::Bool => "bool_get",
        AbiType::Float => "float_get",
        AbiType::Int => "abi_int_lower",
        AbiType::String => "abi_string_lower",
        AbiType::BitArray => "abi_bits_lower",
        AbiType::ExternRef => "abi_host_lower",
    };
    function.instruction(&I::Call(lookup(name)?));
    Ok(())
}

fn get_flat(function: &mut Function, start: u32, ty: AbiType) {
    for offset in 0..flattened_types(ty).len() as u32 {
        function.instruction(&I::LocalGet(start + offset));
    }
}

fn set_flat(function: &mut Function, start: u32, ty: AbiType) {
    for offset in (0..flattened_types(ty).len() as u32).rev() {
        function.instruction(&I::LocalSet(start + offset));
    }
}

fn free_flat(
    function: &mut Function,
    start: u32,
    ty: AbiType,
    lookup: &impl Fn(&str) -> Result<u32, Error>,
) -> Result<(), Error> {
    match ty {
        AbiType::Int => {
            function.instruction(&I::LocalGet(start + 1));
            function.instruction(&I::LocalGet(start + 2));
            function.instruction(&I::I32Const(4));
            function.instruction(&I::I32Mul);
        }
        AbiType::String => {
            function.instruction(&I::LocalGet(start));
            function.instruction(&I::LocalGet(start + 1));
        }
        AbiType::BitArray => {
            function.instruction(&I::LocalGet(start));
            function.instruction(&I::LocalGet(start + 1));
            function.instruction(&I::I64ExtendI32U);
            function.instruction(&I::I64Const(7));
            function.instruction(&I::I64Add);
            function.instruction(&I::I64Const(3));
            function.instruction(&I::I64ShrU);
            function.instruction(&I::I32WrapI64);
        }
        _ => return Ok(()),
    }
    function.instruction(&I::Call(lookup("free")?));
    Ok(())
}

/// An internal `(environment, arguments) -> value` function that calls a raw
/// host import. Each temporary argument and returned buffer has one owner.
pub(super) fn import_wrapper(
    signature: &Signature,
    raw_index: u32,
    types: &Types,
    lookup: &impl Fn(&str) -> Result<u32, Error>,
) -> Result<Function, Error> {
    // Parameters occupy 0 and 1; local 2 keeps the lifted result alive while
    // its transfer buffer is returned to the allocator.
    let boxed_result = 2;
    let mut locals = vec![ValType::Ref(RefType::EQREF)];
    let mut arguments = Vec::with_capacity(signature.params.len());
    for ty in &signature.params {
        arguments.push(2 + locals.len() as u32);
        locals.extend(flattened_types(*ty));
    }
    let result_start = 2 + locals.len() as u32;
    locals.extend(flattened_types(signature.result));
    let mut function = Function::new_with_locals_types(locals);
    for (index, (&ty, &start)) in signature.params.iter().zip(&arguments).enumerate() {
        function.instruction(&I::LocalGet(1));
        function.instruction(&I::RefCastNonNull(HeapType::Concrete(types.values)));
        function.instruction(&I::I32Const(index as i32));
        function.instruction(&I::ArrayGet(types.values));
        lower(&mut function, ty, lookup)?;
        set_flat(&mut function, start, ty);
    }
    for (&ty, &start) in signature.params.iter().zip(&arguments) {
        get_flat(&mut function, start, ty);
    }
    function.instruction(&I::Call(raw_index));
    set_flat(&mut function, result_start, signature.result);
    for (&ty, &start) in signature.params.iter().zip(&arguments) {
        free_flat(&mut function, start, ty, lookup)?;
    }
    get_flat(&mut function, result_start, signature.result);
    lift(&mut function, signature.result, lookup)?;
    function.instruction(&I::LocalSet(boxed_result));
    free_flat(&mut function, result_start, signature.result, lookup)?;
    function.instruction(&I::LocalGet(boxed_result));
    function.instruction(&I::End);
    Ok(function)
}

/// A public flattened function that calls the private shared representation.
pub(super) fn export_wrapper(
    signature: &Signature,
    target_index: u32,
    types: &Types,
    lookup: &impl Fn(&str) -> Result<u32, Error>,
) -> Result<Function, Error> {
    let mut function = Function::new([]);
    function.instruction(&I::RefNull(RefType::EQREF.heap_type));
    let mut parameter = 0;
    for ty in &signature.params {
        get_flat(&mut function, parameter, *ty);
        lift(&mut function, *ty, lookup)?;
        parameter += flattened_types(*ty).len() as u32;
    }
    function.instruction(&I::ArrayNewFixed {
        array_type_index: types.values,
        array_size: signature.params.len() as u32,
    });
    function.instruction(&I::Call(target_index));
    lower(&mut function, signature.result, lookup)?;
    function.instruction(&I::End);
    Ok(function)
}

fn string(
    function: &mut Function,
    text: &str,
    types: &Types,
    lookup: &impl Fn(&str) -> Result<u32, Error>,
) -> Result<(), Error> {
    for byte in text.bytes() {
        function.instruction(&I::I32Const(i32::from(byte)));
    }
    function.instruction(&I::ArrayNewFixed {
        array_type_index: types.bytes,
        array_size: text.len() as u32,
    });
    function.instruction(&I::Call(lookup("string_from_bytes")?));
    Ok(())
}

/// Replace the fixed runtime's metadata stubs with program-specific lookups.
pub(super) fn generate_metadata(
    program: &Program,
    types: &Types,
    lookup: &impl Fn(&str) -> Result<u32, Error>,
) -> Result<BTreeMap<String, Function>, Error> {
    let mut constructor = Function::new([]);
    let mut type_name = Function::new([]);
    let mut is_dict = Function::new([]);
    let mut seen = BTreeSet::new();
    for definition in &program.constructors {
        constructor.instruction(&I::LocalGet(0));
        constructor.instruction(&I::I32Const(definition.tag as i32));
        constructor.instruction(&I::I32Eq);
        constructor.instruction(&I::If(BlockType::Empty));
        string(&mut constructor, &definition.name, types, lookup)?;
        constructor.instruction(&I::Return);
        constructor.instruction(&I::End);
        if seen.insert(definition.type_id) {
            type_name.instruction(&I::LocalGet(0));
            type_name.instruction(&I::I64Const(i64::from(definition.type_id)));
            type_name.instruction(&I::I64Eq);
            type_name.instruction(&I::If(BlockType::Empty));
            string(&mut type_name, &definition.type_name, types, lookup)?;
            type_name.instruction(&I::Return);
            type_name.instruction(&I::End);
        }
        if definition.type_name == "gleam/dict.Dict" {
            is_dict.instruction(&I::LocalGet(0));
            is_dict.instruction(&I::I64Const(i64::from(definition.type_id)));
            is_dict.instruction(&I::I64Eq);
            is_dict.instruction(&I::If(BlockType::Empty));
            is_dict.instruction(&I::I32Const(1));
            is_dict.instruction(&I::Return);
            is_dict.instruction(&I::End);
        }
    }
    string(&mut constructor, "Unknown", types, lookup)?;
    constructor.instruction(&I::End);
    string(&mut type_name, "Unknown", types, lookup)?;
    type_name.instruction(&I::End);
    is_dict.instruction(&I::I32Const(0));
    is_dict.instruction(&I::End);

    let mut classify = Function::new([(1, ValType::I32)]);
    classify.instruction(&I::LocalGet(0));
    classify.instruction(&I::Call(lookup("value_tag")?));
    classify.instruction(&I::LocalSet(1));
    for (tag, name) in [
        (0, "Nil"),
        (1, "Bool"),
        (2, "Float"),
        (3, "String"),
        (6, "List"),
        (7, "Function"),
        (8, "Int"),
        (9, "BitArray"),
        (11, "Array"),
    ] {
        classify.instruction(&I::LocalGet(1));
        classify.instruction(&I::I32Const(tag));
        classify.instruction(&I::I32Eq);
        classify.instruction(&I::If(BlockType::Empty));
        string(&mut classify, name, types, lookup)?;
        classify.instruction(&I::Return);
        classify.instruction(&I::End);
    }
    classify.instruction(&I::LocalGet(1));
    classify.instruction(&I::I32Const(5));
    classify.instruction(&I::I32Eq);
    classify.instruction(&I::If(BlockType::Empty));
    classify.instruction(&I::LocalGet(0));
    classify.instruction(&I::RefCastNonNull(HeapType::Concrete(types.obj)));
    classify.instruction(&I::StructGet {
        struct_type_index: types.obj,
        field_index: 2,
    });
    classify.instruction(&I::Call(lookup("type_is_dict")?));
    classify.instruction(&I::If(BlockType::Empty));
    string(&mut classify, "Dict", types, lookup)?;
    classify.instruction(&I::Return);
    classify.instruction(&I::End);
    classify.instruction(&I::End);
    string(&mut classify, "Unknown", types, lookup)?;
    classify.instruction(&I::End);
    Ok(BTreeMap::from([
        ("constructor_name".into(), constructor),
        ("type_name".into(), type_name),
        ("type_is_dict".into(), is_dict),
        ("dynamic_classify".into(), classify),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_encoder::{
        CodeSection, EntityType, FunctionSection, ImportSection, Module, StorageType, TypeSection,
    };

    #[test]
    fn flattened_abi_preserves_nil_and_multivalue_widths() {
        assert!(flattened_types(AbiType::Nil).is_empty());
        assert_eq!(flattened_types(AbiType::Int), [ValType::I32; 3]);
        assert_eq!(flattened_types(AbiType::BitArray), [ValType::I32; 2]);
        assert_eq!(flattened_types(AbiType::ExternRef), [ValType::EXTERNREF]);
    }

    #[test]
    fn mixed_import_and_export_wrappers_validate_for_every_return_type() {
        let kinds = [
            AbiType::Nil,
            AbiType::Bool,
            AbiType::Float,
            AbiType::Int,
            AbiType::String,
            AbiType::BitArray,
            AbiType::ExternRef,
        ];
        for result in kinds {
            let eqref = ValType::Ref(RefType::EQREF);
            let mut type_section = TypeSection::new();
            type_section.ty().array(&StorageType::Val(eqref), true);
            type_section.ty().function([eqref, eqref], [eqref]);
            let types = Types {
                bytes: 0,
                values: 0,
                limbs: 0,
                call: 1,
                obj: 0,
            };
            let mut imports = ImportSection::new();
            let mut helpers = BTreeMap::new();
            for (name, parameters, results) in [
                ("abi_bool_lift", vec![ValType::I32], vec![eqref]),
                ("abi_float_lift", vec![ValType::F64], vec![eqref]),
                ("abi_int_lift", vec![ValType::I32; 3], vec![eqref]),
                ("abi_string_lift", vec![ValType::I32; 2], vec![eqref]),
                ("abi_bits_lift", vec![ValType::I32; 2], vec![eqref]),
                ("abi_host_lift", vec![ValType::EXTERNREF], vec![eqref]),
                ("bool_get", vec![eqref], vec![ValType::I32]),
                ("float_get", vec![eqref], vec![ValType::F64]),
                ("abi_int_lower", vec![eqref], vec![ValType::I32; 3]),
                ("abi_string_lower", vec![eqref], vec![ValType::I32; 2]),
                ("abi_bits_lower", vec![eqref], vec![ValType::I32; 2]),
                ("abi_host_lower", vec![eqref], vec![ValType::EXTERNREF]),
                ("free", vec![ValType::I32; 2], vec![]),
            ] {
                let type_index = type_section.len();
                type_section.ty().function(parameters, results);
                helpers.insert(name, imports.len());
                imports.import("test", name, EntityType::Function(type_index));
            }
            let signature = Signature {
                params: kinds.to_vec(),
                result,
            };
            let raw_type = type_section.len();
            let parameters = signature
                .params
                .iter()
                .flat_map(|ty| flattened_types(*ty))
                .collect::<Vec<_>>();
            type_section
                .ty()
                .function(parameters, flattened_types(result));
            let raw_index = imports.len();
            imports.import("test", "foreign", EntityType::Function(raw_type));
            let lookup = |name: &str| {
                helpers
                    .get(name)
                    .copied()
                    .ok_or_else(|| Error::Abi(format!("unknown test helper {name}")))
            };
            let internal_index = imports.len();
            let mut functions = FunctionSection::new();
            functions.function(1);
            functions.function(raw_type);
            let mut code = CodeSection::new();
            code.function(&import_wrapper(&signature, raw_index, &types, &lookup).unwrap());
            code.function(&export_wrapper(&signature, internal_index, &types, &lookup).unwrap());
            let mut module = Module::new();
            module.section(&type_section);
            module.section(&imports);
            module.section(&functions);
            module.section(&code);
            wasmparser::Validator::new()
                .validate_all(&module.finish())
                .unwrap();
        }
    }

    #[test]
    fn all_host_types_roundtrip_through_compiled_gleam_and_javascript() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("src")).unwrap();
        std::fs::write(
            directory.path().join("gleam.toml"),
            "name = \"abi_test\"\nversion = \"1.0.0\"\ntarget = \"wasm\"\n",
        )
        .unwrap();
        std::fs::write(
            directory.path().join("src/abi_test.gleam"),
            r#"
pub type HostReference

@external(wasm, "custom:abi/test", "echo_nil")
fn host_nil(value: Nil) -> Nil
@external(wasm, "custom:abi/test", "echo_bool")
fn host_bool(value: Bool) -> Bool
@external(wasm, "custom:abi/test", "echo_float")
fn host_float(value: Float) -> Float
@external(wasm, "custom:abi/test", "echo_int")
fn host_int(value: Int) -> Int
@external(wasm, "custom:abi/test", "echo_string")
fn host_string(value: String) -> String
@external(wasm, "custom:abi/test", "echo_bits")
fn host_bits(value: BitArray) -> BitArray
@external(wasm, "custom:abi/test", "echo_ref")
fn host_ref(value: HostReference) -> HostReference
@external(wasm, "custom:abi/test", "mixed")
fn host_mixed(
  first: Nil, boolean: Bool, second: Nil, float: Float, integer: Int,
  text: String, bits: BitArray, reference: HostReference, last: Nil,
) -> HostReference

pub fn echo_nil(value: Nil) -> Nil { host_nil(value) }
pub fn echo_bool(value: Bool) -> Bool { host_bool(value) }
pub fn echo_float(value: Float) -> Float { host_float(value) }
pub fn echo_int(value: Int) -> Int { host_int(value) }
pub fn echo_string(value: String) -> String { host_string(value) }
pub fn echo_bits(value: BitArray) -> BitArray { host_bits(value) }
pub fn echo_ref(value: HostReference) -> HostReference { host_ref(value) }
pub fn same_reference(left: HostReference, right: HostReference) -> Bool {
  left == right
}
pub fn mixed(
  first: Nil, boolean: Bool, second: Nil, float: Float, integer: Int,
  text: String, bits: BitArray, reference: HostReference, last: Nil,
) -> HostReference {
  host_mixed(first, boolean, second, float, integer, text, bits, reference, last)
}
pub fn unaligned_bits() -> BitArray {
  let assert <<_:3, tail:bits>> = <<5:3, 511:9>>
  tail
}
"#,
        )
        .unwrap();
        let checked = wleam_frontend::check_project(directory.path(), &Default::default()).unwrap();
        let options = crate::Options {
            library: true,
            exports: [
                "echo_nil",
                "echo_bool",
                "echo_float",
                "echo_int",
                "echo_string",
                "echo_bits",
                "echo_ref",
                "same_reference",
                "mixed",
                "unaligned_bits",
            ]
            .map(|name| format!("abi_test.{name}"))
            .to_vec(),
            ..Default::default()
        };
        let program = crate::lower::lower(&checked.modules, &options.exports).unwrap();
        let artifact = crate::binary::compile(&program, &options).unwrap();
        assert_eq!(artifact.abi.imports["custom:abi/test"].len(), 8);
        let binary = directory.path().join("program.wasm");
        std::fs::write(&binary, artifact.wasm).unwrap();
        let sdk = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bindings/javascript/wleam.mjs")
            .canonicalize()
            .unwrap();
        let script = directory.path().join("execute.mjs");
        std::fs::write(
            &script,
            format!(
                r#"
import {{ readFile }} from 'node:fs/promises';
import {{ strict as assert }} from 'node:assert';
import {{ instantiate }} from {};
const count = new Map();
let guest, fail = false, reenter = false;
const record = (name, value) => {{ count.set(name, (count.get(name) ?? 0) + 1); return value; }};
const reference = {{ token: Symbol('host identity') }};
const mixedInteger = -(1n << 257n) + 12345n;
const mixedBits = {{ bytes: Uint8Array.of(0xaf, 0x80), bitLength: 9 }};
const callbacks = {{
  echo_nil(value) {{ assert.equal(value, undefined); return record('nil', value); }},
  echo_bool(value) {{ assert.equal(typeof value, 'boolean'); return record('bool', value); }},
  echo_float(value) {{ assert.ok(Number.isFinite(value)); return record('float', value); }},
  echo_int(value) {{ assert.equal(typeof value, 'bigint'); return record('int', value); }},
  echo_string(value) {{
    assert.equal(typeof value, 'string');
    if (fail) throw new Error('expected host failure');
    if (reenter) assert.equal(guest.call('abi_test.echo_int', [mixedInteger]), mixedInteger);
    return record('string', value);
  }},
  echo_bits(value) {{ assert.ok(value.bytes instanceof Uint8Array); return record('bits', value); }},
  echo_ref(value) {{ return record('ref', value); }},
  mixed(...args) {{
    assert.deepEqual(args.slice(0, 7), [undefined, true, undefined, -0, mixedInteger, 'e\u0301 中文 👩🏽‍🚀\0', mixedBits]);
    assert.equal(args[7], reference);
    assert.equal(args[8], undefined);
    assert.equal(args.length, 9);
    return record('mixed', args[7]);
  }},
}};
guest = await instantiate(await readFile(process.argv[2]), {{ imports: {{ 'custom:abi/test': callbacks }} }});
const call = (name, ...args) => guest.call(`abi_test.${{name}}`, args);
const heap = () => {{ const mark = guest.exports.__wleam_abi_mark(); guest.exports.__wleam_abi_reset(mark); return mark; }};
const initialHeap = heap();
assert.equal(call('echo_nil', undefined), undefined);
assert.equal(call('echo_nil', null), undefined);
for (const value of [true, false]) assert.equal(call('echo_bool', value), value);
for (const value of [-0, 0, Number.MIN_VALUE, Number.MAX_VALUE, -1.2345, 1e-200]) assert.equal(call('echo_float', value), value);
for (const value of [0n, 1n, -1n, (1n << 30n) - 1n, 1n << 30n, -(1n << 30n), -(1n << 64n), mixedInteger, (1n << 4096n) + 0x123456789abcdefn]) assert.equal(call('echo_int', value), value);
for (const value of ['', 'a\0b', 'e\u0301 中文 👩🏽‍🚀', '長'.repeat(50000)]) assert.equal(call('echo_string', value), value);
for (const value of [{{ bytes: new Uint8Array(), bitLength: 0 }}, {{ bytes: Uint8Array.of(0xa8), bitLength: 5 }}, mixedBits, {{ bytes: Uint8Array.of(0xff, 0, 0x80), bitLength: 17 }}]) {{
  const result = call('echo_bits', value);
  assert.deepEqual(result, value);
  assert.notEqual(result.bytes.buffer, value.bytes.buffer);
}}
assert.deepEqual(call('unaligned_bits'), {{ bytes: Uint8Array.of(0xff, 0x80), bitLength: 9 }});
for (const value of [reference, null, undefined, 'opaque primitive']) assert.equal(call('echo_ref', value), value);
assert.equal(call('same_reference', reference, reference), true);
assert.equal(call('same_reference', reference, {{}}), false);
assert.equal(call('same_reference', null, null), true);
assert.equal(call('mixed', undefined, true, null, -0, mixedInteger, 'e\u0301 中文 👩🏽‍🚀\0', mixedBits, reference, undefined), reference);
assert.throws(() => call('echo_float', Infinity), /finite/);
assert.throws(() => call('echo_float', NaN), /finite/);
fail = true;
assert.throws(() => call('echo_string', 'owned temporary during failure'), /expected host failure/);
fail = false;
reenter = true;
assert.equal(call('echo_string', 'nested ABI frames'), 'nested ABI frames');
reenter = false;
for (let iteration = 0; iteration < 32; iteration++) {{
  assert.equal(call('echo_int', mixedInteger), mixedInteger);
  assert.equal(call('echo_string', 'repeated Unicode 🌍'), 'repeated Unicode 🌍');
}}
assert.equal(heap(), initialHeap, 'ABI frames must reclaim successful, failed and nested transfer allocations');
assert.deepEqual([...count.keys()].sort(), ['bits', 'bool', 'float', 'int', 'mixed', 'nil', 'ref', 'string']);
"#,
                serde_json::to_string(&format!("file://{}", sdk.display())).unwrap()
            ),
        )
        .unwrap();
        let output =
            std::process::Command::new(std::env::var_os("NODE").unwrap_or_else(|| "node".into()))
                .arg(script)
                .arg(binary)
                .output()
                .expect("Node.js is required to execute the compiled WasmGC ABI test");
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
