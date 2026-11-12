//! Optimizes WasmGC modules with Binaryen, linked into the compiler as a
//! library.
//!
//! The i compiler emits simple, unoptimized Wasm and leaves optimization to
//! Binaryen. Only the few C API calls that this needs are bound here.
//!
//! Custom sections whose names start with `ilang.` (the binary interface
//! metadata of a compiled module) are taken out before Binaryen reads the
//! module and are put back, unchanged and in order, at the end of the result.
//! So the optimizer can never change or lose them.

mod ffi;

use std::sync::Mutex;

/// How much to optimize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// No optimization: the module is given back as it is.
    O0,
    /// Optimizes for speed, as `wasm-opt -O2`, but with the inlining of
    /// [`ALWAYS_INLINE`].
    O2,
    /// Optimizes more, as `wasm-opt -O3`, with the same inlining.
    O3,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the module is not a well-formed Wasm binary: {0}")]
    Malformed(&'static str),
    #[error("Binaryen could not read the module")]
    Read,
    #[error("Binaryen found the module not valid")]
    Invalid,
    #[error("Binaryen gave no output")]
    Write,
}

/// Functions of at most this many Binaryen expressions are always inlined
/// (Binaryen's default is 2). The runtime and the generated code call many
/// small functions, such as `$list_head` and the shadow stack ones, and
/// Wasmtime inlines nothing, so each would stay a call with its spills of GC
/// references. 20 makes list-heavy benchmarks about 25% faster and the
/// optimization of `std`'s tests about 1 s slower; 40 gains little more and
/// costs twice that (docs/performance.md).
pub const ALWAYS_INLINE: u32 = 20;

/// Binaryen keeps its optimization settings in global state, so every use of
/// it is done while holding this lock.
static BINARYEN: Mutex<()> = Mutex::new(());

/// The Wasm features that i programs may use: GC with typed function
/// references, tail calls, and exception handling, and
/// the small features that every engine with these also has.
fn features() -> ffi::BinaryenFeatures {
    // SAFETY: these functions only return constants.
    unsafe {
        ffi::BinaryenFeatureGC()
            | ffi::BinaryenFeatureReferenceTypes()
            | ffi::BinaryenFeatureTailCall()
            | ffi::BinaryenFeatureExceptionHandling()
            | ffi::BinaryenFeatureMultivalue()
            | ffi::BinaryenFeatureBulkMemory()
            | ffi::BinaryenFeatureBulkMemoryOpt()
            | ffi::BinaryenFeatureSignExt()
            | ffi::BinaryenFeatureNontrappingFPToInt()
            | ffi::BinaryenFeatureMutableGlobals()
    }
}

/// Optimizes `wasm` at `level`.
///
/// The optimizer is run in closed-world mode: only numbers, buffers in linear
/// memory, and `externref` values cross the host boundary, so no GC type is
/// seen from outside. Traps are never assumed away, because a trap is how a
/// program stops.
pub fn optimize(wasm: &[u8], level: Level) -> Result<Vec<u8>, Error> {
    let (module, kept) = split_custom_sections(wasm)?;
    if level == Level::O0 {
        return Ok(wasm.to_vec());
    }
    let optimize_level = match level {
        Level::O0 => unreachable!(),
        Level::O2 => 2,
        Level::O3 => 3,
    };

    let _guard = BINARYEN
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut input = module;
    // SAFETY: Binaryen copies the input; the module is disposed on every path
    // below, and the output buffer is freed after it is copied.
    let mut output = unsafe {
        let module =
            ffi::BinaryenModuleReadWithFeatures(input.as_mut_ptr().cast(), input.len(), features());
        if module.is_null() {
            return Err(Error::Read);
        }
        let module = ModuleGuard(module);
        ffi::BinaryenModuleSetFeatures(module.0, features());
        if !ffi::BinaryenModuleValidate(module.0) {
            return Err(Error::Invalid);
        }
        ffi::BinaryenSetOptimizeLevel(optimize_level);
        ffi::BinaryenSetShrinkLevel(0);
        ffi::BinaryenSetClosedWorld(true);
        ffi::BinaryenSetTrapsNeverHappen(false);
        ffi::BinaryenSetDebugInfo(false);
        ffi::BinaryenSetAlwaysInlineMaxSize(ALWAYS_INLINE);
        ffi::BinaryenModuleOptimize(module.0);
        if !ffi::BinaryenModuleValidate(module.0) {
            return Err(Error::Invalid);
        }
        let result = ffi::BinaryenModuleAllocateAndWrite(module.0, std::ptr::null());
        if result.binary.is_null() {
            return Err(Error::Write);
        }
        let bytes =
            std::slice::from_raw_parts(result.binary.cast::<u8>(), result.binaryBytes).to_vec();
        libc_free(result.binary);
        if !result.sourceMap.is_null() {
            libc_free(result.sourceMap.cast());
        }
        bytes
    };
    for section in kept {
        output.extend_from_slice(&section);
    }
    Ok(output)
}

struct ModuleGuard(ffi::BinaryenModuleRef);

impl Drop for ModuleGuard {
    fn drop(&mut self) {
        // SAFETY: the module was made by BinaryenModuleReadWithFeatures and is
        // disposed only here.
        unsafe { ffi::BinaryenModuleDispose(self.0) }
    }
}

unsafe extern "C" {
    #[link_name = "free"]
    fn libc_free(pointer: *mut std::ffi::c_void);
}

/// Splits a module into its bytes without the `ilang.*` custom sections, and
/// those sections, each as a complete section with its header.
fn split_custom_sections(wasm: &[u8]) -> Result<(Vec<u8>, Vec<Vec<u8>>), Error> {
    const HEADER: usize = 8;
    if wasm.len() < HEADER || wasm[..4] != *b"\0asm" {
        return Err(Error::Malformed("no Wasm header"));
    }
    let mut module = wasm[..HEADER].to_vec();
    let mut kept = Vec::new();
    let mut at = HEADER;
    while at < wasm.len() {
        let start = at;
        let id = wasm[at];
        at += 1;
        let size = read_u32(wasm, &mut at)? as usize;
        let end = at
            .checked_add(size)
            .filter(|end| *end <= wasm.len())
            .ok_or(Error::Malformed("a section goes past the end"))?;
        let section = &wasm[start..end];
        if id == 0 {
            let mut name_at = at;
            let name_len = read_u32(wasm, &mut name_at)? as usize;
            let name = wasm
                .get(name_at..name_at + name_len)
                .ok_or(Error::Malformed("a custom section name goes past its end"))?;
            if name.starts_with(b"ilang.") {
                kept.push(section.to_vec());
                at = end;
                continue;
            }
        }
        module.extend_from_slice(section);
        at = end;
    }
    Ok((module, kept))
}

fn read_u32(bytes: &[u8], at: &mut usize) -> Result<u32, Error> {
    let mut result = 0u32;
    for shift in (0..35).step_by(7) {
        let byte = *bytes
            .get(*at)
            .ok_or(Error::Malformed("a number goes past the end"))?;
        *at += 1;
        result |= u32::from(byte & 0x7f)
            .checked_shl(shift)
            .ok_or(Error::Malformed("a number is too big"))?;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
    }
    Err(Error::Malformed("a number is too long"))
}

#[cfg(test)]
mod tests;
