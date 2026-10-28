//! Best-effort, content-addressed storage for reachable module definitions.
//!
//! Cache I/O never prevents compilation. A missing, stale, unreadable or damaged
//! entry is a miss, and each replacement becomes visible with one rename.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use gleam_core::ast::TypedModule;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    Error,
    ir::{Constructor, Expr, Function},
};

#[path = "canonical.rs"]
mod canonical;

const SCHEMA: &str = "wleam-module-ir-v3:wasm-gc-shared-value-abi-v1";
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) enum Definition {
    Function(Function),
    Constant(Expr),
}

#[derive(Debug, Serialize, Deserialize)]
struct Entry {
    schema: String,
    key: String,
    checksum: String,
    definitions: BTreeMap<String, Definition>,
}

struct Loaded {
    path: PathBuf,
    entry: Entry,
    pending: BTreeSet<String>,
}

pub(super) struct Cache {
    directory: PathBuf,
    context: [u8; 32],
    modules: BTreeMap<String, Loaded>,
}

impl Cache {
    pub(super) fn new(
        directory: &Path,
        compiler_fingerprint: &str,
        registry: &BTreeMap<String, Constructor>,
        external_types: &BTreeSet<String>,
    ) -> Result<Self, Error> {
        let context = Sha256::digest(canonical_bytes(&(
            SCHEMA,
            compiler_fingerprint,
            registry,
            external_types,
        ))?)
        .into();
        Ok(Self {
            directory: directory.into(),
            context,
            modules: BTreeMap::new(),
        })
    }

    pub(super) fn get(
        &mut self,
        module: &TypedModule,
        symbol: &str,
    ) -> Result<Option<Definition>, Error> {
        Ok(self.load(module)?.entry.definitions.get(symbol).cloned())
    }

    pub(super) fn insert(
        &mut self,
        module: &TypedModule,
        symbol: String,
        definition: Definition,
    ) -> Result<(), Error> {
        let loaded = self.load(module)?;
        loaded.pending.insert(symbol.clone());
        loaded.entry.definitions.insert(symbol, definition);
        Ok(())
    }

    /// Count definitions whose containing module was successfully replaced.
    /// Failed writes remain pending and can be retried by a later flush.
    pub(super) fn flush(&mut self) -> usize {
        if self
            .modules
            .values()
            .all(|module| module.pending.is_empty())
        {
            return 0;
        }
        if fs::create_dir_all(&self.directory).is_err() {
            return 0;
        }
        let mut written = 0;
        for module in self.modules.values_mut() {
            if module.pending.is_empty() {
                continue;
            }
            let Ok(checksum) = definitions_checksum(&module.entry.definitions) else {
                continue;
            };
            module.entry.checksum = checksum;
            let Ok(bytes) = bitcode::serialize(&module.entry) else {
                continue;
            };
            if atomic_write(&module.path, &bytes).is_ok() {
                written += module.pending.len();
                module.pending.clear();
            }
        }
        written
    }

    fn load(&mut self, module: &TypedModule) -> Result<&mut Loaded, Error> {
        let name = module.name.as_str();
        if !self.modules.contains_key(name) {
            let key = module_key(&self.context, module)?;
            let path = entry_path(&self.directory, name);
            let entry = fs::read(&path)
                .ok()
                .and_then(|bytes| bitcode::deserialize::<Entry>(&bytes).ok())
                .filter(|entry| {
                    entry.schema == SCHEMA
                        && entry.key == key
                        && definitions_checksum(&entry.definitions)
                            .is_ok_and(|checksum| checksum == entry.checksum)
                })
                .unwrap_or_else(|| Entry {
                    schema: SCHEMA.into(),
                    key,
                    checksum: String::new(),
                    definitions: BTreeMap::new(),
                });
            self.modules.insert(
                name.into(),
                Loaded {
                    path,
                    entry,
                    pending: BTreeSet::new(),
                },
            );
        }
        Ok(self.modules.get_mut(name).expect("module inserted above"))
    }
}

fn definitions_checksum(definitions: &BTreeMap<String, Definition>) -> Result<String, Error> {
    Ok(format!(
        "{:x}",
        Sha256::digest(canonical_bytes(definitions)?)
    ))
}

fn module_key(context: &[u8; 32], module: &TypedModule) -> Result<String, Error> {
    // Names and References are editor indexes, never inputs to lowering.
    // Warnings and unused
    // source-position sets likewise have no influence on emitted definitions.
    // Retain the complete typed definitions and their source spans.
    let mut interface = module.type_info.clone();
    interface.references = Default::default();
    interface.warnings.clear();
    let bytes = canonical_bytes(&(
        &module.name,
        &module.documentation,
        &module.definitions,
        &interface,
    ))
    .map_err(|error| Error::Lowering(format!("module {}: {error}", module.name)))?;
    let mut hash = Sha256::new();
    hash.update(context);
    hash.update(bytes);
    Ok(format!("{:x}", hash.finalize()))
}

fn entry_path(directory: &Path, name: &str) -> PathBuf {
    directory.join(format!("{:x}.bin", Sha256::digest(name.as_bytes())))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = path.with_extension(format!(
        "{}-{}.tmp",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed),
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn canonical_bytes(value: &impl Serialize) -> Result<Vec<u8>, Error> {
    canonical::bytes(value)
        .map_err(|error| Error::Lowering(format!("cannot fingerprint module IR: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, sync::Arc};

    fn module() -> (tempfile::TempDir, Arc<TypedModule>) {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("src")).unwrap();
        fs::write(
            root.path().join("gleam.toml"),
            "name = \"app\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        fs::write(
            root.path().join("src/app.gleam"),
            r#"
pub type Boxed(a) { Boxed(value: a) }
pub fn main() { Boxed(42) }
pub fn branch(value) {
  case value {
    Boxed(1) -> "one"
    Boxed(2) -> "two"
    Boxed(_) -> "other"
  }
}
pub fn bits(value: BitArray) {
  case value {
    <<size, part:size(size), _rest:bits>> -> part
    _ -> 0
  }
}
"#,
        )
        .unwrap();
        let project = wleam_frontend::check_project(root.path(), &Default::default()).unwrap();
        (root, Arc::clone(&project.modules[0]))
    }

    #[test]
    fn canonical_maps_ignore_hashmap_iteration_order_at_every_depth() {
        let first = HashMap::from([
            ("z", HashMap::from([("b", 2), ("a", 1)])),
            ("a", HashMap::from([("d", 4), ("c", 3)])),
        ]);
        let second = HashMap::from([
            ("a", HashMap::from([("c", 3), ("d", 4)])),
            ("z", HashMap::from([("a", 1), ("b", 2)])),
        ]);
        assert_eq!(
            canonical_bytes(&first).unwrap(),
            canonical_bytes(&second).unwrap()
        );
    }

    #[test]
    fn corrupted_entries_miss_and_are_replaced_atomically() {
        let (root, module) = module();
        let directory = root.path().join("ir");
        fs::create_dir(&directory).unwrap();
        let path = entry_path(&directory, module.name.as_str());
        fs::write(&path, b"{ incomplete").unwrap();
        let mut cache =
            Cache::new(&directory, "compiler-1", &BTreeMap::new(), &BTreeSet::new()).unwrap();
        assert!(cache.get(&module, "app.answer").unwrap().is_none());
        cache
            .insert(
                &module,
                "app.answer".into(),
                Definition::Constant(Expr::Int(42.into())),
            )
            .unwrap();
        assert_eq!(cache.flush(), 1);
        assert_eq!(cache.flush(), 0);
        let mut cache =
            Cache::new(&directory, "compiler-1", &BTreeMap::new(), &BTreeSet::new()).unwrap();
        assert!(
            matches!(cache.get(&module, "app.answer").unwrap(), Some(Definition::Constant(Expr::Int(value))) if value == 42.into())
        );
        assert_eq!(fs::read_dir(directory).unwrap().count(), 1);
    }

    #[test]
    fn structurally_valid_payload_damage_is_a_cache_miss() {
        let (root, module) = module();
        let directory = root.path().join("ir");
        let mut cache =
            Cache::new(&directory, "compiler-1", &BTreeMap::new(), &BTreeSet::new()).unwrap();
        cache
            .insert(
                &module,
                "app.answer".into(),
                Definition::Constant(Expr::Int(42.into())),
            )
            .unwrap();
        assert_eq!(cache.flush(), 1);
        let path = entry_path(&directory, module.name.as_str());
        let mut entry: Entry = bitcode::deserialize(&fs::read(&path).unwrap()).unwrap();
        entry.definitions.insert(
            "app.answer".into(),
            Definition::Constant(Expr::Int(43.into())),
        );
        fs::write(&path, bitcode::serialize(&entry).unwrap()).unwrap();
        let mut cache =
            Cache::new(&directory, "compiler-1", &BTreeMap::new(), &BTreeSet::new()).unwrap();
        assert!(cache.get(&module, "app.answer").unwrap().is_none());
    }

    #[test]
    fn binary_payload_preserves_float_bits_and_deep_ir() {
        let (root, module) = module();
        let directory = root.path().join("ir");
        let bits = 0x3fd5_5555_5555_5556_u64;
        let mut value = Expr::Float(f64::from_bits(bits));
        for _ in 0..160 {
            value = Expr::Tuple(vec![value]);
        }
        let mut cache =
            Cache::new(&directory, "compiler-1", &BTreeMap::new(), &BTreeSet::new()).unwrap();
        cache
            .insert(&module, "app.deep".into(), Definition::Constant(value))
            .unwrap();
        assert_eq!(cache.flush(), 1);
        let mut cache =
            Cache::new(&directory, "compiler-1", &BTreeMap::new(), &BTreeSet::new()).unwrap();
        let Some(Definition::Constant(mut value)) = cache.get(&module, "app.deep").unwrap() else {
            panic!("binary IR payload should load regardless of JSON nesting limits");
        };
        for _ in 0..160 {
            let Expr::Tuple(mut elements) = value else {
                panic!("expected nested tuple")
            };
            value = elements.pop().unwrap();
        }
        assert!(matches!(value, Expr::Float(value) if value.to_bits() == bits));
    }

    #[test]
    fn layout_compiler_and_external_type_changes_invalidate_definitions() {
        let (root, module) = module();
        let directory = root.path().join("ir");
        let mut registry = BTreeMap::from([(
            "app.Boxed".into(),
            Constructor {
                symbol: "app.Boxed".into(),
                name: "Boxed".into(),
                type_name: "app.Boxed".into(),
                tag: 32,
                type_id: 32,
                arity: 1,
            },
        )]);
        let mut cache = Cache::new(&directory, "compiler-1", &registry, &BTreeSet::new()).unwrap();
        cache
            .insert(
                &module,
                "app.answer".into(),
                Definition::Constant(Expr::Nil),
            )
            .unwrap();
        assert_eq!(cache.flush(), 1);

        let mut compiler =
            Cache::new(&directory, "compiler-2", &registry, &BTreeSet::new()).unwrap();
        assert!(compiler.get(&module, "app.answer").unwrap().is_none());
        let mut external = Cache::new(
            &directory,
            "compiler-1",
            &registry,
            &BTreeSet::from(["app.Handle".into()]),
        )
        .unwrap();
        assert!(external.get(&module, "app.answer").unwrap().is_none());
        registry.get_mut("app.Boxed").unwrap().tag = 33;
        let mut layout = Cache::new(&directory, "compiler-1", &registry, &BTreeSet::new()).unwrap();
        assert!(layout.get(&module, "app.answer").unwrap().is_none());
    }

    #[test]
    fn unavailable_cache_directory_does_not_prevent_in_memory_use() {
        let (root, module) = module();
        let directory = root.path().join("not-a-directory");
        fs::write(&directory, "occupied").unwrap();
        let mut cache =
            Cache::new(&directory, "compiler-1", &BTreeMap::new(), &BTreeSet::new()).unwrap();
        assert!(cache.get(&module, "app.answer").unwrap().is_none());
        cache
            .insert(
                &module,
                "app.answer".into(),
                Definition::Constant(Expr::Nil),
            )
            .unwrap();
        assert!(cache.get(&module, "app.answer").unwrap().is_some());
        assert_eq!(cache.flush(), 0);
    }
}
