//! Content-addressed semantic caches. Cache data is an optimisation: malformed,
//! old, or incomplete entries are ignored and never bypass source validation.

use gleam_core::{ast::TypedModule, type_::ModuleInterface};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

pub(crate) const ABI_VERSION: &str = "wleam-core-wasm-gc-abi-1";
const CACHE_MAGIC: &[u8; 8] = b"WLEAMCF2";
const CACHE_HEADER_LENGTH: usize = CACHE_MAGIC.len() + 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub schema: String,
    pub source_key: String,
    pub dependencies: Vec<(String, src_span::SrcSpan)>,
    pub dependency_interfaces: BTreeMap<String, String>,
    pub interface_hash: String,
    pub next_id: u64,
    pub module: Arc<TypedModule>,
    pub warnings: Vec<String>,
}

pub(crate) fn schema() -> String {
    format!(
        "{}:{}:{}",
        env!("CARGO_PKG_VERSION"),
        ABI_VERSION,
        crate::compiler_fingerprint()
    )
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn source_key(
    code: &str,
    path: &Path,
    config: &gleam_core::config::PackageConfig,
    is_test: bool,
) -> String {
    // serde_json::Map is ordered, so HashMap insertion order cannot invalidate
    // otherwise identical package configurations.
    let config = serde_json::to_value(config).expect("package configuration is serialisable");
    let bytes = serde_json::to_vec(&(schema(), path, code, config, is_test))
        .expect("cache key is serialisable");
    digest(&bytes)
}

pub(crate) fn entry_path(directory: &Path, name: &str) -> PathBuf {
    directory.join(format!("{}.bin", digest(name.as_bytes())))
}

pub(crate) fn read(directory: &Path, name: &str) -> Option<Entry> {
    let bytes = fs::read(entry_path(directory, name)).ok()?;
    if !bytes.starts_with(CACHE_MAGIC) || bytes.len() < CACHE_HEADER_LENGTH {
        return None;
    }
    let payload = &bytes[CACHE_HEADER_LENGTH..];
    // A valid bitcode stream can still contain corrupted values. Verify the
    // entire payload before trusting its source key or restoring typed trees.
    let checksum = Sha256::digest(payload);
    if bytes[CACHE_MAGIC.len()..CACHE_HEADER_LENGTH] != checksum[..] {
        return None;
    }
    let entry: Entry = bitcode::deserialize(payload).ok()?;
    (entry.schema == schema() && entry.module.name == name).then_some(entry)
}

pub(crate) fn write(directory: &Path, name: &str, entry: &Entry) -> std::io::Result<()> {
    static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);
    fs::create_dir_all(directory)?;
    let payload = bitcode::serialize(entry).map_err(std::io::Error::other)?;
    let mut bytes = Vec::with_capacity(CACHE_HEADER_LENGTH + payload.len());
    bytes.extend_from_slice(CACHE_MAGIC);
    bytes.extend_from_slice(&Sha256::digest(&payload));
    bytes.extend_from_slice(&payload);
    let path = entry_path(directory, name);
    // Each cache write owns its temporary name; readers only observe a
    // complete, atomically replaced entry.
    let temporary = path.with_extension(format!(
        "{}-{}.tmp",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed),
    ));
    fs::write(&temporary, bytes)?;
    if let Err(error) = fs::rename(&temporary, &path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

/// Hash the importable contract, not source positions or a function's body.
/// Generic IDs are alpha-renamed so inference allocation order does not change
/// the contract. Public constants remain included because users inline them.
pub(crate) fn interface_hash(interface: &ModuleInterface) -> String {
    let mut interface = interface.clone();
    // References have tuple and record map keys, which JSON cannot encode.
    // They describe editor locations rather than the importable contract.
    // Remove them before serialisation, not during JSON normalisation.
    interface.references = Default::default();
    interface.warnings.clear();
    interface
        .values
        .retain(|_, value| value.publicity.is_importable());
    interface
        .types
        .retain(|_, value| value.publicity.is_importable());
    interface
        .type_aliases
        .retain(|_, value| value.publicity.is_importable());
    let exported_types: std::collections::HashSet<_> = interface.types.keys().cloned().collect();
    interface
        .accessors
        .retain(|name, _| exported_types.contains(name));
    interface
        .types_value_constructors
        .retain(|name, _| exported_types.contains(name));
    digest(&crate::semantic::bytes(&interface).expect("module interfaces are serialisable"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gleam_core::{
        analyse::{ModuleAnalyzerConstructor, TargetSupport},
        build::{Origin, Target},
        config::PackageConfig,
        type_::{PRELUDE_MODULE_NAME, build_prelude},
        uid::UniqueIdGenerator,
        warning::{TypeWarningEmitter, WarningEmitter},
    };
    use src_span::LineNumbers;

    fn compile(source: &str, first_id: u64) -> TypedModule {
        let ids = UniqueIdGenerator::new();
        ids.reserve_until(first_id);
        let path = camino::Utf8PathBuf::from("cache_subject.gleam");
        let mut parsed =
            gleam_core::parse::parse_module(path.clone(), source, &WarningEmitter::null())
                .expect("valid cache fixture syntax");
        parsed.module.name = "cache_subject".into();
        let mut modules = imbl::HashMap::new();
        let _ = modules.insert(PRELUDE_MODULE_NAME.into(), build_prelude(&ids));
        let config = PackageConfig {
            name: "cache_fixture".into(),
            ..Default::default()
        };
        ModuleAnalyzerConstructor::<()> {
            target: Target::Wasm,
            ids: &ids,
            origin: Origin::Src,
            importable_modules: &modules,
            warnings: &TypeWarningEmitter::null(),
            direct_dependencies: &std::collections::HashMap::new(),
            dev_dependencies: &std::collections::HashSet::new(),
            target_support: TargetSupport::Enforced,
            package_config: &config,
        }
        .infer_module(parsed.module, LineNumbers::new(source), path)
        .expect("valid cache fixture types")
    }

    #[test]
    fn typed_module_binary_round_trip_preserves_inference_and_patterns() {
        let original = compile(
            r#"
pub type Box(a) { Box(value: a) }
pub const boxed = Box(42)
pub fn map(box: Box(a), transform: fn(a) -> b) -> Box(b) {
  let Box(value) = box
  Box(transform(value))
}
pub fn main() {
  let transform = fn(value) { value + 1 }
  case map(boxed, transform) {
    Box(value: 43) -> <<43:size(6), 1:size(2)>>
    Box(value: _) -> <<0>>
  }
}
"#,
            0,
        );
        assert!(!original.type_info.references.value_references.is_empty());
        assert!(!original.type_info.references.label_references.is_empty());
        let bytes = bitcode::serialize(&original).expect("serialise typed module");
        let restored: TypedModule = bitcode::deserialize(&bytes).expect("restore typed module");
        assert_eq!(original.type_info, restored.type_info);
        assert_eq!(original.names, restored.names);
        assert_eq!(
            original.definitions.functions,
            restored.definitions.functions
        );
        assert_eq!(
            original.definitions.constants,
            restored.definitions.constants
        );
        assert_eq!(
            original.definitions.custom_types,
            restored.definitions.custom_types
        );
        assert_eq!(
            original.unused_definition_positions,
            restored.unused_definition_positions
        );
    }

    #[test]
    fn interface_hash_ignores_inference_ids_source_spans_and_private_changes() {
        let source = r#"
pub type Box(a) { Box(value: a) }
pub type Wrapped(a) = Box(a)
pub const boxed = Box(42)
pub fn identity(value: a) -> a { value }
"#;
        let moved = format!("// Source moved.\n\nfn private_helper() {{ Nil }}\n{source}");
        let first = compile(source, 0);
        let second = compile(&moved, 100_000);
        assert!(!first.type_info.references.type_references.is_empty());
        assert_eq!(
            interface_hash(&first.type_info),
            interface_hash(&second.type_info)
        );
    }

    #[test]
    fn interface_hash_changes_when_exported_values_or_types_change() {
        let first = compile("pub const answer = 1", 0);
        let second = compile("pub const answer = 2", 500);
        let different_type = compile("pub const answer = \"1\"", 1000);
        assert_ne!(
            interface_hash(&first.type_info),
            interface_hash(&second.type_info)
        );
        assert_ne!(
            interface_hash(&first.type_info),
            interface_hash(&different_type.type_info)
        );
    }

    #[test]
    fn interface_hash_ignores_function_body_changes_with_the_same_contract() {
        let first = compile("pub fn increment(value: Int) -> Int { value + 1 }", 0);
        let second = compile("pub fn increment(value: Int) -> Int { value + 2 }", 500);
        assert_eq!(
            interface_hash(&first.type_info),
            interface_hash(&second.type_info)
        );
    }

    #[test]
    fn interface_hash_preserves_relationships_between_generic_parameters() {
        let first = compile("pub fn choose(first: a, _second: b) -> a { first }", 0);
        let second = compile("pub fn choose(_first: a, second: b) -> b { second }", 500);
        assert_ne!(
            interface_hash(&first.type_info),
            interface_hash(&second.type_info)
        );
    }

    #[test]
    fn valid_bitcode_payload_corruption_rechecks_source_and_repairs_cache() {
        use crate::{CheckOptions, check_project};
        use gleam_core::ast::{Statement, TypedExpr};

        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("src")).unwrap();
        fs::write(root.path().join("gleam.toml"), "name = \"app\"\n").unwrap();
        fs::write(root.path().join("src/app.gleam"), "pub fn main() { 42 }\n").unwrap();
        let options = CheckOptions::default();
        let first = check_project(root.path(), &options).unwrap();
        assert_eq!(first.stats.cache_writes, 1);

        let directory = root.path().join("build/.wleam/frontend-v1");
        let path = entry_path(&directory, "app");
        let mut entry = read(&directory, "app").unwrap();
        let module = Arc::make_mut(&mut entry.module);
        let Statement::Expression(TypedExpr::Int {
            value, int_value, ..
        }) = &mut module.definitions.functions[0].body[0]
        else {
            panic!("fixture main returns an integer")
        };
        *value = "99".into();
        *int_value = 99.into();
        let damaged_payload = bitcode::serialize(&entry).unwrap();
        assert!(bitcode::deserialize::<Entry>(&damaged_payload).is_ok());
        let mut damaged_file = fs::read(&path).unwrap();
        damaged_file.truncate(CACHE_HEADER_LENGTH);
        damaged_file.extend_from_slice(&damaged_payload);
        fs::write(&path, damaged_file).unwrap();
        assert!(read(&directory, "app").is_none());

        let restored = check_project(root.path(), &options).unwrap();
        assert_eq!(restored.stats.checked, 1);
        assert_eq!(restored.stats.disk_hits, 0);
        assert_eq!(restored.stats.cache_writes, 1);
        let Statement::Expression(TypedExpr::Int { value, .. }) =
            &restored.module("app").unwrap().definitions.functions[0].body[0]
        else {
            panic!("rechecked main returns an integer")
        };
        assert_eq!(value, "42");
        assert_eq!(
            check_project(root.path(), &options)
                .unwrap()
                .stats
                .disk_hits,
            1
        );

        // Older raw-bitcode files must also be treated as cache misses.
        fs::write(path, damaged_payload).unwrap();
        assert!(read(&directory, "app").is_none());
    }
}
