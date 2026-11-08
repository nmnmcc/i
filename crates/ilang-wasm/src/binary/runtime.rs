//! Cached runtime assembly and binary-only linking of program sections.
//!
//! The runtime is fixed compiler data. It is assembled once per process; every
//! program body and wrapper supplied to this linker is already binary encoded.

use std::{borrow::Cow, collections::BTreeMap, convert::Infallible, sync::OnceLock};

use wasm_encoder::{
    CodeSection, DataSection, ElementSection, Elements, Encode, EntityType, ExportSection,
    Function, FunctionSection, GlobalSection, ImportSection, Module, NameMap, NameSection,
    RawSection, TypeSection,
    reencode::{self, Reencode},
};
use wasmparser::{BinaryReader, KnownCustom, Name, Parser, Payload, TypeRef};

use crate::{Error, abi, runtime};

/// Extra sections emitted directly from the program IR. All indices are final
/// module indices; replacement keys alone use the original runtime indices.
#[derive(Default)]
pub(super) struct Addition {
    pub extraimports: Vec<(String, String, u32)>,
    pub types: TypeSection,
    pub functions: Vec<(u32, Function)>,
    pub globals: GlobalSection,
    pub exports: ExportSection,
    pub data: DataSection,
    pub replacement_functions: BTreeMap<u32, Function>,
    pub names: BTreeMap<u32, String>,
    pub declare_functions: Vec<u32>,
    pub custom_abi: Vec<u8>,
}

pub(super) struct Runtime {
    bytes: Vec<u8>,
    pub functions: BTreeMap<String, u32>,
    pub types: BTreeMap<String, u32>,
    pub imported_functions: u32,
    /// All functions, including imported functions.
    pub function_count: u32,
    /// Individual types, rather than recursive groups.
    pub type_count: u32,
    pub global_count: u32,
    pub data_count: u32,
}

impl Runtime {
    pub fn get() -> Result<&'static Self, Error> {
        static RUNTIME: OnceLock<Result<Runtime, String>> = OnceLock::new();
        RUNTIME
            .get_or_init(Self::assemble)
            .as_ref()
            .map_err(|error| Error::Encoding(error.clone()))
    }

    fn assemble() -> Result<Self, String> {
        // Keep these signatures stable: the direct emitter replaces only their
        // bodies, leaving every cached runtime type and function index intact.
        const METADATA: &str = r#"
(func $constructor_name (param i32) (result eqref) unreachable)
(func $type_name (param i64) (result eqref) unreachable)
(func $type_is_dict (param i64) (result i32) unreachable)
(func $dynamic_classify (param eqref) (result eqref) unreachable)
"#;
        let source = format!(
            "(module {} {} {} {} {} {})",
            runtime::TYPES,
            abi::type_wat(),
            runtime::HOST,
            runtime::wat(),
            abi::runtime_wat(),
            METADATA
        );
        let bytes = wat::parse_str(source).map_err(|error| error.to_string())?;
        wasmparser::Validator::new()
            .validate_all(&bytes)
            .map_err(|error| error.to_string())?;
        let mut result = Self {
            bytes: Vec::new(),
            functions: BTreeMap::new(),
            types: BTreeMap::new(),
            imported_functions: 0,
            function_count: 0,
            type_count: 0,
            global_count: 0,
            data_count: 0,
        };
        for payload in Parser::new(0).parse_all(&bytes) {
            match payload.map_err(|error| error.to_string())? {
                Payload::TypeSection(section) => {
                    for group in section {
                        result.type_count +=
                            group.map_err(|error| error.to_string())?.types().len() as u32;
                    }
                }
                Payload::ImportSection(section) => {
                    for import in section.into_imports() {
                        match import.map_err(|error| error.to_string())?.ty {
                            TypeRef::Func(_) | TypeRef::FuncExact(_) => {
                                result.imported_functions += 1
                            }
                            TypeRef::Global(_) => result.global_count += 1,
                            _ => {}
                        }
                    }
                }
                Payload::FunctionSection(section) => result.function_count += section.count(),
                Payload::GlobalSection(section) => result.global_count += section.count(),
                Payload::DataSection(section) => result.data_count += section.count(),
                Payload::CustomSection(section) => {
                    if let KnownCustom::Name(names) = section.as_known() {
                        for name in names {
                            match name.map_err(|error| error.to_string())? {
                                Name::Function(names) => {
                                    for name in names {
                                        let name = name.map_err(|error| error.to_string())?;
                                        result.functions.insert(name.name.to_owned(), name.index);
                                    }
                                }
                                Name::Type(names) => {
                                    for name in names {
                                        let name = name.map_err(|error| error.to_string())?;
                                        result.types.insert(name.name.to_owned(), name.index);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        result.function_count += result.imported_functions;
        result.bytes = bytes;
        Ok(result)
    }

    pub fn function(&self, name: &str, extraimports: u32) -> Result<u32, Error> {
        self.functions
            .get(name.strip_prefix('$').unwrap_or(name))
            .copied()
            .map(|index| self.shift_function(index, extraimports))
            .ok_or_else(|| Error::Codegen(format!("unknown runtime function {name}")))
    }

    fn shift_function(&self, index: u32, extraimports: u32) -> u32 {
        index
            + if index >= self.imported_functions {
                extraimports
            } else {
                0
            }
    }

    pub fn finish(&self, addition: &Addition) -> Result<Vec<u8>, Error> {
        let extraimports = u32::try_from(addition.extraimports.len())
            .map_err(|_| Error::Codegen("too many host imports".into()))?;
        let mut base = Module::new();
        let mut reencoder = RuntimeReencoder {
            imported: self.imported_functions,
            shift: extraimports,
            next_body: self.imported_functions,
            replacements: &addition.replacement_functions,
        };
        reencoder
            .parse_core_module(&mut base, Parser::new(0), &self.bytes)
            .map_err(|error| Error::Encoding(error.to_string()))?;
        let base = base.finish();
        let mut sections = BTreeMap::<u8, Vec<u8>>::new();
        for payload in Parser::new(0).parse_all(&base) {
            let payload = payload.map_err(|error| Error::Encoding(error.to_string()))?;
            if let Some((id, range)) = payload.as_section()
                && id != 0
            {
                sections.insert(id, base[range.start as usize..range.end as usize].to_vec());
            }
        }
        append_section(&mut sections, 1, &addition.types)?;
        let mut imports = ImportSection::new();
        for (module, name, typ) in &addition.extraimports {
            imports.import(module, name, EntityType::Function(*typ));
        }
        append_section(&mut sections, 2, &imports)?;
        let mut functions = FunctionSection::new();
        let mut code = CodeSection::new();
        for (typ, function) in &addition.functions {
            functions.function(*typ);
            code.function(function);
        }
        append_section(&mut sections, 3, &functions)?;
        append_section(&mut sections, 6, &addition.globals)?;
        append_section(&mut sections, 7, &addition.exports)?;
        let mut elements = ElementSection::new();
        if !addition.declare_functions.is_empty() {
            elements.declared(Elements::Functions(Cow::Borrowed(
                &addition.declare_functions,
            )));
        }
        append_section(&mut sections, 9, &elements)?;
        append_section(&mut sections, 10, &code)?;
        append_section(&mut sections, 11, &addition.data)?;
        let total_data = self
            .data_count
            .checked_add(addition.data.len())
            .ok_or_else(|| Error::Codegen("too many data segments".into()))?;
        if total_data > 0 {
            let mut count = Vec::new();
            total_data.encode(&mut count);
            sections.insert(12, count);
        }
        let mut module = Module::new();
        // Tag comes before global; data count comes before code. Numeric section
        // order is otherwise valid for the current Core Wasm specification.
        for id in [1, 2, 3, 4, 5, 13, 6, 7, 8, 9, 12, 10, 11] {
            if let Some(data) = sections.remove(&id) {
                module.section(&RawSection { id, data: &data });
            }
        }
        if !sections.is_empty() {
            return Err(Error::Encoding("unsupported cached runtime section".into()));
        }
        let mut names = NameSection::new();
        let mut function_names = BTreeMap::new();
        for (name, index) in &self.functions {
            function_names.insert(self.shift_function(*index, extraimports), name.clone());
        }
        function_names.extend(
            addition
                .names
                .iter()
                .map(|(index, name)| (*index, name.clone())),
        );
        let mut map = NameMap::new();
        for (index, name) in &function_names {
            map.append(*index, name);
        }
        names.functions(&map);
        let mut types = self
            .types
            .iter()
            .map(|(name, index)| (*index, name))
            .collect::<Vec<_>>();
        types.sort_unstable_by_key(|(index, _)| *index);
        let mut map = NameMap::new();
        for (index, name) in types {
            map.append(index, name);
        }
        names.types(&map);
        module.section(&names);
        module.section(&wasm_encoder::CustomSection {
            name: Cow::Borrowed("ilang.abi"),
            data: Cow::Borrowed(&addition.custom_abi),
        });
        Ok(module.finish())
    }
}

/// Merge two vector section payloads without decoding or copying each entry.
/// This preserves recursive type groups and pre-encoded function bodies.
fn append_section<T: Encode>(
    sections: &mut BTreeMap<u8, Vec<u8>>,
    id: u8,
    addition: &T,
) -> Result<(), Error> {
    let mut encoded = Vec::new();
    addition.encode(&mut encoded);
    let mut reader = BinaryReader::new(&encoded, 0);
    reader
        .read_var_u32()
        .map_err(|error| Error::Encoding(error.to_string()))?; // payload length
    let offset = reader.original_position() as usize;
    let extra = &encoded[offset..];
    let (count, entries) = vector_entries(extra)?;
    if count == 0 {
        return Ok(());
    }
    if let Some(original) = sections.get_mut(&id) {
        let (base_count, base_entries) = vector_entries(original)?;
        let mut joined = Vec::with_capacity(original.len() + entries.len() + 5);
        base_count
            .checked_add(count)
            .ok_or_else(|| Error::Encoding("section count overflow".into()))?
            .encode(&mut joined);
        joined.extend_from_slice(base_entries);
        joined.extend_from_slice(entries);
        *original = joined;
    } else {
        sections.insert(id, extra.to_vec());
    }
    Ok(())
}

fn vector_entries(bytes: &[u8]) -> Result<(u32, &[u8]), Error> {
    let mut reader = BinaryReader::new(bytes, 0);
    let count = reader
        .read_var_u32()
        .map_err(|error| Error::Encoding(error.to_string()))?;
    Ok((count, &bytes[reader.original_position() as usize..]))
}

struct RuntimeReencoder<'a> {
    imported: u32,
    shift: u32,
    next_body: u32,
    replacements: &'a BTreeMap<u32, Function>,
}

impl Reencode for RuntimeReencoder<'_> {
    type Error = Infallible;

    fn function_index(&mut self, index: u32) -> Result<u32, reencode::Error<Self::Error>> {
        Ok(index
            + if index >= self.imported {
                self.shift
            } else {
                0
            })
    }

    fn parse_function_body(
        &mut self,
        code: &mut CodeSection,
        body: wasmparser::FunctionBody<'_>,
    ) -> Result<(), reencode::Error<Self::Error>> {
        let original = self.next_body;
        self.next_body += 1;
        if let Some(replacement) = self.replacements.get(&original) {
            code.function(replacement);
            Ok(())
        } else if self.shift == 0 {
            // The common case has only the fixed host imports. Preserve the
            // cached runtime instructions without walking their operators.
            code.raw(body.as_bytes());
            Ok(())
        } else {
            reencode::utils::parse_function_body(self, code, body)
        }
    }

    fn parse_custom_section(
        &mut self,
        _module: &mut Module,
        _section: wasmparser::CustomSectionReader<'_>,
    ) -> Result<(), reencode::Error<Self::Error>> {
        // Rebuild names after final linking; runtime has no semantic custom data.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_encoder::{ConstExpr, ExportKind, GlobalType, HeapType, Instruction as I, ValType};

    #[test]
    fn cached_runtime_is_valid_and_reused() {
        let first = Runtime::get().unwrap();
        let second = Runtime::get().unwrap();
        assert!(std::ptr::eq(first, second));
        assert_eq!(first.imported_functions, 4);
        for name in ["bytes", "values", "limbs", "call", "obj", "hostbox"] {
            assert!(
                first.types.contains_key(name),
                "missing runtime type {name}"
            );
        }
        wasmparser::Validator::new()
            .validate_all(&first.bytes)
            .unwrap();
        let linked = first.finish(&Addition::default()).unwrap();
        wasmparser::Validator::new().validate_all(&linked).unwrap();
    }

    #[test]
    fn links_imports_gc_data_globals_and_replacement_bodies() {
        let runtime = Runtime::get().unwrap();
        let mut addition = Addition::default();
        let extra_type = runtime.type_count;
        addition.types.ty().function([], [ValType::I32]);
        addition
            .extraimports
            .push(("test".into(), "answer".into(), extra_type));
        let foreign = runtime.imported_functions;
        let added_function = runtime.function_count + 1;
        addition.declare_functions.push(added_function);
        addition
            .exports
            .export("test", ExportKind::Func, added_function);
        addition.names.insert(added_function, "binary_test".into());
        addition.globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &ConstExpr::i32_const(7),
        );
        addition.data.passive([1, 2, 3, 4]);
        let mut body = Function::new([]);
        body.instruction(&I::RefFunc(added_function));
        body.instruction(&I::Drop);
        body.instruction(&I::I32Const(0));
        body.instruction(&I::I32Const(4));
        body.instruction(&I::ArrayNewData {
            array_type_index: runtime.types["bytes"],
            array_data_index: runtime.data_count,
        });
        body.instruction(&I::Drop);
        body.instruction(&I::RefNull(HeapType::Abstract {
            shared: false,
            ty: wasm_encoder::AbstractHeapType::Eq,
        }));
        body.instruction(&I::Drop);
        body.instruction(&I::GlobalGet(runtime.global_count));
        body.instruction(&I::Drop);
        body.instruction(&I::Call(foreign));
        body.instruction(&I::End);
        addition.functions.push((extra_type, body));
        let original = runtime.functions["type_is_dict"];
        let mut replacement = Function::new([]);
        replacement.instruction(&I::I32Const(0));
        replacement.instruction(&I::End);
        addition.replacement_functions.insert(original, replacement);
        addition.custom_abi = br#"{"version":1,"tests":[],"exports":{},"imports":{}}"#.to_vec();
        let bytes = runtime.finish(&addition).unwrap();
        wasmparser::Validator::new().validate_all(&bytes).unwrap();
        assert_eq!(
            runtime.function("int_add", 1).unwrap(),
            runtime.functions["int_add"] + 1
        );
        assert_eq!(runtime.function("host_write", 1).unwrap(), 0);
    }
}
