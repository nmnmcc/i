//! Direct Wasm binary emission from the compact language IR.
//! Only the invariant runtime template is assembled once; every program
//! instruction and public ABI wrapper is encoded with `wasm-encoder`.
mod abi;
mod expressions;
mod pattern;
mod runtime;

use crate::{Artifact, Error, Options, ir};
use expressions::location_text;
use runtime::{Addition, Runtime};
use std::collections::BTreeMap;
use wasm_encoder::{
    AbstractHeapType, BlockType, ConstExpr, ExportKind, Function, GlobalType, HeapType,
    Instruction, RefType, ValType,
};

const EQREF: ValType = ValType::Ref(RefType::EQREF);
const EQ: HeapType = HeapType::Abstract {
    shared: false,
    ty: AbstractHeapType::Eq,
};
fn heap(index: u32) -> HeapType {
    HeapType::Concrete(index)
}
fn reference(index: u32, nullable: bool) -> ValType {
    ValType::Ref(RefType {
        nullable,
        heap_type: heap(index),
    })
}

#[derive(Clone, Copy)]
struct Types {
    bytes: u32,
    values: u32,
    limbs: u32,
    call: u32,
    obj: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Label(u32);
struct Context {
    instructions: Vec<Instruction<'static>>,
    locals: Vec<ValType>,
    variables: BTreeMap<String, u32>,
    parameters: u32,
    labels: Vec<Label>,
    next_label: u32,
}
impl Context {
    fn new(parameters: u32) -> Self {
        Self {
            instructions: Vec::new(),
            locals: Vec::new(),
            variables: BTreeMap::new(),
            parameters,
            labels: Vec::new(),
            next_label: 0,
        }
    }
    fn emit(&mut self, instruction: Instruction<'static>) {
        self.instructions.push(instruction);
    }
    fn local(&mut self, typ: ValType) -> u32 {
        let index = self.parameters + self.locals.len() as u32;
        self.locals.push(typ);
        index
    }
    fn variable(&mut self, name: &str) -> u32 {
        if let Some(index) = self.variables.get(name) {
            *index
        } else {
            let index = self.local(EQREF);
            self.variables.insert(name.to_owned(), index);
            index
        }
    }
    fn label(&mut self) -> Label {
        let label = Label(self.next_label);
        self.next_label += 1;
        self.labels.push(label);
        label
    }
    fn push_block(&mut self, typ: BlockType) -> Label {
        self.emit(Instruction::Block(typ));
        self.label()
    }
    fn push_if(&mut self, typ: BlockType) -> Label {
        self.emit(Instruction::If(typ));
        self.label()
    }
    fn else_(&mut self) {
        self.emit(Instruction::Else);
    }
    fn end(&mut self) {
        self.labels.pop().expect("structured control block");
        self.emit(Instruction::End);
    }
    fn depth(&self, label: Label) -> u32 {
        self.labels
            .iter()
            .rev()
            .position(|candidate| *candidate == label)
            .expect("active branch label") as u32
    }
    fn br(&mut self, label: Label) {
        self.emit(Instruction::Br(self.depth(label)));
    }
    fn br_if(&mut self, label: Label) {
        self.emit(Instruction::BrIf(self.depth(label)));
    }
    fn finish(self) -> Function {
        assert!(self.labels.is_empty());
        let mut function = Function::new(self.locals.into_iter().map(|typ| (1, typ)));
        for instruction in self.instructions {
            function.instruction(&instruction);
        }
        function.instruction(&Instruction::End);
        function
    }
}

struct Backend<'a> {
    program: &'a ir::Program,
    runtime: &'static Runtime,
    types: Types,
    addition: Addition,
    extra_imports: u32,
    functions: BTreeMap<String, u32>,
    constructors: BTreeMap<u32, u32>,
    constants: BTreeMap<String, (u32, u32, u32)>,
    definitions: Vec<(u32, Option<Function>)>,
    signatures: Vec<(Vec<ValType>, Vec<ValType>, u32)>,
    lambda_count: u32,
    current_name: String,
    current_location: Option<ir::Location>,
    source_locations: BTreeMap<u32, ir::Location>,
}
impl<'a> Backend<'a> {
    fn runtime_function(&self, name: &str) -> Result<u32, Error> {
        self.runtime.function(name, self.extra_imports)
    }
    fn call(&self, name: &str, ctx: &mut Context) -> Result<(), Error> {
        ctx.emit(Instruction::Call(self.runtime_function(name)?));
        Ok(())
    }
    fn signature(&mut self, params: Vec<ValType>, results: Vec<ValType>) -> u32 {
        if let Some((_, _, index)) = self
            .signatures
            .iter()
            .find(|(p, r, _)| *p == params && *r == results)
        {
            return *index;
        }
        let index = self.runtime.type_count + self.addition.types.len();
        self.addition
            .types
            .ty()
            .function(params.iter().copied(), results.iter().copied());
        self.signatures.push((params, results, index));
        index
    }
    fn reserve(&mut self, typ: u32, name: String) -> u32 {
        let index =
            self.runtime.function_count + self.extra_imports + self.definitions.len() as u32;
        self.definitions.push((typ, None));
        self.addition.names.insert(index, name);
        index
    }
    fn define(&mut self, index: u32, body: Function) {
        self.definitions[(index - self.runtime.function_count - self.extra_imports) as usize].1 =
            Some(body);
    }
    fn function_index(&self, name: &str) -> Result<u32, Error> {
        self.functions
            .get(name)
            .copied()
            .ok_or_else(|| Error::Codegen(format!("unknown function {name}")))
    }
    fn find_function(&self, name: &str) -> Result<&ir::Function, Error> {
        self.program
            .functions
            .iter()
            .find(|f| f.symbol == name)
            .ok_or_else(|| Error::Codegen(format!("unknown entry or export {name}")))
    }
    fn function_body(
        &mut self,
        args: &[String],
        captures: &[String],
        body: &ir::Expr,
    ) -> Result<Function, Error> {
        let mut ctx = Context::new(2);
        for (parameter, values) in [(0, captures), (1, args)] {
            for (index, name) in values.iter().enumerate() {
                let local = ctx.variable(name);
                ctx.emit(Instruction::LocalGet(parameter));
                ctx.emit(Instruction::RefCastNonNull(heap(self.types.values)));
                ctx.emit(Instruction::I32Const(index as i32));
                ctx.emit(Instruction::ArrayGet(self.types.values));
                ctx.emit(Instruction::LocalSet(local));
            }
        }
        self.expression(body, &mut ctx, true)?;
        Ok(ctx.finish())
    }
    fn empty_arguments(&self, ctx: &mut Context) {
        ctx.emit(Instruction::RefNull(EQ));
        ctx.emit(Instruction::ArrayNewFixed {
            array_type_index: self.types.values,
            array_size: 0,
        });
    }
    fn zero_wrapper(&self, target: Option<u32>) -> Function {
        let mut ctx = Context::new(0);
        if let Some(target) = target {
            self.empty_arguments(&mut ctx);
            ctx.emit(Instruction::Call(target));
            ctx.emit(Instruction::Drop);
        }
        ctx.finish()
    }
}

pub fn compile(program: &ir::Program, options: &Options) -> Result<Artifact, Error> {
    let runtime = Runtime::get()?;
    let typ = |name: &str| {
        runtime
            .types
            .get(name)
            .copied()
            .ok_or_else(|| Error::Codegen(format!("runtime type {name} is missing")))
    };
    let types = Types {
        bytes: typ("bytes")?,
        values: typ("values")?,
        limbs: typ("limbs")?,
        call: typ("call")?,
        obj: typ("obj")?,
    };
    let extra_imports = program
        .functions
        .iter()
        .filter(|f| {
            f.external
                .as_ref()
                .is_some_and(|(module, _)| module != "ilang:runtime/v1")
        })
        .count() as u32;
    let mut backend = Backend {
        program,
        runtime,
        types,
        addition: Addition::default(),
        extra_imports,
        functions: BTreeMap::new(),
        constructors: BTreeMap::new(),
        constants: BTreeMap::new(),
        definitions: Vec::new(),
        signatures: Vec::new(),
        lambda_count: 0,
        current_name: String::new(),
        current_location: None,
        source_locations: BTreeMap::new(),
    };
    let mut metadata = crate::abi::Abi {
        version: 1,
        ..Default::default()
    };
    let mut foreign = BTreeMap::new();
    for function in &program.functions {
        if let Some((module, name)) = &function.external
            && module != "ilang:runtime/v1"
        {
            let signature = public_signature(function, &program.external_types)?;
            if let Some(previous) = metadata
                .imports
                .entry(module.clone())
                .or_default()
                .insert(name.clone(), signature.clone())
                && previous != signature
            {
                return Err(Error::Abi(format!(
                    "inconsistent signatures for {module}.{name}"
                )));
            }
            let parameters = signature
                .params
                .iter()
                .flat_map(|typ| abi::flattened_types(*typ))
                .collect();
            let results = abi::flattened_types(signature.result);
            let type_index = backend.signature(parameters, results);
            let raw_index = runtime.imported_functions + backend.addition.extraimports.len() as u32;
            backend
                .addition
                .extraimports
                .push((module.clone(), name.clone(), type_index));
            foreign.insert(function.symbol.clone(), (raw_index, signature));
        }
        let index = backend.reserve(
            types.call,
            format!(
                "{} ({})",
                function.symbol,
                location_text(&function.location)
            ),
        );
        backend
            .source_locations
            .insert(index, function.location.clone());
        backend.functions.insert(function.symbol.clone(), index);
    }
    for constructor in &program.constructors {
        let index = backend.reserve(types.call, format!("constructor.{}", constructor.symbol));
        backend.constructors.insert(constructor.tag, index);
    }
    let constant_type = backend.signature(Vec::new(), vec![EQREF]);
    for (ordinal, name) in program.constants.keys().enumerate() {
        let function = backend.reserve(constant_type, format!("constant.{name}"));
        let value = runtime.global_count + ordinal as u32 * 2;
        let ready = value + 1;
        backend.addition.globals.global(
            GlobalType {
                val_type: EQREF,
                mutable: true,
                shared: false,
            },
            &ConstExpr::ref_null(EQ),
        );
        backend.addition.globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &ConstExpr::i32_const(0),
        );
        backend
            .constants
            .insert(name.clone(), (function, value, ready));
    }
    for function in &program.functions {
        backend.current_name = function.symbol.clone();
        backend.current_location = Some(function.location.clone());
        let index = backend.function_index(&function.symbol)?;
        let body = if let Some((module, name)) = &function.external {
            if module == "ilang:runtime/v1" {
                let mut ctx = Context::new(2);
                for argument in 0..function.arguments.len() {
                    ctx.emit(Instruction::LocalGet(1));
                    ctx.emit(Instruction::RefCastNonNull(heap(types.values)));
                    ctx.emit(Instruction::I32Const(argument as i32));
                    ctx.emit(Instruction::ArrayGet(types.values));
                }
                backend.call(name, &mut ctx)?;
                ctx.finish()
            } else {
                let (raw, signature) = &foreign[&function.symbol];
                abi::import_wrapper(signature, *raw, &types, &|name| {
                    backend.runtime_function(name)
                })?
            }
        } else {
            backend.function_body(&function.arguments, &[], &function.body)?
        };
        backend.define(index, body);
    }
    for constructor in &program.constructors {
        let mut ctx = Context::new(2);
        ctx.emit(Instruction::I32Const(constructor.tag as i32));
        ctx.emit(Instruction::I64Const(constructor.type_id as i64));
        ctx.emit(Instruction::LocalGet(1));
        ctx.emit(Instruction::RefCastNonNull(heap(types.values)));
        backend.call("record", &mut ctx)?;
        backend.define(backend.constructors[&constructor.tag], ctx.finish());
    }
    for (name, expression) in &program.constants {
        let (index, value, ready) = backend.constants[name];
        let mut ctx = Context::new(0);
        ctx.emit(Instruction::GlobalGet(ready));
        ctx.emit(Instruction::I32Eqz);
        ctx.push_if(BlockType::Empty);
        backend.expression(expression, &mut ctx, false)?;
        ctx.emit(Instruction::GlobalSet(value));
        ctx.emit(Instruction::I32Const(1));
        ctx.emit(Instruction::GlobalSet(ready));
        ctx.end();
        ctx.emit(Instruction::GlobalGet(value));
        backend.define(index, ctx.finish());
    }
    let void_type = backend.signature(Vec::new(), Vec::new());
    let init = backend.reserve(void_type, "__ilang_init".into());
    backend.define(init, backend.zero_wrapper(None));
    backend
        .addition
        .exports
        .export("__ilang_init", ExportKind::Func, init);
    if !options.library {
        if !backend.find_function(&options.entry)?.arguments.is_empty() {
            return Err(Error::Abi(format!(
                "entry {} must have no arguments",
                options.entry
            )));
        }
        let target = backend.function_index(&options.entry)?;
        let entry = backend.reserve(void_type, "_start".into());
        backend.define(entry, backend.zero_wrapper(Some(target)));
        backend
            .addition
            .exports
            .export("_start", ExportKind::Func, entry);
        metadata.entry = Some("_start".into());
    }
    for name in &options.exports {
        let signature = public_signature(backend.find_function(name)?, &program.external_types)?;
        let typ = backend.signature(
            signature
                .params
                .iter()
                .flat_map(|typ| abi::flattened_types(*typ))
                .collect(),
            abi::flattened_types(signature.result),
        );
        let body =
            abi::export_wrapper(&signature, backend.function_index(name)?, &types, &|name| {
                backend.runtime_function(name)
            })?;
        let index = backend.reserve(typ, format!("export.{name}"));
        backend.define(index, body);
        backend
            .addition
            .exports
            .export(name, ExportKind::Func, index);
        metadata.exports.insert(name.clone(), signature);
    }
    for name in &options.tests {
        let function = backend.find_function(name)?;
        if !function.arguments.is_empty() || !function.return_type.is_nil() {
            return Err(Error::Abi(format!(
                "test {name} must take no arguments and return Nil"
            )));
        }
        let target = backend.function_index(name)?;
        let index = backend.reserve(void_type, format!("test.{name}"));
        backend.define(index, backend.zero_wrapper(Some(target)));
        backend
            .addition
            .exports
            .export(name, ExportKind::Func, index);
        metadata.tests.push(name.clone());
    }
    for (name, body) in
        abi::generate_metadata(program, &types, &|name| backend.runtime_function(name))?
    {
        let index = runtime
            .functions
            .get(&name)
            .copied()
            .ok_or_else(|| Error::Codegen(format!("runtime metadata {name} is missing")))?;
        backend.addition.replacement_functions.insert(index, body);
    }
    for (ordinal, (typ, body)) in backend.definitions.into_iter().enumerate() {
        backend
            .addition
            .declare_functions
            .push(runtime.function_count + extra_imports + ordinal as u32);
        backend
            .addition
            .functions
            .push((typ, body.expect("reserved function must be defined")));
    }
    backend.addition.custom_abi =
        serde_json::to_vec(&metadata).map_err(|error| Error::Abi(error.to_string()))?;
    let mut wasm = runtime.finish(&backend.addition)?;
    let locations = serde_json::to_vec(&serde_json::json!({
        "version": 1,
        "functions": backend.source_locations,
    }))
    .map_err(|error| Error::Encoding(error.to_string()))?;
    let section = wasm_encoder::CustomSection {
        name: "ilang.source".into(),
        data: locations.into(),
    };
    wasm.push(wasm_encoder::Section::id(&section));
    wasm_encoder::Encode::encode(&section, &mut wasm);
    wasmparser::Validator::new()
        .validate_all(&wasm)
        .map_err(|error| Error::Validation(error.to_string()))?;
    Ok(Artifact {
        wasm,
        abi: metadata,
    })
}

fn public_signature(
    function: &ir::Function,
    externals: &std::collections::BTreeSet<String>,
) -> Result<crate::abi::Signature, Error> {
    Ok(crate::abi::Signature {
        params: function
            .argument_types
            .iter()
            .map(|typ| crate::abi::type_of_with_externals(typ, externals).map_err(Error::Abi))
            .collect::<Result<_, _>>()?,
        result: crate::abi::type_of_with_externals(&function.return_type, externals)
            .map_err(Error::Abi)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_runtime_and_empty_library_validate() {
        let artifact = compile(
            &ir::Program::default(),
            &Options {
                library: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(artifact.wasm.starts_with(b"\0asm"));
        assert_eq!(artifact.abi.version, 1);
    }

    #[test]
    fn direct_binary_program_and_foreign_wrappers_execute() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("src")).unwrap();
        std::fs::write(
            directory.path().join("i.toml"),
            "name = \"direct_test\"\nversion = \"1.0.0\"\ntarget = \"wasm\"\n",
        )
        .unwrap();
        std::fs::write(
            directory.path().join("src/direct_test.i"),
            r#"
@external(wasm, "tests", "suffix")
fn suffix(value: String) -> String

pub fn foreign(value: String) -> String { suffix(value) }
pub fn identity_int(value: Int) -> Int { value }
pub fn identity_bits(value: BitArray) -> BitArray { value }

pub type Pair { Pair(Int, Int) }
const table = <<1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17>>

fn boolean_tail(value: Int) -> Bool { value == 0 || boolean_tail(value - 1) }
fn binding_tail(value: Int) -> Int {
  case value { 0 -> 42 _ -> { let result = binding_tail(value - 1) } }
}

pub fn language_test() -> Nil {
  assert boolean_tail(100_000)
  assert binding_tail(100_000) == 42
  let final_let = { let value = 42 }
  assert final_let == 42
  let capture = 7
  let add = fn(value) { value + capture }
  assert add(35) == 42
  let chooser = case #(1, 2) {
    #(x, _) | #(_, x) if x == 2 -> x
    _ -> 0
  }
  assert chooser == 2
  let Pair(a, b) = Pair(4, 5)
  assert a + b == 9
  let assert <<1, 2, rest:bytes>> = table
  assert rest == <<3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17>>
  assert <<4:size(-1)>> == <<>>
  assert case <<255, 255>> { <<_:bytes-unit(2)-size(3), _:bits>> -> True _ -> False }
  Nil
}
"#,
        )
        .unwrap();
        let checked = ilang_frontend::check_project(directory.path(), &Default::default()).unwrap();
        let options = Options {
            library: true,
            exports: vec![
                "direct_test.foreign".into(),
                "direct_test.identity_int".into(),
                "direct_test.identity_bits".into(),
            ],
            tests: vec!["direct_test.language_test".into()],
            ..Default::default()
        };
        let mut roots = options.exports.clone();
        roots.extend(options.tests.iter().cloned());
        let program = crate::lower::lower(&checked.modules, &roots).unwrap();
        let artifact = compile(&program, &options).unwrap();
        let binary = directory.path().join("program.wasm");
        std::fs::write(&binary, artifact.wasm).unwrap();
        let sdk = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bindings/javascript/ilang.mjs")
            .canonicalize()
            .unwrap();
        let script = directory.path().join("execute.mjs");
        std::fs::write(&script,format!(r#"
import {{ readFile }} from 'node:fs/promises';
import {{ strict as assert }} from 'node:assert';
import {{ instantiate }} from {};
const guest=await instantiate(await readFile(process.argv[2]),{{imports:{{tests:{{suffix:value=>value+"🌍"}}}}}});
guest.call("direct_test.language_test");
assert.equal(guest.call("direct_test.foreign",["héllo"]),"héllo🌍");
const integer=-(1n<<200n)+123n;
assert.equal(guest.call("direct_test.identity_int",[integer]),integer);
const bits={{bytes:Uint8Array.of(0xaf,0x80),bitLength:9}};
assert.deepEqual(guest.call("direct_test.identity_bits",[bits]),bits);
"#,serde_json::to_string(&format!("file://{}",sdk.display())).unwrap())).unwrap();
        let node = std::env::var_os("NODE").unwrap_or_else(|| "node".into());
        let output = std::process::Command::new(node)
            .arg(script)
            .arg(binary)
            .output()
            .expect("Node.js is required to execute WasmGC tests");
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
