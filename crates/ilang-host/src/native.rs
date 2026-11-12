//! Native packages (docs/design/native.md, N1, N2 and N4).
//!
//! A native package is one executable file: a copy of the runner
//! (`crates/ilang-runner`), which has the Wasmtime runtime, the standard host
//! services and the reactor but no compiler, followed by a payload with the
//! program, compiled ahead of time by `i build`:
//!
//! ```text
//! runner | manifest (JSON) | metadata module | precompiled code | trailer
//! trailer = payload length (u64 LE) | manifest length (u64 LE) | SHA-256 of the payload | MAGIC
//! ```
//!
//! The payload is the three parts between the runner and the trailer. The
//! runner finds the trailer at the end of its own file, checks its magic,
//! lengths and digest, then the [`Manifest`]: its format, and the engine
//! contract ([`contract`]) that the code was compiled for, which must be the
//! runner's own. Only then does it give the code to Wasmtime's unsafe
//! `Module::deserialize`, which checks the compiler settings once more. The
//! manifest binds the binding plan to the exact module (its artifact digest)
//! and the code to its digest. The digest detects a damaged or truncated
//! file; it is not a signature (signing stays open, native.md §7).
//!
//! The metadata module is a Wasm module with only the `ilang.*` custom
//! sections of the program, read exactly as [`Program::from_bytes`] reads
//! them.
//!
//! Packages are built for the triple of the runner (statically linked, musl on
//! Linux), with a
//! fixed CPU feature baseline ([`BASELINE`]) rather than the features of the
//! building machine's CPU.

use crate::{
    HostOptions, Metadata, Outcome, Program, bindings::BindingPlan, runtime_config, services,
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use wasmtime::{Collector, Config, Engine, Module};

/// The version of the payload format.
pub const FORMAT: u32 = 1;

/// The last bytes of every native package.
pub const MAGIC: &[u8; 16] = b"ilang-native-v1\0";

/// The size of the trailer after the payload.
pub const TRAILER: usize = 8 + 8 + 32 + MAGIC.len();

/// The Wasmtime version of both `i build` and the runner. Cargo.toml pins
/// `wasmtime = "=50.0.0"` for `ilang-host`, the one crate both use; a test
/// checks that this text is that pin.
pub const WASMTIME: &str = "50.0.0";

/// The CPU features that compiled code may use, beyond those of the target
/// architecture itself: x86-64-v2 (SSE3, SSSE3, SSE4.1, SSE4.2, POPCNT) on
/// x86-64, and the base architecture elsewhere. Wasmtime checks at load time
/// that the CPU has them.
pub const BASELINE: &str = if cfg!(target_arch = "x86_64") {
    "x86-64-v2"
} else {
    "base"
};

#[cfg(feature = "compiler")]
const BASELINE_FLAGS: &[&str] = if cfg!(target_arch = "x86_64") {
    &[
        "has_sse3",
        "has_ssse3",
        "has_sse41",
        "has_sse42",
        "has_popcnt",
    ]
} else {
    &[]
};

/// The target triple of this build of the host: a runner loads only code
/// compiled for its own triple. `i build` compiles for the triple of the
/// runner it carries, which is a static target (crates/ilang-cli/build.rs),
/// not necessarily the triple of `i` itself.
pub fn triple() -> String {
    target_lexicon::Triple::host().to_string()
}

/// Everything the compiled code depends on, apart from the program: the
/// Wasmtime version, target triple, CPU baseline, collector and engine
/// tunables. `i build` writes it into the manifest, and the runner refuses a
/// payload whose contract is not its own.
pub fn contract(target: &str, collector: Collector, gc_heap: u64) -> String {
    format!(
        "ilang-host {}; abi {}; wasmtime {WASMTIME}; target {}; cpu {BASELINE}; collector {}; gc heap {gc_heap}; wasm stack {}; thread stack {}; epochs off; fuel off",
        env!("CARGO_PKG_VERSION"),
        crate::ABI_VERSION,
        target,
        collector_name(collector),
        crate::WASM_STACK,
        crate::THREAD_STACK,
    )
}

fn collector_name(collector: Collector) -> &'static str {
    match collector {
        Collector::Copying => "copying",
        Collector::DeferredReferenceCounting => "drc",
        Collector::Null => "null",
        _ => panic!("the host configures only the copying, drc and null collectors"),
    }
}

fn collector_of(name: &str) -> Result<Collector> {
    Ok(match name {
        "copying" => Collector::Copying,
        "drc" => Collector::DeferredReferenceCounting,
        "null" => Collector::Null,
        _ => bail!("the package names an unknown collector {name:?}"),
    })
}

/// The engine configuration that both `i build` and the runner use; the
/// compiler adds only settings of the compiler to it.
fn config(target: &str, collector: Collector, gc_heap: u64) -> Result<Config> {
    let mut config = runtime_config(collector, gc_heap);
    // An explicit target: Wasmtime then infers no CPU features of this
    // machine, so the baseline below is all the code uses.
    config.target(target)?;
    Ok(config)
}

#[cfg(feature = "compiler")]
fn compiler_engine(target: &str, collector: Collector, gc_heap: u64) -> Result<Engine> {
    let mut config = config(target, collector, gc_heap)?;
    config.parallel_compilation(true);
    config.cranelift_opt_level(wasmtime::OptLevel::Speed);
    for flag in BASELINE_FLAGS {
        // SAFETY: Wasmtime checks each enabled ISA flag against the CPU
        // when the runner loads the code.
        unsafe {
            config.cranelift_flag_enable(flag);
        }
    }
    Ok(Engine::new(&config)?)
}

fn runtime_engine(collector: Collector, gc_heap: u64) -> Result<Engine> {
    Ok(Engine::new(&config(&triple(), collector, gc_heap)?)?)
}

/// The description of a payload, the first part of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// [`FORMAT`].
    pub format: u32,
    /// The [`contract`] the code was compiled for.
    pub contract: String,
    pub collector: String,
    pub gc_heap: u64,
    /// The name of the package or of the selected entry.
    pub name: String,
    /// The digest of the complete Wasm module the code was compiled from.
    pub artifact_digest: String,
    /// The compiler's trusted binding plan for that module (native.md §3.1).
    pub binding_plan: BindingPlan,
    /// The content identity of the library package whose declarations the
    /// runner grants the standard host services.
    pub trusted_package: String,
    pub metadata_len: u64,
    pub code_len: u64,
    /// The SHA-256 digest of the precompiled code.
    pub code_digest: String,
}

/// The three parts of a payload.
#[derive(Clone, Debug)]
pub struct Payload {
    pub manifest: Manifest,
    pub metadata: Vec<u8>,
    pub code: Vec<u8>,
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256-{:x}", Sha256::digest(bytes))
}

impl Payload {
    /// The bytes that follow the runner: the payload and its trailer.
    pub fn encode(&self) -> Result<Vec<u8>> {
        let manifest = serde_json::to_vec(&self.manifest)?;
        let mut out =
            Vec::with_capacity(manifest.len() + self.metadata.len() + self.code.len() + TRAILER);
        out.extend_from_slice(&manifest);
        out.extend_from_slice(&self.metadata);
        out.extend_from_slice(&self.code);
        let sum = Sha256::digest(&out);
        let payload_len = out.len() as u64;
        out.extend_from_slice(&payload_len.to_le_bytes());
        out.extend_from_slice(&(manifest.len() as u64).to_le_bytes());
        out.extend_from_slice(&sum);
        out.extend_from_slice(MAGIC);
        Ok(out)
    }

    /// Read the payload at the end of `file`, and the length of the runner
    /// before it. This checks the trailer, the digest and the lengths, not
    /// the contract.
    pub fn decode(file: &[u8]) -> Result<(usize, Payload)> {
        let damaged = "the program payload of this executable is missing, truncated or damaged";
        ensure!(file.len() >= TRAILER, "{damaged} (no trailer)");
        let trailer = &file[file.len() - TRAILER..];
        ensure!(&trailer[48..] == MAGIC, "{damaged} (no trailer)");
        let payload_len = u64::from_le_bytes(trailer[..8].try_into().unwrap());
        let manifest_len = u64::from_le_bytes(trailer[8..16].try_into().unwrap());
        let before = (file.len() - TRAILER) as u64;
        ensure!(
            payload_len <= before && manifest_len <= payload_len,
            "{damaged} (bad lengths)"
        );
        let start = (before - payload_len) as usize;
        let payload = &file[start..file.len() - TRAILER];
        ensure!(
            Sha256::digest(payload).as_slice() == &trailer[16..48],
            "{damaged} (the digest does not match)"
        );
        let (manifest, rest) = payload.split_at(manifest_len as usize);
        let manifest: Manifest =
            serde_json::from_slice(manifest).context("the payload manifest is not valid")?;
        ensure!(
            manifest.format == FORMAT,
            "the program payload has format {}, but this runner reads format {FORMAT}",
            manifest.format
        );
        ensure!(
            manifest.metadata_len.checked_add(manifest.code_len) == Some(rest.len() as u64),
            "{damaged} (the manifest lengths do not match)"
        );
        let (metadata, code) = rest.split_at(manifest.metadata_len as usize);
        ensure!(
            digest(code) == manifest.code_digest,
            "{damaged} (the code digest does not match the manifest)"
        );
        Ok((
            start,
            Payload {
                manifest,
                metadata: metadata.to_vec(),
                code: code.to_vec(),
            },
        ))
    }
}

/// A Wasm module with only the `ilang.*` custom sections of `wasm`.
#[cfg(feature = "compiler")]
fn metadata_module(wasm: &[u8]) -> Result<Vec<u8>> {
    fn leb(mut n: usize, out: &mut Vec<u8>) {
        loop {
            let byte = (n & 0x7f) as u8;
            n >>= 7;
            if n == 0 {
                out.push(byte);
                return;
            }
            out.push(byte | 0x80);
        }
    }
    let mut out = b"\0asm\x01\0\0\0".to_vec();
    for payload in wasmparser::Parser::new(0).parse_all(wasm) {
        if let wasmparser::Payload::CustomSection(section) = payload?
            && section.name().starts_with("ilang.")
        {
            let mut body = Vec::new();
            leb(section.name().len(), &mut body);
            body.extend_from_slice(section.name().as_bytes());
            body.extend_from_slice(section.data());
            out.push(0);
            leb(body.len(), &mut out);
            out.extend_from_slice(&body);
        }
    }
    Ok(out)
}

/// The host options of a native package: the standard host services of
/// `i run`, with `args`, granted to the declarations of the trusted library
/// package of the plan. The same for `i build`'s check (N4) and the runner.
pub fn host_options(args: Vec<String>, plan: &BindingPlan, trusted_package: &str) -> HostOptions {
    let mut options = HostOptions::new(services::stdout_with_args(args));
    options.imports.grant_package(plan, trusted_package);
    options
}

/// Make a native package: `runner`, built for the triple `target`, followed
/// by the payload of `wasm`, compiled for that triple ([`contract`]). Fails when the module
/// has no program entry, or imports a function the runner cannot give (N4).
#[cfg(feature = "compiler")]
pub fn package(
    runner: &[u8],
    target: &str,
    name: &str,
    wasm: &[u8],
    plan: &BindingPlan,
    trusted_package: &str,
) -> Result<Vec<u8>> {
    let artifact_digest = crate::bindings::artifact_digest(wasm);
    let metadata = Metadata::read(wasm, artifact_digest.clone())?;
    ensure!(
        metadata.abi.entry.is_some(),
        "native packaging requires the program entry: define `main` in `src/main.i` or name a declared entry; a package with only host exports is built with --target host (or ilang)"
    );
    let options = host_options(Vec::new(), plan, trusted_package);
    let mut missing = Vec::new();
    for table in [&metadata.abi.services, &metadata.abi.externals] {
        for (namespace, fields) in table {
            for name in fields.keys() {
                let key = (namespace.clone(), name.clone());
                if !options.imports.functions.contains_key(&key) {
                    missing.push(format!("{namespace} {name}"));
                }
            }
        }
    }
    for declaration in &plan.declarations {
        let key = (declaration.namespace.clone(), declaration.name.clone());
        if options.imports.functions.contains_key(&key)
            && !options.imports.grants.allows(plan, declaration)
        {
            missing.push(format!(
                "{} {} (declared by {} {}, which is not a declaration of the standard library)",
                declaration.namespace,
                declaration.name,
                declaration.module,
                declaration.declaration
            ));
        }
    }
    if !missing.is_empty() {
        bail!(
            "the native runner cannot supply {} required host import{}: {}; build with --target host (or ilang) and give {} from an embedding host",
            if missing.len() == 1 { "this" } else { "these" },
            if missing.len() == 1 { "" } else { "s" },
            missing.join(", "),
            if missing.len() == 1 { "it" } else { "them" },
        );
    }
    crate::check_imports(
        &metadata.abi,
        &metadata.provider_registrations,
        &metadata.artifact_digest,
        Some(plan),
        &options.imports,
    )?;
    let (collector, gc_heap) = crate::gc_settings()?;
    let code = compiler_engine(target, collector, gc_heap)?
        .precompile_module(wasm)
        .map_err(anyhow::Error::from)
        .context("compiling the i WebAssembly module for the native package")?;
    let metadata = metadata_module(wasm)?;
    let payload = Payload {
        manifest: Manifest {
            format: FORMAT,
            contract: contract(target, collector, gc_heap),
            collector: collector_name(collector).into(),
            gc_heap,
            name: name.into(),
            artifact_digest,
            binding_plan: plan.clone(),
            trusted_package: trusted_package.into(),
            metadata_len: metadata.len() as u64,
            code_len: code.len() as u64,
            code_digest: digest(&code),
        },
        metadata,
        code,
    };
    let mut out = runner.to_vec();
    out.extend_from_slice(&payload.encode()?);
    Ok(out)
}

/// Load the program of a native package from the bytes of its file.
pub fn load(file: &[u8]) -> Result<(Program, Manifest)> {
    let (_, payload) = Payload::decode(file)?;
    let manifest = payload.manifest;
    let collector = collector_of(&manifest.collector)?;
    let expected = contract(&triple(), collector, manifest.gc_heap);
    ensure!(
        manifest.contract == expected,
        "the program was compiled for another runner: it needs `{}`, and this runner is `{expected}`; build it again with the `i` of this runner",
        manifest.contract
    );
    let metadata = Metadata::read(&payload.metadata, manifest.artifact_digest.clone())?;
    let engine = runtime_engine(collector, manifest.gc_heap)?;
    // SAFETY: the code is the payload that `i build` compiled for this
    // contract, whose digest was checked above; Wasmtime checks the
    // compiler settings and CPU features again. An arbitrary .cwasm file is
    // never loaded.
    let module = unsafe { Module::deserialize(&engine, &payload.code) }
        .map_err(anyhow::Error::from)
        .context("loading the compiled program")?;
    let program = Program::from_parts(engine, module, metadata)?
        .with_binding_plan(manifest.binding_plan.clone())?;
    Ok((program, manifest))
}

/// The payload and trailer at the end of the file `exe`, read without the
/// runner before them.
pub fn read_payload(exe: &Path) -> Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let damaged = "the program payload of this executable is missing, truncated or damaged";
    let mut file =
        std::fs::File::open(exe).with_context(|| format!("opening {}", exe.display()))?;
    let len = file.metadata()?.len();
    ensure!(len >= TRAILER as u64, "{damaged} (no trailer)");
    let mut trailer = [0; TRAILER];
    file.seek(SeekFrom::Start(len - TRAILER as u64))?;
    file.read_exact(&mut trailer)?;
    ensure!(&trailer[48..] == MAGIC, "{damaged} (no trailer)");
    let payload_len = u64::from_le_bytes(trailer[..8].try_into().unwrap());
    ensure!(
        payload_len <= len - TRAILER as u64,
        "{damaged} (bad lengths)"
    );
    let start = len - TRAILER as u64 - payload_len;
    file.seek(SeekFrom::Start(start))?;
    let mut tail = Vec::with_capacity((len - start) as usize);
    file.read_to_end(&mut tail)?;
    Ok(tail)
}

/// The runner: load the program of the executable at `exe` and run its
/// entry with `args`, the standard host services and the exit status of
/// `i run`.
pub fn run(exe: &Path, args: Vec<String>) -> std::process::ExitCode {
    use std::process::ExitCode;
    let loaded = read_payload(exe).and_then(|tail| load(&tail));
    let (program, manifest) = match loaded {
        Ok(loaded) => loaded,
        Err(error) => {
            eprintln!("{}: {error:#}", exe.display());
            return ExitCode::FAILURE;
        }
    };
    let options = host_options(args, &manifest.binding_plan, &manifest.trusted_package);
    match program.run_entry(options) {
        Ok(Outcome::Ok(_)) => ExitCode::SUCCESS,
        Ok(Outcome::Stop(stop)) => {
            eprintln!("{stop}");
            ExitCode::FAILURE
        }
        Ok(Outcome::Err(error)) => {
            eprintln!("error: {:?}", error.0);
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}
