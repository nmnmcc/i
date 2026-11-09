//! Wasmtime embedding of the i binary interface, version 2 (spec Appendix B).
//!
//! A [`Program`] is a compiled module with its `ilang.abi` metadata. Every
//! [`Instance`] runs `ilang_start` once before anything else. Calls give an
//! [`Outcome`]: a value, an error of the function (`? E`), or a [`Stop`] with
//! the stop message of spec §11.7. After a stop, or any other trap, an
//! instance refuses further calls (B.5); make a new one.
//!
//! Buffers follow B.3: the host makes the buffers of export parameters with
//! `ilang_alloc` and frees them after the call, and it frees the result
//! buffers after copying them. For imports, the host makes the result buffers.
//!
//! # Stack
//!
//! Ordinary recursion that is not in tail position uses the Wasm stack. Wasm
//! code may use up to [`WASM_STACK`] bytes of it (Wasmtime's
//! `max_wasm_stack`, 32 MiB). A frame of a small i function takes about 60
//! bytes, so this is about 500,000 frames of such functions, and fewer of
//! bigger ones; more traps as "call stack exhausted", which is a stop with
//! no message (spec §11.7). Every call into the module runs on a
//! thread of its own with a native stack of [`THREAD_STACK`] bytes, so that
//! the Wasm stack always fits in it, whatever thread calls the host.

pub mod services;
#[cfg(test)]
mod tests;

use std::{
    any::Any,
    collections::{BTreeMap, HashMap},
    fmt,
    future::Future,
    pin::Pin,
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

pub const ABI_VERSION: u32 = 2;
pub const HOST_MODULE: &str = "ilang:host/v1";
/// The stop reason for a value from the host that is not correct (B.2).
pub const HOST_VALUE_REASON: &str = "host: value is not correct";

/// Types that the host can take (spec §10.3), in the canonical order of B.2
/// for the members of an error type.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AbiType {
    Nil,
    Bool,
    Int,
    Float,
    String,
    BitArray,
    Externref,
}

impl AbiType {
    fn wasm_types(self) -> &'static [ValType] {
        match self {
            Self::Nil => &[],
            Self::Bool => &[ValType::I32],
            Self::Float => &[ValType::F64],
            Self::Int => &[ValType::I32, ValType::I32, ValType::I32],
            Self::String | Self::BitArray => &[ValType::I32, ValType::I32],
            Self::Externref => &[ValType::EXTERNREF],
        }
    }
}

/// A function at the boundary: `(P1, …, Pn) -> A ? E`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Signature {
    pub params: Vec<AbiType>,
    pub result: AbiType,
    /// The members of `E` in canonical order; empty when `E` is `Never`.
    #[serde(default)]
    pub errors: Vec<AbiType>,
    #[serde(default, rename = "async")]
    pub is_async: bool,
}

impl Signature {
    pub fn new(params: impl Into<Vec<AbiType>>, result: AbiType) -> Self {
        Self {
            params: params.into(),
            result,
            errors: Vec::new(),
            is_async: false,
        }
    }

    pub fn with_errors(mut self, errors: impl Into<Vec<AbiType>>) -> Self {
        self.errors = errors.into();
        self
    }

    pub fn asynchronous(mut self) -> Self {
        self.is_async = true;
        self
    }

    fn param_types(&self) -> Vec<ValType> {
        self.params
            .iter()
            .flat_map(|kind| kind.wasm_types().iter().cloned())
            .collect()
    }

    /// The results of B.2: the tag when there are errors, the value, then
    /// every error member.
    fn result_types(&self) -> Vec<ValType> {
        let mut types = Vec::new();
        if !self.errors.is_empty() {
            types.push(ValType::I32);
        }
        types.extend(self.result.wasm_types().iter().cloned());
        for error in &self.errors {
            types.extend(error.wasm_types().iter().cloned());
        }
        types
    }

    fn validate(&self, location: &str) -> Result<()> {
        ensure!(
            self.errors.windows(2).all(|pair| pair[0] < pair[1]),
            "{location}: errors must be in canonical order without repeats"
        );
        ensure!(
            !self.errors.contains(&AbiType::Externref),
            "{location}: an error cannot be an externref"
        );
        Ok(())
    }
}

/// The program entry or a test (B.5).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EntryPoint {
    pub name: String,
    #[serde(default, rename = "async")]
    pub is_async: bool,
}

impl EntryPoint {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_async: false,
        }
    }
}

/// The `ilang.abi` metadata (B.6).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Abi {
    pub version: u32,
    pub entry: Option<EntryPoint>,
    pub tests: Vec<EntryPoint>,
    pub exports: BTreeMap<String, Signature>,
    pub services: BTreeMap<String, BTreeMap<String, Signature>>,
    pub externals: BTreeMap<String, BTreeMap<String, Signature>>,
}

impl Default for Abi {
    fn default() -> Self {
        Self {
            version: ABI_VERSION,
            entry: None,
            tests: Vec::new(),
            exports: BTreeMap::new(),
            services: BTreeMap::new(),
            externals: BTreeMap::new(),
        }
    }
}

impl Abi {
    /// Read the metadata of a module; a module without it has none.
    pub fn from_wasm(bytes: &[u8]) -> Result<Option<Self>> {
        let mut found = None;
        for payload in wasmparser::Parser::new(0).parse_all(bytes) {
            if let wasmparser::Payload::CustomSection(section) = payload?
                && section.name() == "ilang.abi"
            {
                ensure!(found.is_none(), "duplicate ilang.abi custom section");
                found = Some(Self::from_json(section.data())?);
            }
        }
        Ok(found)
    }

    /// Parse and check metadata in the JSON form of B.6. Every field must be
    /// there; `entry` is `null` when the program has no entry.
    pub fn from_json(json: &[u8]) -> Result<Self> {
        let value: serde_json::Value =
            serde_json::from_slice(json).context("invalid ilang.abi JSON")?;
        ensure!(
            value.get("entry").is_some(),
            "the ilang.abi metadata needs the field entry"
        );
        let abi: Self = serde_json::from_value(value).context("invalid ilang.abi metadata")?;
        abi.validate()?;
        Ok(abi)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == ABI_VERSION,
            "unsupported i ABI version {} (expected {ABI_VERSION})",
            self.version
        );
        for (name, signature) in &self.exports {
            signature.validate(&format!("export {name}"))?;
            ensure!(
                !is_reserved_export(name),
                "export {name} uses a name of the binary interface"
            );
        }
        for entry in self.entry.iter().chain(&self.tests) {
            ensure!(
                !self.exports.contains_key(&entry.name),
                "{} is both an entry point and a host export",
                entry.name
            );
            ensure!(
                !is_reserved_export(&entry.name),
                "entry point {} uses a name of the binary interface",
                entry.name
            );
        }
        for (kind, modules) in [("service", &self.services), ("external", &self.externals)] {
            for (module, functions) in modules {
                ensure!(
                    module != HOST_MODULE,
                    "{HOST_MODULE} is reserved for the runtime imports"
                );
                for (name, signature) in functions {
                    signature.validate(&format!("{kind} {module} {name}"))?;
                }
            }
        }
        for (module, functions) in &self.externals {
            for (name, signature) in functions {
                ensure!(
                    signature.errors.is_empty() && !signature.is_async,
                    "external {module} {name} cannot have errors or be async"
                );
                ensure!(
                    !self
                        .services
                        .get(module)
                        .is_some_and(|service| service.contains_key(name)),
                    "{module} {name} is both a service function and an external"
                );
            }
        }
        Ok(())
    }
}

fn is_reserved_export(name: &str) -> bool {
    matches!(
        name,
        "memory" | "ilang_alloc" | "ilang_free" | "ilang_start"
    ) || name.starts_with("ilang_resume:")
        || name.starts_with("ilang_result:")
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BitArray {
    pub bytes: Vec<u8>,
    pub bit_length: u32,
}

impl BitArray {
    pub fn new(bytes: Vec<u8>, bit_length: u32) -> Result<Self> {
        let value = Self { bytes, bit_length };
        ensure!(
            value.is_correct(),
            "a bit array has ceil(bits / 8) bytes and its bits after the end are 0"
        );
        Ok(value)
    }

    fn is_correct(&self) -> bool {
        let unused = (8 - self.bit_length % 8) % 8;
        self.bytes.len() == bytes_for_bits(self.bit_length)
            && (unused == 0
                || self
                    .bytes
                    .last()
                    .is_none_or(|byte| byte & ((1 << unused) - 1) == 0))
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

/// A value of a type that the host can take.
#[derive(Clone, Debug)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(BigInt),
    Float(f64),
    String(String),
    BitArray(BitArray),
    Externref(HostReference),
    NullExternref,
}

impl Value {
    pub fn abi_type(&self) -> AbiType {
        match self {
            Self::Nil => AbiType::Nil,
            Self::Bool(_) => AbiType::Bool,
            Self::Int(_) => AbiType::Int,
            Self::Float(_) => AbiType::Float,
            Self::String(_) => AbiType::String,
            Self::BitArray(_) => AbiType::BitArray,
            Self::Externref(_) | Self::NullExternref => AbiType::Externref,
        }
    }

    /// Is this a correct value of `kind` (B.2)?
    fn is_correct(&self, kind: AbiType) -> bool {
        self.abi_type() == kind
            && match self {
                Self::Float(value) => value.is_finite(),
                Self::BitArray(value) => value.is_correct(),
                _ => true,
            }
    }
}

/// An error that a function gave (`? E`).
#[derive(Clone, Debug)]
pub struct GuestError(pub Value);

/// The program stopped (spec §11.7). The instance cannot be used again.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stop {
    /// The stop message from the `panic` import, or `None` when the module
    /// trapped without one, for example with no more memory or stack.
    pub message: Option<String>,
    /// What the engine reported.
    pub trap: String,
}

impl fmt::Display for Stop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.message {
            Some(message) => f.write_str(message),
            None => write!(f, "trap: {}", self.trap),
        }
    }
}

impl std::error::Error for Stop {}

/// What a call gave.
#[derive(Clone, Debug)]
pub enum Outcome {
    Ok(Value),
    Err(GuestError),
    Stop(Stop),
}

impl Outcome {
    /// The value, or an error for `Err` and `Stop`.
    pub fn into_value(self) -> Result<Value> {
        match self {
            Self::Ok(value) => Ok(value),
            Self::Err(GuestError(value)) => bail!("the function gave the error {value:?}"),
            Self::Stop(stop) => Err(stop.into()),
        }
    }
}

/// The result of a test (spec §10.2).
#[derive(Debug)]
pub enum TestResult {
    Passed,
    /// The test stopped: it failed.
    Stopped(Stop),
    /// The host could not run it, for example because a host function failed.
    Failed(anyhow::Error),
}

#[derive(Debug)]
pub struct TestReport {
    pub name: String,
    pub result: TestResult,
}

impl TestReport {
    pub fn passed(&self) -> bool {
        matches!(self.result, TestResult::Passed)
    }
}

/// A host service function: the value or an error value of its `E`. An
/// `Err` of the outer result is a failure of the host, which traps.
pub type ServiceFunction =
    Arc<dyn Fn(&[Value]) -> Result<std::result::Result<Value, Value>> + Send + Sync>;
/// The work of an async host service function (B.4).
pub type ServiceFuture =
    Pin<Box<dyn Future<Output = Result<std::result::Result<Value, Value>>> + Send>>;
pub type AsyncServiceFunction = Arc<dyn Fn(&[Value]) -> ServiceFuture + Send + Sync>;
/// An external function (spec §10.1).
pub type ExternalFunction = Arc<dyn Fn(&[Value]) -> Result<Value> + Send + Sync>;

#[derive(Clone)]
enum HostFunction {
    Service(ServiceFunction),
    AsyncService(AsyncServiceFunction),
    External(ExternalFunction),
}

/// The functions that the host gives, by import module and name.
#[derive(Clone, Default)]
pub struct HostImports(BTreeMap<(String, String), HostFunction>);

impl HostImports {
    fn insert(&mut self, module: String, name: String, function: HostFunction) -> Result<()> {
        ensure!(
            module != HOST_MODULE,
            "{HOST_MODULE} is reserved for the runtime imports"
        );
        let key = (module, name);
        ensure!(
            !self.0.contains_key(&key),
            "duplicate host function {} {}",
            key.0,
            key.1
        );
        self.0.insert(key, function);
        Ok(())
    }

    /// A function of a host service: the import `label` of module `namespace`.
    pub fn service(
        &mut self,
        namespace: impl Into<String>,
        label: impl Into<String>,
        function: impl Fn(&[Value]) -> Result<std::result::Result<Value, Value>> + Send + Sync + 'static,
    ) -> Result<()> {
        self.insert(
            namespace.into(),
            label.into(),
            HostFunction::Service(Arc::new(function)),
        )
    }

    /// A function of an async host service (`@host("…", "async")`). It starts
    /// the work and gives a future of its value; see [`Instance::call_async`].
    pub fn async_service<F>(
        &mut self,
        namespace: impl Into<String>,
        label: impl Into<String>,
        function: impl Fn(&[Value]) -> F + Send + Sync + 'static,
    ) -> Result<()>
    where
        F: Future<Output = Result<std::result::Result<Value, Value>>> + Send + 'static,
    {
        self.insert(
            namespace.into(),
            label.into(),
            HostFunction::AsyncService(Arc::new(move |arguments| Box::pin(function(arguments)))),
        )
    }

    /// Adds every function of `other`. A function that is in both is an
    /// error.
    pub fn merge(&mut self, other: HostImports) -> Result<()> {
        for ((module, name), function) in other.0 {
            self.insert(module, name, function)?;
        }
        Ok(())
    }

    /// An external function (`@external("module", "function")`).
    pub fn external(
        &mut self,
        module: impl Into<String>,
        name: impl Into<String>,
        function: impl Fn(&[Value]) -> Result<Value> + Send + Sync + 'static,
    ) -> Result<()> {
        self.insert(
            module.into(),
            name.into(),
            HostFunction::External(Arc::new(function)),
        )
    }
}

#[derive(Clone, Default)]
pub struct HostOptions {
    pub imports: HostImports,
    /// Give the answers of host functions to the module when they have the
    /// right types, without checking that they are correct (a `Float` that
    /// is not finite, a `BitArray` with bits after the end): the module
    /// checks them and stops, with the positions of the call (B.2, §11.7).
    /// When off, the host stops the program itself, with no positions.
    pub unchecked_replies: bool,
}

impl HostOptions {
    pub fn new(imports: HostImports) -> Self {
        Self {
            imports,
            unchecked_replies: false,
        }
    }
}

/// An async host function that has been started and not answered yet.
struct Pending {
    namespace: String,
    label: String,
    signature: Signature,
    work: ServiceFuture,
}

struct State {
    references: HashMap<usize, (Weak<dyn Any + Send + Sync>, i64)>,
    next_reference: i64,
    /// The message of the `panic` import, or of a host value that is not correct.
    stop_message: Option<String>,
    /// A failure of the host or an incorrect guest value inside an import.
    failure: Option<anyhow::Error>,
    pending: Vec<Pending>,
}

/// A compiled program. Reuse it to make independent instances, for example
/// one per test, without compiling again.
pub struct Program {
    engine: Engine,
    module: Module,
    abi: Abi,
    embedded_abi: bool,
}

impl Program {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let embedded = Abi::from_wasm(bytes)?;
        let embedded_abi = embedded.is_some();
        let abi = embedded.unwrap_or_default();
        let mut config = Config::new();
        config.wasm_gc(true);
        config.wasm_function_references(true);
        config.wasm_tail_call(true);
        config.wasm_exceptions(true);
        config.max_wasm_stack(WASM_STACK);
        // Only checked against `max_wasm_stack`: the host does not use
        // Wasmtime's async support.
        config.async_stack_size(THREAD_STACK);
        // Compile independent functions concurrently while preserving the
        // optimized machine code used for long-running guest programs.
        config.parallel_compilation(true);
        config.cranelift_opt_level(wasmtime::OptLevel::Speed);
        let engine = Engine::new(&config)?;
        let module = Module::new(&engine, bytes)
            .map_err(anyhow::Error::from)
            .context("compiling the i WebAssembly module")?;
        Ok(Self {
            engine,
            module,
            abi,
            embedded_abi,
        })
    }

    /// Give the metadata of a module that has no `ilang.abi` section.
    pub fn with_abi(mut self, abi: Abi) -> Result<Self> {
        ensure!(
            !self.embedded_abi,
            "cannot replace the ilang.abi metadata of a module"
        );
        abi.validate()?;
        self.abi = abi;
        self.embedded_abi = true;
        Ok(self)
    }

    pub fn abi(&self) -> &Abi {
        &self.abi
    }

    /// Make an instance and call `ilang_start` (B.1). When the program stops
    /// during `ilang_start`, the error is a [`Stop`].
    pub fn instantiate(&self, options: HostOptions) -> Result<Instance> {
        let mut linker = Linker::new(&self.engine);
        define_builtins(&mut linker)?;
        for (kind, modules) in [(true, &self.abi.services), (false, &self.abi.externals)] {
            for (module, functions) in modules {
                for (name, signature) in functions {
                    let function = options
                        .imports
                        .0
                        .get(&(module.clone(), name.clone()))
                        .with_context(|| format!("missing host function {module} {name}"))?
                        .clone();
                    match (&function, kind, signature.is_async) {
                        (HostFunction::Service(_), true, false)
                        | (HostFunction::AsyncService(_), true, true)
                        | (HostFunction::External(_), false, _) => {}
                        _ => bail!("host function {module} {name} has the wrong kind"),
                    }
                    define_import(
                        &mut linker,
                        &self.engine,
                        module,
                        name,
                        signature,
                        function,
                        !options.unchecked_replies,
                    )?;
                }
            }
        }
        let mut store = Store::new(
            &self.engine,
            State {
                references: HashMap::new(),
                next_reference: 1,
                stop_message: None,
                failure: None,
                pending: Vec::new(),
            },
        );
        let instance = {
            let mut scope = RootScope::new(&mut store);
            linker
                .instantiate(&mut scope, &self.module)
                .map_err(anyhow::Error::from)
                .context("instantiating the i module (check the host functions)")?
        };
        let mut instance = Instance {
            store,
            instance,
            abi: self.abi.clone(),
            unusable: None,
            checked: !options.unchecked_replies,
        };
        let start = instance
            .instance
            .get_func(&mut instance.store, "ilang_start")
            .context("the module does not export ilang_start")?;
        match instance.invoke(start, &[], &mut [])? {
            None => Ok(instance),
            Some(stop) => {
                Err(anyhow::Error::new(stop).context("the program stopped in ilang_start"))
            }
        }
    }

    /// Run the program entry in a new instance.
    pub fn run_entry(&self, options: HostOptions) -> Result<Outcome> {
        let entry = self.abi.entry.clone().context("the program has no entry")?;
        let mut instance = match self.instantiate(options) {
            Ok(instance) => instance,
            Err(error) => return stop_of(error).map(Outcome::Stop),
        };
        if entry.is_async {
            block_on(instance.run_async(&entry.name))
        } else {
            instance.run(&entry.name)
        }
    }

    /// Run every test of the metadata, each in a new instance (spec §10.2).
    pub fn run_tests(&self, options: &HostOptions) -> Vec<TestReport> {
        self.abi
            .tests
            .iter()
            .map(|test| TestReport {
                name: test.name.clone(),
                result: self.run_test(test, options),
            })
            .collect()
    }

    fn run_test(&self, test: &EntryPoint, options: &HostOptions) -> TestResult {
        let mut instance = match self.instantiate(options.clone()) {
            Ok(instance) => instance,
            Err(error) => {
                return match error.downcast::<Stop>() {
                    Ok(stop) => TestResult::Stopped(stop),
                    Err(error) => TestResult::Failed(error),
                };
            }
        };
        let outcome = if test.is_async {
            block_on(instance.run_async(&test.name))
        } else {
            instance.run(&test.name)
        };
        match outcome {
            Ok(Outcome::Ok(_)) => TestResult::Passed,
            Ok(Outcome::Stop(stop)) => TestResult::Stopped(stop),
            Ok(Outcome::Err(error)) => TestResult::Failed(anyhow!("a test gave {:?}", error.0)),
            Err(error) => TestResult::Failed(error),
        }
    }
}

fn stop_of(error: anyhow::Error) -> Result<Stop> {
    error.downcast::<Stop>()
}

/// Drive a future whose waiting is done by the host functions themselves.
/// Real hosts call [`Instance::call_async`] from their own executor.
pub fn block_on<F: Future>(future: F) -> F::Output {
    use std::task::{Context as TaskContext, Poll, Wake, Waker};
    struct Thread(std::thread::Thread);
    impl Wake for Thread {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(Thread(std::thread::current())));
    let mut context = TaskContext::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park(),
        }
    }
}

pub struct Instance {
    store: Store<State>,
    instance: WasmInstance,
    abi: Abi,
    unusable: Option<String>,
    /// Whether the answers of host functions are checked by the host.
    checked: bool,
}

impl Instance {
    pub fn abi(&self) -> &Abi {
        &self.abi
    }

    /// Why the instance cannot be used any more, after a stop or a trap.
    pub fn unusable(&self) -> Option<&str> {
        self.unusable.as_deref()
    }

    fn usable(&self) -> Result<()> {
        if let Some(reason) = &self.unusable {
            bail!("the instance cannot be used after it stopped ({reason}); make a new instance");
        }
        Ok(())
    }

    fn export(&mut self, name: &str) -> Result<Func> {
        self.instance
            .get_func(&mut self.store, name)
            .with_context(|| format!("the module does not export {name}"))
    }

    /// Call a Wasm function with no references among its parameters.
    fn invoke(
        &mut self,
        function: Func,
        params: &[Val],
        results: &mut [Val],
    ) -> Result<Option<Stop>> {
        let invoked = {
            let mut scope = RootScope::new(&mut self.store);
            invoke_in(&mut scope, function, params, results)
        };
        self.settle(invoked)
    }

    /// A trap makes the instance unusable. It gives `Some(stop)`, or the
    /// failure of a host function as an error.
    fn settle(&mut self, invoked: Invoked) -> Result<Option<Stop>> {
        match invoked {
            Invoked::Done => Ok(None),
            Invoked::Stopped(stop) => {
                self.unusable = Some(stop.to_string());
                Ok(Some(stop))
            }
            Invoked::Failed(error) => {
                self.unusable = Some(format!("{error:#}"));
                Err(error)
            }
        }
    }

    fn host_value_stop(&mut self) -> Outcome {
        Outcome::Stop(self.host_value_stop_inner())
    }

    fn host_value_stop_inner(&mut self) -> Stop {
        let stop = Stop {
            message: Some(HOST_VALUE_REASON.into()),
            trap: "the host gave a value that is not correct".into(),
        };
        self.unusable = Some(stop.to_string());
        stop
    }

    /// Run the program entry or a test by its export name (B.5).
    pub fn run(&mut self, name: &str) -> Result<Outcome> {
        self.usable()?;
        let function = self.export(name)?;
        match self.invoke(function, &[], &mut [])? {
            None => Ok(Outcome::Ok(Value::Nil)),
            Some(stop) => Ok(Outcome::Stop(stop)),
        }
    }

    /// Run the program entry of the metadata.
    pub fn run_entry(&mut self) -> Result<Outcome> {
        let entry = self.abi.entry.clone().context("the program has no entry")?;
        if entry.is_async {
            block_on(self.run_async(&entry.name))
        } else {
            self.run(&entry.name)
        }
    }

    /// Call a host export (B.2). The arguments must be correct values of the
    /// parameter types; otherwise the program stops (`host: value is not
    /// correct`) without running.
    pub fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Outcome> {
        self.usable()?;
        let signature = self.signature(name, arguments)?;
        ensure!(
            !signature.is_async,
            "export {name} is async; use call_async"
        );
        if !correct_arguments(&signature, arguments) {
            return Ok(self.host_value_stop());
        }
        let function = self.export(name)?;
        let bridge = Bridge::from_instance(&self.instance, &mut self.store);
        let (invoked, outcome) = {
            let mut scope = RootScope::new(&mut self.store);
            let mut allocations = Vec::new();
            let mut results = defaults(&signature.result_types());
            let invoked = match lower_all(
                &mut scope,
                bridge,
                &signature.params,
                arguments,
                &mut allocations,
            ) {
                Ok(params) => invoke_in(&mut scope, function, &params, &mut results),
                Err(invoked) => invoked,
            };
            let outcome = match invoked {
                Invoked::Done => Some(finish(
                    &mut scope,
                    bridge,
                    &signature,
                    &results,
                    allocations,
                )),
                _ => None,
            };
            (invoked, outcome)
        };
        if let Some(stop) = self.settle(invoked)? {
            return Ok(Outcome::Stop(stop));
        }
        self.after_finish(outcome.expect("a finished call has an outcome"))
    }

    fn after_finish(&mut self, outcome: Result<Outcome>) -> Result<Outcome> {
        outcome.inspect_err(|error| self.unusable = Some(format!("{error:#}")))
    }

    /// Call an async host export (B.5). Each async host function that the
    /// program waits for is answered by awaiting its future, then calling
    /// `ilang_resume:namespace:label`; the value comes from `ilang_result:name`.
    pub async fn call_async(&mut self, name: &str, arguments: &[Value]) -> Result<Outcome> {
        self.usable()?;
        let signature = self.signature(name, arguments)?;
        ensure!(signature.is_async, "export {name} is not async; use call");
        if !correct_arguments(&signature, arguments) {
            return Ok(self.host_value_stop());
        }
        let function = self.export(name)?;
        let result = self.export(&format!("ilang_result:{name}"))?;
        let bridge = Bridge::from_instance(&self.instance, &mut self.store);
        let mut allocations = Vec::new();
        let mut status = [Val::I32(0)];
        let invoked = {
            let mut scope = RootScope::new(&mut self.store);
            match lower_all(
                &mut scope,
                bridge,
                &signature.params,
                arguments,
                &mut allocations,
            ) {
                Ok(params) => invoke_in(&mut scope, function, &params, &mut status),
                Err(invoked) => invoked,
            }
        };
        if let Some(stop) = self.settle(invoked)? {
            return Ok(Outcome::Stop(stop));
        }
        if let Some(stop) = self.wait(name, status).await? {
            return Ok(Outcome::Stop(stop));
        }
        let mut results = defaults(&signature.result_types());
        let (invoked, outcome) = {
            let mut scope = RootScope::new(&mut self.store);
            let invoked = invoke_in(&mut scope, result, &[], &mut results);
            let outcome = match invoked {
                Invoked::Done => Some(finish(
                    &mut scope,
                    bridge,
                    &signature,
                    &results,
                    allocations,
                )),
                _ => None,
            };
            (invoked, outcome)
        };
        if let Some(stop) = self.settle(invoked)? {
            return Ok(Outcome::Stop(stop));
        }
        self.after_finish(outcome.expect("a finished call has an outcome"))
    }

    /// Run an async program entry or test by its export name.
    pub async fn run_async(&mut self, name: &str) -> Result<Outcome> {
        self.usable()?;
        let function = self.export(name)?;
        let result = self.export(&format!("ilang_result:{name}"))?;
        let mut status = [Val::I32(0)];
        if let Some(stop) = self.invoke(function, &[], &mut status)? {
            return Ok(Outcome::Stop(stop));
        }
        if let Some(stop) = self.wait(name, status).await? {
            return Ok(Outcome::Stop(stop));
        }
        match self.invoke(result, &[], &mut [])? {
            None => Ok(Outcome::Ok(Value::Nil)),
            Some(stop) => Ok(Outcome::Stop(stop)),
        }
    }

    /// Answer the async host functions that the module waits for until the
    /// status is 0 (B.5).
    async fn wait(&mut self, name: &str, mut status: [Val; 1]) -> Result<Option<Stop>> {
        loop {
            match status[0].i32() {
                Some(0) => return Ok(None),
                Some(1) => {}
                _ => {
                    self.unusable = Some("the module gave a wrong status".into());
                    bail!("export {name} gave a status that is not 0 or 1");
                }
            }
            let pending = std::mem::take(&mut self.store.data_mut().pending);
            let count = pending.len();
            let Ok([pending]) = <[Pending; 1]>::try_from(pending) else {
                self.unusable =
                    Some("the module waits for a wrong number of host functions".into());
                bail!("the module waits for {count} host functions instead of one");
            };
            let reply = match pending.work.await {
                Ok(reply) => reply,
                Err(failure) => {
                    self.unusable = Some(format!("{failure:#}"));
                    return Err(failure);
                }
            };
            let resume = self.export(&format!(
                "ilang_resume:{}:{}",
                pending.namespace, pending.label
            ))?;
            let bridge = Bridge::from_instance(&self.instance, &mut self.store);
            let checked = self.checked;
            let invoked = {
                let mut scope = RootScope::new(&mut self.store);
                let mut values = Vec::new();
                match lower_reply(
                    &mut scope,
                    bridge,
                    &pending.signature,
                    reply,
                    &mut values,
                    checked,
                ) {
                    Ok(true) => Some(invoke_in(&mut scope, resume, &values, &mut status)),
                    Ok(false) => None,
                    Err(error) => Some(Invoked::Failed(error)),
                }
            };
            let Some(invoked) = invoked else {
                return Ok(Some(self.host_value_stop_inner()));
            };
            if let Some(stop) = self.settle(invoked)? {
                return Ok(Some(stop));
            }
        }
    }

    fn signature(&self, name: &str, arguments: &[Value]) -> Result<Signature> {
        let signature = self
            .abi
            .exports
            .get(name)
            .with_context(|| format!("{name} is not a host export of the metadata"))?
            .clone();
        ensure!(
            signature.params.len() == arguments.len(),
            "export {name} takes {} arguments, not {}",
            signature.params.len(),
            arguments.len()
        );
        Ok(signature)
    }

    /// Collect now; useful for embedders checking that opaque references remain
    /// alive exactly while retained by a guest object.
    pub fn collect_garbage(&mut self) -> Result<()> {
        self.store.gc(None)?;
        Ok(())
    }
}

/// How a call of a Wasm function ended.
enum Invoked {
    Done,
    Stopped(Stop),
    Failed(anyhow::Error),
}

/// The most stack, in bytes, that Wasm code may use (see the module
/// documentation).
pub const WASM_STACK: usize = 32 << 20;

/// The native stack, in bytes, of the thread that runs Wasm code: the Wasm
/// stack, and room for the host functions it calls.
pub const THREAD_STACK: usize = WASM_STACK + (16 << 20);

/// Call a Wasm function inside the root scope of its arguments, on a thread
/// with a stack of [`THREAD_STACK`] bytes.
fn invoke_in(
    context: &mut (impl AsContextMut<Data = State> + Send),
    function: Func,
    params: &[Val],
    results: &mut [Val],
) -> Invoked {
    let called = std::thread::scope(|scope| {
        let thread = std::thread::Builder::new()
            .name("ilang-wasm".into())
            .stack_size(THREAD_STACK)
            .spawn_scoped(scope, || function.call(&mut *context, params, results))
            .expect("a thread for Wasm code");
        match thread.join() {
            Ok(called) => called,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    });
    let Err(error) = called else {
        return Invoked::Done;
    };
    let error = anyhow::Error::from(error);
    let mut context = context.as_context_mut();
    let state = context.data_mut();
    state.pending.clear();
    if let Some(failure) = state.failure.take() {
        return Invoked::Failed(failure);
    }
    Invoked::Stopped(Stop {
        message: state.stop_message.take(),
        trap: error.root_cause().to_string(),
    })
}

/// Lower the arguments into new buffers. A trap of `ilang_alloc` ends the call.
fn lower_all(
    context: &mut impl AsContextMut<Data = State>,
    bridge: Bridge,
    kinds: &[AbiType],
    arguments: &[Value],
    allocations: &mut Vec<Allocation>,
) -> std::result::Result<Vec<Val>, Invoked> {
    let mut params = Vec::new();
    for (kind, value) in kinds.iter().zip(arguments) {
        if let Err(error) = lower(context, bridge, *kind, value, &mut params, allocations) {
            if error.is::<GuestTrap>() {
                let mut context = context.as_context_mut();
                let state = context.data_mut();
                return Err(Invoked::Stopped(Stop {
                    message: state.stop_message.take(),
                    trap: "ilang_alloc trapped".into(),
                }));
            }
            return Err(Invoked::Failed(error));
        }
    }
    Ok(params)
}

/// Read the results, then free the parameter and result buffers (B.3).
fn finish(
    context: &mut impl AsContextMut<Data = State>,
    bridge: Bridge,
    signature: &Signature,
    results: &[Val],
    mut allocations: Vec<Allocation>,
) -> Result<Outcome> {
    let outcome = lift_results(context, bridge, signature, results, &mut allocations)?;
    bridge
        .free_all(context, &allocations)
        .context("freeing the buffers of a call")?;
    Ok(outcome)
}

fn correct_arguments(signature: &Signature, arguments: &[Value]) -> bool {
    signature
        .params
        .iter()
        .zip(arguments)
        .all(|(kind, value)| value.is_correct(*kind))
}

fn defaults(types: &[ValType]) -> Vec<Val> {
    types
        .iter()
        .map(|ty| Val::default_for_ty(ty).expect("every boundary type has a default"))
        .collect()
}

/// A trap inside `ilang_alloc` or `ilang_free` while the host moves buffers.
#[derive(Debug)]
struct GuestTrap;

impl fmt::Display for GuestTrap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the module trapped in ilang_alloc")
    }
}

impl std::error::Error for GuestTrap {}

/// The value of a guest result that is not correct.
fn guest_fault(what: impl fmt::Display) -> anyhow::Error {
    anyhow!("the module gave a value that is not correct: {what}")
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
                    .context("externref was not created by this i host")?
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
    // The module traps right after this call (B.4). Only the first message of
    // an instance counts.
    linker.func_wrap(
        HOST_MODULE,
        "panic",
        |mut caller: Caller<'_, State>, address: i32, length: i32| {
            let bridge = Bridge::from_caller(&mut caller);
            let message = match bridge.read(&caller, address as u32, length as u32) {
                Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                Err(error) => format!("(the stop message is outside memory: {error})"),
            };
            caller.data_mut().stop_message.get_or_insert(message);
        },
    )?;
    Ok(())
}

fn define_import(
    linker: &mut Linker<State>,
    engine: &Engine,
    module: &str,
    name: &str,
    signature: &Signature,
    function: HostFunction,
    checked: bool,
) -> Result<()> {
    let results = if signature.is_async {
        Vec::new()
    } else {
        signature.result_types()
    };
    let ty = FuncType::new(engine, signature.param_types(), results);
    let signature = signature.clone();
    let (module_name, label) = (module.to_owned(), name.to_owned());
    linker.func_new(module, name, ty, move |mut caller, parameters, results| {
        let bridge = Bridge::from_caller(&mut caller);
        let fail = |caller: &mut Caller<'_, State>, error: anyhow::Error| {
            caller.data_mut().failure = Some(error);
            wasmtime::Error::from_anyhow(anyhow!("a host function failed"))
        };
        let mut offset = 0;
        let mut arguments = Vec::new();
        for kind in &signature.params {
            match lift(&mut caller, bridge, *kind, parameters, &mut offset, None) {
                Ok(value) => arguments.push(value),
                Err(error) => {
                    return Err(fail(
                        &mut caller,
                        error.context(format!("a parameter of {module_name} {label}")),
                    ));
                }
            }
        }
        let reply = match &function {
            HostFunction::AsyncService(start) => {
                let work = start(&arguments);
                caller.data_mut().pending.push(Pending {
                    namespace: module_name.clone(),
                    label: label.clone(),
                    signature: signature.clone(),
                    work,
                });
                return Ok(());
            }
            HostFunction::Service(service) => service(&arguments),
            HostFunction::External(external) => external(&arguments).map(Ok),
        };
        let reply = match reply {
            Ok(reply) => reply,
            Err(error) => return Err(fail(&mut caller, error)),
        };
        let mut values = Vec::new();
        match lower_reply(&mut caller, bridge, &signature, reply, &mut values, checked) {
            Ok(true) => {
                results.clone_from_slice(&values);
                Ok(())
            }
            Ok(false) => {
                caller
                    .data_mut()
                    .stop_message
                    .get_or_insert(HOST_VALUE_REASON.into());
                Err(wasmtime::Error::from_anyhow(anyhow!(HOST_VALUE_REASON)))
            }
            Err(error) => Err(fail(&mut caller, error)),
        }
    })?;
    Ok(())
}

/// Encode the answer of a host function as results of B.2. `Ok(false)` when
/// the answer is not a correct value: the program then stops. Unless
/// `checked`, a value of the right type is given to the module even when it
/// is not correct, and the module stops.
fn lower_reply(
    context: &mut impl AsContextMut<Data = State>,
    bridge: Bridge,
    signature: &Signature,
    reply: std::result::Result<Value, Value>,
    values: &mut Vec<Val>,
    checked: bool,
) -> Result<bool> {
    let fits = |value: &Value, kind: AbiType| {
        if checked {
            value.is_correct(kind)
        } else {
            value.abi_type() == kind
        }
    };
    let (tag, value) = match &reply {
        Ok(value) => (0, value),
        Err(error) => match signature.errors.iter().position(|kind| fits(error, *kind)) {
            Some(index) => (index + 1, error),
            None => return Ok(false),
        },
    };
    if tag == 0 && !fits(value, signature.result) {
        return Ok(false);
    }
    // The buffers belong to the module after a successful return (B.3).
    let mut allocations = Vec::new();
    let lowered = (|| -> Result<()> {
        if !signature.errors.is_empty() {
            values.push(Val::I32(tag as i32));
        }
        let mut slot = |kind: AbiType, used: bool, values: &mut Vec<Val>| -> Result<()> {
            if used {
                lower(context, bridge, kind, value, values, &mut allocations)
            } else {
                values.extend(defaults(kind.wasm_types()));
                Ok(())
            }
        };
        slot(signature.result, tag == 0, values)?;
        for (index, kind) in signature.errors.iter().enumerate() {
            slot(*kind, tag == index + 1, values)?;
        }
        Ok(())
    })();
    if let Err(error) = lowered {
        let _ = bridge.free_all(context, &allocations);
        return Err(error);
    }
    Ok(true)
}

#[derive(Clone, Copy)]
struct Bridge {
    memory: Option<Memory>,
    alloc: Option<Func>,
    free: Option<Func>,
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
                .get_export("ilang_alloc")
                .and_then(|value| value.into_func()),
            free: caller
                .get_export("ilang_free")
                .and_then(|value| value.into_func()),
        }
    }

    fn from_instance(instance: &WasmInstance, store: &mut Store<State>) -> Self {
        Self {
            memory: instance.get_memory(&mut *store, "memory"),
            alloc: instance.get_func(&mut *store, "ilang_alloc"),
            free: instance.get_func(&mut *store, "ilang_free"),
        }
    }

    fn read(self, context: impl AsContext<Data = State>, ptr: u32, len: u32) -> Result<Vec<u8>> {
        if len == 0 {
            return Ok(Vec::new());
        }
        let memory = self.memory.context("the module does not export memory")?;
        let bytes = memory.data(context.as_context());
        Ok(bytes[checked_range(ptr, len, bytes.len())?].to_vec())
    }

    /// Copy into a new buffer from `ilang_alloc`; empty buffers have address 0.
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
        let memory = self.memory.context("the module does not export memory")?;
        let alloc = self
            .alloc
            .context("the module does not export ilang_alloc")?
            .typed::<i32, i32>(context.as_context())?;
        let ptr = alloc
            .call(context.as_context_mut(), len as i32)
            .map_err(|_| anyhow::Error::new(GuestTrap))? as u32;
        allocations.push(Allocation { ptr, len });
        memory
            .write(context.as_context_mut(), ptr as usize, bytes)
            .context("ilang_alloc gave a buffer outside memory")?;
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
            .context("the module does not export ilang_free")?
            .typed::<(i32, i32), ()>(context.as_context())?;
        for (index, allocation) in allocations.iter().enumerate().rev() {
            if allocation.ptr == 0
                || allocations[..index]
                    .iter()
                    .any(|earlier| earlier.ptr == allocation.ptr)
            {
                continue;
            }
            free.call(
                context.as_context_mut(),
                (allocation.ptr as i32, allocation.len as i32),
            )?;
        }
        Ok(())
    }
}

fn checked_range(ptr: u32, len: u32, memory_len: usize) -> Result<std::ops::Range<usize>> {
    let start = ptr as usize;
    let end = start
        .checked_add(len as usize)
        .context("buffer address overflow")?;
    ensure!(
        end <= memory_len,
        "buffer {start}..{end} is outside memory ({memory_len} bytes)"
    );
    Ok(start..end)
}

fn bytes_for_bits(bits: u32) -> usize {
    (u64::from(bits).div_ceil(8)) as usize
}

/// Encode a correct value; buffers come from `ilang_alloc`.
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
        (AbiType::Float, Value::Float(value)) => values.push(Val::F64(value.to_bits())),
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

/// Read the results of an export: the tag, the value or one error.
fn lift_results(
    context: &mut impl AsContextMut<Data = State>,
    bridge: Bridge,
    signature: &Signature,
    results: &[Val],
    allocations: &mut Vec<Allocation>,
) -> Result<Outcome> {
    let mut offset = 0;
    let tag = if signature.errors.is_empty() {
        0
    } else {
        offset = 1;
        let tag = results[0].i32().context("the tag is not an i32")?;
        usize::try_from(tag)
            .ok()
            .filter(|tag| *tag <= signature.errors.len())
            .ok_or_else(|| guest_fault(format!("error tag {tag}")))?
    };
    let kinds = std::iter::once(signature.result).chain(signature.errors.iter().copied());
    let mut outcome = None;
    for (index, kind) in kinds.enumerate() {
        if index == tag {
            let value = lift(
                context,
                bridge,
                kind,
                results,
                &mut offset,
                Some(allocations),
            )?;
            outcome = Some(if tag == 0 {
                Outcome::Ok(value)
            } else {
                Outcome::Err(GuestError(value))
            });
        } else {
            offset += kind.wasm_types().len();
        }
    }
    Ok(outcome.expect("the tag selects one of the kinds"))
}

/// Read a value that the module gave, checking that it is correct (B.2).
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
            && ptr != 0
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
            invalid => return Err(guest_fault(format!("Bool {invalid}"))),
        },
        AbiType::Float => {
            let value = take(offset)?.f64().context("expected f64 ABI value")?;
            ensure!(value.is_finite(), guest_fault("a Float that is not finite"));
            Value::Float(value)
        }
        AbiType::Int => {
            let sign = integer(offset)?;
            let ptr = integer(offset)? as u32;
            let count = integer(offset)? as u32;
            let len = count
                .checked_mul(4)
                .ok_or_else(|| guest_fault("an Int too large for memory"))?;
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
                _ => return Err(guest_fault(format!("Int sign {sign}"))),
            };
            ensure!(
                (sign == Sign::NoSign) == limbs.is_empty(),
                guest_fault("an Int whose sign does not match its count")
            );
            ensure!(
                limbs.last().is_none_or(|limb| *limb != 0),
                guest_fault("an Int whose last part is 0")
            );
            Value::Int(BigInt::from_slice(sign, &limbs))
        }
        AbiType::String => {
            let ptr = integer(offset)? as u32;
            let len = integer(offset)? as u32;
            Value::String(
                String::from_utf8(buffer(ptr, len)?)
                    .map_err(|_| guest_fault("a String that is not UTF-8"))?,
            )
        }
        AbiType::BitArray => {
            let ptr = integer(offset)? as u32;
            let bit_length = integer(offset)? as u32;
            let value = BitArray {
                bytes: buffer(ptr, bytes_for_bits(bit_length) as u32)?,
                bit_length,
            };
            ensure!(
                value.is_correct(),
                guest_fault("a BitArray whose bits after the end are not 0")
            );
            Value::BitArray(value)
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
                .context("externref was not created by this i host")?
                .clone();
            Value::Externref(value)
        }
    })
}
