//! Wasmtime embedding for Wleam's versioned, synchronous host ABI.
//!
//! Arguments are borrowed for the duration of a call. A returned buffer transfers
//! ownership to its receiver; it must be a fresh allocation, not an argument alias.
//! Public Rust values own copies, so no guest-memory borrow escapes a call.

#[cfg(test)]
mod tests;

use std::{
    any::Any,
    collections::{BTreeMap, HashMap},
    fmt,
    io::{self, Write},
    sync::{Arc, Weak},
};

use anyhow::{Context, Result, anyhow, bail, ensure};
pub use num_bigint::BigInt;
use num_bigint::Sign;
use serde::{Deserialize, Serialize};
use wasmtime::{
    AsContext, AsContextMut, Caller, Config, Engine, ExternRef, Func, FuncType,
    Instance as WasmInstance, Linker, Memory, Module, RootScope, Store, Val, ValType,
};

pub const ABI_VERSION: u32 = 1;
pub const HOST_MODULE: &str = "wleam:host/v1";

/// Types supported at a synchronous host boundary. Composite Gleam values stay
/// inside Wasm and can be passed through an explicitly chosen encoding.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AbiType {
    Nil,
    Bool,
    Float,
    Int,
    String,
    BitArray,
    Externref,
}

impl AbiType {
    fn wasm_types(self) -> Vec<ValType> {
        match self {
            Self::Nil => vec![],
            Self::Bool => vec![ValType::I32],
            Self::Float => vec![ValType::F64],
            Self::Int => vec![ValType::I32, ValType::I32, ValType::I32],
            Self::String | Self::BitArray => vec![ValType::I32, ValType::I32],
            Self::Externref => vec![ValType::EXTERNREF],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Signature {
    pub params: Vec<AbiType>,
    pub result: AbiType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Abi {
    pub version: u32,
    #[serde(default)]
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
            version: ABI_VERSION,
            entry: Some("_start".into()),
            tests: Vec::new(),
            exports: BTreeMap::new(),
            imports: BTreeMap::new(),
        }
    }
}

impl Abi {
    pub fn from_wasm(bytes: &[u8]) -> Result<Self> {
        Ok(Self::read_wasm(bytes)?.unwrap_or_default())
    }

    fn read_wasm(bytes: &[u8]) -> Result<Option<Self>> {
        let mut found = None;
        for payload in wasmparser::Parser::new(0).parse_all(bytes) {
            if let wasmparser::Payload::CustomSection(section) = payload?
                && section.name() == "wleam.abi"
            {
                ensure!(found.is_none(), "duplicate wleam.abi custom section");
                found = Some(
                    serde_json::from_slice::<Self>(section.data())
                        .context("invalid wleam.abi JSON")?,
                );
            }
        }
        if let Some(abi) = &found {
            abi.validate()?;
        }
        Ok(found)
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            self.version == ABI_VERSION,
            "unsupported Wleam ABI version {} (expected {ABI_VERSION})",
            self.version
        );
        ensure!(
            !self.imports.contains_key(HOST_MODULE),
            "{HOST_MODULE} is reserved for builtin host imports"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BitArray {
    pub bytes: Vec<u8>,
    pub bit_length: u32,
}

impl BitArray {
    pub fn new(bytes: Vec<u8>, bit_length: u32) -> Result<Self> {
        let value = Self { bytes, bit_length };
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            self.bytes.len() == bytes_for_bits(self.bit_length),
            "bit array byte length does not match its bit length"
        );
        let unused = (8 - self.bit_length % 8) % 8;
        if unused != 0 {
            ensure!(
                self.bytes
                    .last()
                    .is_none_or(|byte| byte & ((1 << unused) - 1) == 0),
                "bit array has nonzero padding bits"
            );
        }
        Ok(())
    }
}

/// An opaque, identity-preserving Rust object rooted by Wasm while it is used.
#[derive(Clone)]
pub struct HostReference(Arc<dyn Any + Send + Sync>);

impl HostReference {
    pub fn new(value: impl Any + Send + Sync) -> Self {
        Self(Arc::new(value))
    }

    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for HostReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HostReference(..)")
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    Nil,
    Bool(bool),
    Float(f64),
    Int(BigInt),
    String(String),
    BitArray(BitArray),
    Externref(HostReference),
    NullExternref,
}

pub type HostFunction = Arc<dyn Fn(&[Value]) -> Result<Value> + Send + Sync>;
pub type OutputSink = Arc<dyn Fn(&str) -> Result<()> + Send + Sync>;

#[derive(Clone, Default)]
pub struct HostImports(BTreeMap<(String, String), HostFunction>);

impl HostImports {
    pub fn define(
        &mut self,
        module: impl Into<String>,
        name: impl Into<String>,
        function: impl Fn(&[Value]) -> Result<Value> + Send + Sync + 'static,
    ) -> Result<()> {
        let key = (module.into(), name.into());
        ensure!(
            key.0 != HOST_MODULE,
            "{HOST_MODULE} is reserved for Wleam host functions"
        );
        ensure!(
            !self.0.contains_key(&key),
            "duplicate host function {}.{}",
            key.0,
            key.1
        );
        self.0.insert(key, Arc::new(function));
        Ok(())
    }
}

#[derive(Clone)]
pub struct HostOptions {
    /// Capture exact output in [`Instance::output`]. Disable for unbounded streams.
    pub capture_output: bool,
    pub stdout: Option<OutputSink>,
    pub stderr: Option<OutputSink>,
    pub imports: HostImports,
}

impl Default for HostOptions {
    fn default() -> Self {
        Self {
            capture_output: true,
            stdout: None,
            stderr: None,
            imports: HostImports::default(),
        }
    }
}

impl HostOptions {
    pub fn stdio() -> Self {
        Self {
            capture_output: false,
            stdout: Some(Arc::new(|text| {
                let mut output = io::stdout().lock();
                output.write_all(text.as_bytes())?;
                output.flush()?;
                Ok(())
            })),
            stderr: Some(Arc::new(|text| {
                let mut output = io::stderr().lock();
                output.write_all(text.as_bytes())?;
                output.flush()?;
                Ok(())
            })),
            imports: HostImports::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
}

struct State {
    options: HostOptions,
    output: Output,
    references: HashMap<usize, (Weak<dyn Any + Send + Sync>, i64)>,
    next_reference: i64,
}

/// A compiled program. Reuse it to create independent instances, for example one
/// per test, without recompiling machine code after a failed test traps.
pub struct Program {
    engine: Engine,
    module: Module,
    abi: Abi,
    embedded_abi: bool,
}

impl Program {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let embedded = Abi::read_wasm(bytes)?;
        let embedded_abi = embedded.is_some();
        let abi = embedded.unwrap_or_default();
        let mut config = Config::new();
        config.wasm_gc(true);
        config.wasm_function_references(true);
        config.wasm_tail_call(true);
        // Compile independent functions concurrently while preserving the
        // optimized machine code used for long-running guest programs.
        config.parallel_compilation(true);
        config.cranelift_opt_level(wasmtime::OptLevel::Speed);
        let engine = Engine::new(&config)?;
        let module = Module::new(&engine, bytes)
            .map_err(anyhow::Error::from)
            .context("compiling Wleam WebAssembly module")?;
        Ok(Self {
            engine,
            module,
            abi,
            embedded_abi,
        })
    }

    /// Supply descriptors when embedding a module that has no ABI custom section.
    pub fn with_abi(mut self, abi: Abi) -> Result<Self> {
        ensure!(
            !self.embedded_abi,
            "cannot override an existing Wleam ABI descriptor"
        );
        abi.validate()?;
        self.abi = abi;
        self.embedded_abi = true;
        Ok(self)
    }

    pub fn abi(&self) -> &Abi {
        &self.abi
    }

    pub fn instantiate(&self, options: HostOptions) -> Result<Instance> {
        let mut linker = Linker::new(&self.engine);
        define_builtins(&mut linker)?;
        for (module, functions) in &self.abi.imports {
            if module == HOST_MODULE {
                continue;
            }
            for (name, signature) in functions {
                let callback = options
                    .imports
                    .0
                    .get(&(module.clone(), name.clone()))
                    .with_context(|| format!("missing host import {module}.{name}"))?
                    .clone();
                define_import(
                    &mut linker,
                    &self.engine,
                    module,
                    name,
                    signature.clone(),
                    callback,
                )?;
            }
        }
        let mut store = Store::new(
            &self.engine,
            State {
                options,
                output: Output::default(),
                references: HashMap::new(),
                next_reference: 1,
            },
        );
        let instance = {
            let mut scope = RootScope::new(&mut store);
            let instance = linker
                .instantiate(&mut scope, &self.module)
                .map_err(anyhow::Error::from)
                .context("instantiating Wleam module (check its required host imports)")?;
            if let Some(init) = instance.get_func(&mut scope, "__wleam_init") {
                init.typed::<(), ()>(&scope)?
                    .call(&mut scope, ())
                    .map_err(anyhow::Error::from)
                    .context("initializing Wleam module")?;
            }
            instance
        };
        Ok(Instance {
            store,
            instance,
            abi: self.abi.clone(),
        })
    }
}

pub struct Instance {
    store: Store<State>,
    instance: WasmInstance,
    abi: Abi,
}

impl Instance {
    pub fn output(&self) -> &Output {
        &self.store.data().output
    }

    pub fn take_output(&mut self) -> Output {
        std::mem::take(&mut self.store.data_mut().output)
    }

    pub fn call_raw_empty(&mut self, name: &str) -> Result<()> {
        let function = self
            .instance
            .get_func(&mut self.store, name)
            .with_context(|| format!("missing Wasm export {name}"))?;
        let function = function
            .typed::<(), ()>(&self.store)
            .map_err(anyhow::Error::from)
            .with_context(|| {
                format!("export {name} must take no arguments and return no values")
            })?;
        let bridge = Bridge::from_instance(&self.instance, &mut self.store);
        let mut scope = RootScope::new(&mut self.store);
        let frame = bridge.begin_frame(&mut scope)?;
        let result = function
            .call(&mut scope, ())
            .map_err(anyhow::Error::from)
            .with_context(|| format!("executing {name}"));
        let cleanup = bridge.end_frame(&mut scope, frame);
        result.and(cleanup)
    }

    pub fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value> {
        let signature = self
            .abi
            .exports
            .get(name)
            .with_context(|| format!("export {name} has no Wleam ABI descriptor"))?
            .clone();
        ensure!(
            signature.params.len() == arguments.len(),
            "export {name} expects {} arguments, got {}",
            signature.params.len(),
            arguments.len()
        );
        let function = self
            .instance
            .get_func(&mut self.store, name)
            .with_context(|| format!("missing Wasm export {name}"))?;
        let bridge = Bridge::from_instance(&self.instance, &mut self.store);
        let mut scope = RootScope::new(&mut self.store);
        let frame = bridge.begin_frame(&mut scope)?;
        let mut allocations = Vec::new();
        let result = (|| {
            let mut params = Vec::new();
            for (kind, value) in signature.params.iter().zip(arguments) {
                lower(
                    &mut scope,
                    bridge,
                    *kind,
                    value,
                    &mut params,
                    &mut allocations,
                )?;
            }
            let mut results: Vec<Val> = signature
                .result
                .wasm_types()
                .iter()
                .map(|ty| Val::default_for_ty(ty).expect("all Wleam ABI types are defaultable"))
                .collect();
            function
                .call(&mut scope, &params, &mut results)
                .map_err(anyhow::Error::from)
                .with_context(|| format!("executing {name}"))?;
            let mut offset = 0;
            lift(
                &mut scope,
                bridge,
                signature.result,
                &results,
                &mut offset,
                Some(&mut allocations),
            )
        })();
        let cleanup = bridge.free_all(&mut scope, &allocations);
        let reset = bridge.end_frame(&mut scope, frame);
        match (result, cleanup.and(reset)) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error.context("freeing host ABI buffers")),
        }
    }

    /// Collect now; useful for embedders checking that opaque references remain
    /// alive exactly while retained by a guest object.
    pub fn collect_garbage(&mut self) -> Result<()> {
        self.store.gc(None)?;
        Ok(())
    }
}

fn define_builtins(linker: &mut Linker<State>) -> Result<()> {
    linker.func_wrap(
        HOST_MODULE,
        "reference_id",
        |mut caller: Caller<'_, State>, reference: Option<wasmtime::Rooted<ExternRef>>| {
            (|| -> Result<i64> {
                let Some(reference) = reference else {
                    return Ok(0);
                };
                let value = reference
                    .data(&caller)?
                    .context("host reference has no data")?
                    .downcast_ref::<HostReference>()
                    .context("externref was not created by this Wleam host")?
                    .clone();
                let address = Arc::as_ptr(&value.0) as *const () as usize;
                let state = caller.data_mut();
                if let Some((weak, id)) = state.references.get(&address)
                    && let Some(existing) = weak.upgrade()
                    && Arc::ptr_eq(&existing, &value.0)
                {
                    return Ok(*id);
                }
                let id = state.next_reference;
                state.next_reference = id
                    .checked_add(1)
                    .context("host reference identity space exhausted")?;
                if id % 256 == 0 {
                    state
                        .references
                        .retain(|_, (weak, _)| weak.strong_count() != 0);
                }
                state
                    .references
                    .insert(address, (Arc::downgrade(&value.0), id));
                Ok(id)
            })()
            .map_err(wasmtime::Error::from_anyhow)
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "write",
        |mut caller: Caller<'_, State>, channel: i32, ptr: i32, len: i32| {
            (|| -> Result<()> {
                let bridge = Bridge::from_caller(&mut caller);
                let text = String::from_utf8(bridge.read(&caller, ptr as u32, len as u32)?)
                    .context("host output is not valid UTF-8")?;
                let state = caller.data_mut();
                let sink = match channel {
                    1 => {
                        if state.options.capture_output {
                            state.output.stdout.push_str(&text);
                        }
                        state.options.stdout.clone()
                    }
                    2 => {
                        if state.options.capture_output {
                            state.output.stderr.push_str(&text);
                        }
                        state.options.stderr.clone()
                    }
                    _ => bail!("invalid output channel {channel}; expected 1 or 2"),
                };
                if let Some(sink) = sink {
                    sink(&text)?;
                }
                Ok(())
            })()
            .map_err(wasmtime::Error::from_anyhow)
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "random",
        |mut caller: Caller<'_, State>, ptr: i32, len: i32| {
            (|| -> Result<i32> {
                let bridge = Bridge::from_caller(&mut caller);
                let memory = bridge.memory.context("guest does not export memory")?;
                let bytes = memory.data_mut(&mut caller);
                let range = checked_range(ptr as u32, len as u32, bytes.len())?;
                getrandom::fill(&mut bytes[range])
                    .map_err(|error| anyhow!("OS randomness unavailable: {error}"))?;
                Ok(0)
            })()
            .map_err(wasmtime::Error::from_anyhow)
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "panic",
        |mut caller: Caller<'_, State>, ptr: i32, len: i32| {
            (|| -> Result<()> {
                let bridge = Bridge::from_caller(&mut caller);
                let message = String::from_utf8(bridge.read(&caller, ptr as u32, len as u32)?)
                    .context("panic message is not valid UTF-8")?;
                bail!("Gleam panic: {message}")
            })()
            .map_err(wasmtime::Error::from_anyhow)
        },
    )?;
    Ok(())
}

fn define_import(
    linker: &mut Linker<State>,
    engine: &Engine,
    module: &str,
    name: &str,
    signature: Signature,
    callback: HostFunction,
) -> Result<()> {
    let ty = FuncType::new(
        engine,
        signature.params.iter().flat_map(|kind| kind.wasm_types()),
        signature.result.wasm_types(),
    );
    linker.func_new(module, name, ty, move |mut caller, parameters, results| {
        (|| -> Result<()> {
            let bridge = Bridge::from_caller(&mut caller);
            let mut offset = 0;
            let mut arguments = Vec::new();
            for kind in &signature.params {
                arguments.push(lift(
                    &mut caller,
                    bridge,
                    *kind,
                    parameters,
                    &mut offset,
                    None,
                )?);
            }
            let value = callback(&arguments)?;
            let mut lowered = Vec::new();
            let mut allocations = Vec::new();
            if let Err(error) = lower(
                &mut caller,
                bridge,
                signature.result,
                &value,
                &mut lowered,
                &mut allocations,
            ) {
                let _ = bridge.free_all(&mut caller, &allocations);
                return Err(error);
            }
            if results.len() != lowered.len() {
                let _ = bridge.free_all(&mut caller, &allocations);
                bail!("host import result ABI mismatch");
            }
            results.clone_from_slice(&lowered);
            // The guest receives these buffers and frees them after copying them
            // into its GC heap. References are rooted by the outer call scope.
            Ok(())
        })()
        .map_err(wasmtime::Error::from_anyhow)
    })?;
    Ok(())
}

#[derive(Clone, Copy)]
struct Bridge {
    memory: Option<Memory>,
    alloc: Option<Func>,
    free: Option<Func>,
    mark: Option<Func>,
    reset: Option<Func>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct Allocation {
    ptr: u32,
    len: u32,
}

impl Bridge {
    fn from_caller(caller: &mut Caller<'_, State>) -> Self {
        Self {
            memory: caller
                .get_export("memory")
                .and_then(|value| value.into_memory()),
            alloc: caller
                .get_export("__wleam_alloc")
                .and_then(|value| value.into_func()),
            free: caller
                .get_export("__wleam_free")
                .and_then(|value| value.into_func()),
            mark: caller
                .get_export("__wleam_abi_mark")
                .and_then(|value| value.into_func()),
            reset: caller
                .get_export("__wleam_abi_reset")
                .and_then(|value| value.into_func()),
        }
    }

    fn from_instance(instance: &WasmInstance, store: &mut Store<State>) -> Self {
        Self {
            memory: instance.get_memory(&mut *store, "memory"),
            alloc: instance.get_func(&mut *store, "__wleam_alloc"),
            free: instance.get_func(&mut *store, "__wleam_free"),
            mark: instance.get_func(&mut *store, "__wleam_abi_mark"),
            reset: instance.get_func(&mut *store, "__wleam_abi_reset"),
        }
    }

    fn begin_frame(self, context: &mut impl AsContextMut<Data = State>) -> Result<Option<i32>> {
        match (self.mark, self.reset) {
            (None, None) => Ok(None),
            (Some(mark), Some(_)) => Ok(Some(
                mark.typed::<(), i32>(context.as_context())?
                    .call(context.as_context_mut(), ())?,
            )),
            _ => bail!("guest must export both __wleam_abi_mark and __wleam_abi_reset"),
        }
    }

    fn end_frame(
        self,
        context: &mut impl AsContextMut<Data = State>,
        frame: Option<i32>,
    ) -> Result<()> {
        if let Some(frame) = frame {
            self.reset
                .context("missing __wleam_abi_reset")?
                .typed::<i32, ()>(context.as_context())?
                .call(context.as_context_mut(), frame)?;
        }
        Ok(())
    }

    fn read(self, context: impl AsContext<Data = State>, ptr: u32, len: u32) -> Result<Vec<u8>> {
        let memory = self.memory.context("guest does not export memory")?;
        let bytes = memory.data(context.as_context());
        Ok(bytes[checked_range(ptr, len, bytes.len())?].to_vec())
    }

    fn write(
        self,
        context: &mut impl AsContextMut<Data = State>,
        bytes: &[u8],
        allocations: &mut Vec<Allocation>,
    ) -> Result<u32> {
        let len =
            u32::try_from(bytes.len()).context("host buffer exceeds the wasm32 address space")?;
        if len == 0 {
            return Ok(0);
        }
        let memory = self.memory.context("guest does not export memory")?;
        let alloc = self
            .alloc
            .context("guest does not export __wleam_alloc")?
            .typed::<i32, i32>(context.as_context())?;
        let ptr = alloc.call(context.as_context_mut(), len as i32)? as u32;
        ensure!(ptr != 0, "guest allocator failed to allocate {len} bytes");
        allocations.push(Allocation { ptr, len });
        memory
            .write(context.as_context_mut(), ptr as usize, bytes)
            .context("guest allocator returned an invalid buffer")?;
        Ok(ptr)
    }

    fn free_all(
        self,
        context: &mut impl AsContextMut<Data = State>,
        allocations: &[Allocation],
    ) -> Result<()> {
        if allocations.is_empty() {
            return Ok(());
        }
        let free = self
            .free
            .context("guest does not export __wleam_free")?
            .typed::<(i32, i32), ()>(context.as_context())?;
        let mut first_error = None;
        for (index, allocation) in allocations.iter().enumerate().rev() {
            if (allocation.ptr == 0 && allocation.len == 0)
                || allocations[..index]
                    .iter()
                    .any(|earlier| earlier.ptr == allocation.ptr)
            {
                continue;
            }
            if let Err(error) = free.call(
                context.as_context_mut(),
                (allocation.ptr as i32, allocation.len as i32),
            ) {
                first_error.get_or_insert(error);
            }
        }
        match first_error {
            Some(error) => Err(error.into()),
            None => Ok(()),
        }
    }
}

fn checked_range(ptr: u32, len: u32, memory_len: usize) -> Result<std::ops::Range<usize>> {
    let start = ptr as usize;
    let end = start
        .checked_add(len as usize)
        .context("host buffer address overflow")?;
    ensure!(
        end <= memory_len,
        "host buffer {start}..{end} is outside guest memory ({memory_len} bytes)"
    );
    Ok(start..end)
}

fn bytes_for_bits(bits: u32) -> usize {
    (u64::from(bits).div_ceil(8)) as usize
}

fn lower(
    context: &mut impl AsContextMut<Data = State>,
    bridge: Bridge,
    kind: AbiType,
    value: &Value,
    values: &mut Vec<Val>,
    allocations: &mut Vec<Allocation>,
) -> Result<()> {
    match (kind, value) {
        (AbiType::Nil, Value::Nil) => {}
        (AbiType::Bool, Value::Bool(value)) => values.push(Val::I32(i32::from(*value))),
        (AbiType::Float, Value::Float(value)) => {
            ensure!(value.is_finite(), "Wleam Float must be finite");
            values.push(Val::F64(value.to_bits()));
        }
        (AbiType::Int, Value::Int(value)) => {
            let (sign, limbs) = value.to_u32_digits();
            let bytes: Vec<u8> = limbs.iter().flat_map(|limb| limb.to_le_bytes()).collect();
            let ptr = bridge.write(context, &bytes, allocations)?;
            let sign = match sign {
                Sign::Minus => -1,
                Sign::NoSign => 0,
                Sign::Plus => 1,
            };
            values.extend([
                Val::I32(sign),
                Val::I32(ptr as i32),
                Val::I32(limbs.len() as i32),
            ]);
        }
        (AbiType::String, Value::String(value)) => {
            let ptr = bridge.write(context, value.as_bytes(), allocations)?;
            values.extend([Val::I32(ptr as i32), Val::I32(value.len() as i32)]);
        }
        (AbiType::BitArray, Value::BitArray(value)) => {
            value.validate()?;
            let ptr = bridge.write(context, &value.bytes, allocations)?;
            values.extend([Val::I32(ptr as i32), Val::I32(value.bit_length as i32)]);
        }
        (AbiType::Externref, Value::Externref(value)) => {
            values.push(Val::ExternRef(Some(ExternRef::new(
                context.as_context_mut(),
                value.clone(),
            )?)));
        }
        (AbiType::Externref, Value::NullExternref) => values.push(Val::ExternRef(None)),
        _ => bail!("value {value:?} does not match ABI type {kind:?}"),
    }
    Ok(())
}

fn lift(
    context: &mut impl AsContextMut<Data = State>,
    bridge: Bridge,
    kind: AbiType,
    values: &[Val],
    offset: &mut usize,
    mut owned: Option<&mut Vec<Allocation>>,
) -> Result<Value> {
    let take = |offset: &mut usize| -> Result<&Val> {
        let value = values.get(*offset).context("missing ABI value")?;
        *offset += 1;
        Ok(value)
    };
    let integer = |offset: &mut usize| -> Result<i32> {
        take(offset)?.i32().context("expected i32 ABI value")
    };
    let mut buffer = |ptr: u32, len: u32| -> Result<Vec<u8>> {
        let data = bridge.read(context.as_context(), ptr, len)?;
        if let Some(allocations) = owned.as_deref_mut()
            && (ptr != 0 || len > 0)
        {
            allocations.push(Allocation { ptr, len });
        }
        Ok(data)
    };
    Ok(match kind {
        AbiType::Nil => Value::Nil,
        AbiType::Bool => match integer(offset)? {
            0 => Value::Bool(false),
            1 => Value::Bool(true),
            invalid => bail!("invalid boolean ABI value {invalid}"),
        },
        AbiType::Float => {
            let value = take(offset)?.f64().context("expected f64 ABI value")?;
            ensure!(value.is_finite(), "Wleam Float must be finite");
            Value::Float(value)
        }
        AbiType::Int => {
            let sign = integer(offset)?;
            let ptr = integer(offset)? as u32;
            let count = integer(offset)? as u32;
            let len = count
                .checked_mul(4)
                .context("integer limb count exceeds wasm32 memory")?;
            let bytes = buffer(ptr, len)?;
            let limbs: Vec<u32> = bytes
                .as_chunks::<4>()
                .0
                .iter()
                .copied()
                .map(u32::from_le_bytes)
                .collect();
            let sign = match sign {
                -1 => Sign::Minus,
                0 => Sign::NoSign,
                1 => Sign::Plus,
                _ => bail!("invalid integer ABI sign {sign}"),
            };
            ensure!(
                (sign == Sign::NoSign) == limbs.is_empty(),
                "noncanonical zero integer ABI representation"
            );
            ensure!(
                limbs.last().is_none_or(|limb| *limb != 0),
                "integer ABI has a leading zero limb"
            );
            Value::Int(BigInt::from_slice(sign, &limbs))
        }
        AbiType::String => {
            let ptr = integer(offset)? as u32;
            let len = integer(offset)? as u32;
            Value::String(String::from_utf8(buffer(ptr, len)?).context("ABI string is not UTF-8")?)
        }
        AbiType::BitArray => {
            let ptr = integer(offset)? as u32;
            let bit_length = integer(offset)? as u32;
            Value::BitArray(BitArray::new(
                buffer(ptr, bytes_for_bits(bit_length) as u32)?,
                bit_length,
            )?)
        }
        AbiType::Externref => {
            let Val::ExternRef(reference) = take(offset)? else {
                bail!("expected externref ABI value")
            };
            let Some(reference) = reference else {
                return Ok(Value::NullExternref);
            };
            let value = reference
                .data(context.as_context())?
                .context("host reference has no data")?
                .downcast_ref::<HostReference>()
                .context("externref was not created by this Wleam host")?
                .clone();
            Value::Externref(value)
        }
    })
}
