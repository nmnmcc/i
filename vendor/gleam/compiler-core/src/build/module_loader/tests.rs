// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 The Gleam contributors

use src_span::LineNumbers;

use super::*;
use crate::{
    build::SourceFingerprint,
    io::{FileSystemWriter, memory::InMemoryFileSystem},
};
use std::time::Duration;

#[test]
fn no_cache_present() {
    let name = "package".into();
    let artefact = Utf8Path::new("/artefact");
    let fs = InMemoryFileSystem::new();
    let warnings = WarningEmitter::null();
    let incomplete_modules = HashSet::new();
    let loader = make_loader(&warnings, &name, &fs, artefact, &incomplete_modules);

    fs.write(&Utf8Path::new("/src/main.gleam"), "const x = 1")
        .unwrap();

    let file = GleamFile::new("/src".into(), "/src/main.gleam".into());
    let result = loader.load(file).unwrap();

    assert!(result.is_new());
}

#[test]
fn cache_present_and_fresh() {
    let name = "package".into();
    let artefact = Utf8Path::new("/artefact");
    let fs = InMemoryFileSystem::new();
    let warnings = WarningEmitter::null();
    let incomplete_modules = HashSet::new();
    let loader = make_loader(&warnings, &name, &fs, artefact, &incomplete_modules);

    // The mtime of the source is older than that of the cache
    write_src(&fs, TEST_SOURCE_1, "/src/main.gleam", 0);
    write_cache(&fs, TEST_SOURCE_1, "/artefact/main.cache_meta", 1);

    let file = GleamFile::new("/src".into(), "/src/main.gleam".into());
    let result = loader.load(file).unwrap();

    assert!(result.is_cached());
}

#[test]
fn cache_present_and_stale() {
    let name = "package".into();
    let artefact = Utf8Path::new("/artefact");
    let fs = InMemoryFileSystem::new();
    let warnings = WarningEmitter::null();
    let incomplete_modules = HashSet::new();
    let loader = make_loader(&warnings, &name, &fs, artefact, &incomplete_modules);

    // The mtime of the source is newer than that of the cache
    write_src(&fs, TEST_SOURCE_2, "/src/main.gleam", 2);
    write_cache(&fs, TEST_SOURCE_1, "/artefact/main.cache_meta", 1);

    let file = GleamFile::new("/src".into(), "/src/main.gleam".into());
    let result = loader.load(file).unwrap();

    assert!(result.is_new());
}

#[test]
fn changed_source_invalidates_cache_with_equal_or_older_timestamp() {
    for source_timestamp in [0, 1] {
        let name = "package".into();
        let artefact = Utf8Path::new("/artefact");
        let fs = InMemoryFileSystem::new();
        let warnings = WarningEmitter::null();
        let incomplete_modules = HashSet::new();
        let loader = make_loader(&warnings, &name, &fs, artefact, &incomplete_modules);

        write_src(&fs, TEST_SOURCE_2, "/src/main.gleam", source_timestamp);
        write_cache(&fs, TEST_SOURCE_1, "/artefact/main.cache_meta", 1);

        let file = GleamFile::new("/src".into(), "/src/main.gleam".into());
        assert!(loader.load(file).unwrap().is_new());
    }
}

#[test]
fn incomplete_lsp_module_rechecks_with_unchanged_source_and_timestamp() {
    let name = "package".into();
    let artefact = Utf8Path::new("/artefact");
    let fs = InMemoryFileSystem::new();
    let warnings = WarningEmitter::null();
    let incomplete_modules = HashSet::from(["main".into()]);
    let mut loader = make_loader(&warnings, &name, &fs, artefact, &incomplete_modules);
    loader.mode = Mode::Lsp;

    write_src(&fs, TEST_SOURCE_1, "/src/main.gleam", 1);
    write_cache(&fs, TEST_SOURCE_1, "/artefact/main.cache_meta", 1);

    let file = GleamFile::new("/src".into(), "/src/main.gleam".into());
    assert!(loader.load(file).unwrap().is_new());
}

#[test]
fn cache_present_and_stale_but_source_is_the_same() {
    let name = "package".into();
    let artefact = Utf8Path::new("/artefact");
    let fs = InMemoryFileSystem::new();
    let warnings = WarningEmitter::null();
    let incomplete_modules = HashSet::new();
    let loader = make_loader(&warnings, &name, &fs, artefact, &incomplete_modules);

    // The mtime of the source is newer than that of the cache
    write_src(&fs, TEST_SOURCE_1, "/src/main.gleam", 2);
    write_cache(&fs, TEST_SOURCE_1, "/artefact/main.cache_meta", 1);

    let file = GleamFile::new("/src".into(), "/src/main.gleam".into());
    let result = loader.load(file).unwrap();

    assert!(result.is_cached());
}

#[test]
fn cache_present_and_stale_source_is_the_same_lsp_mode() {
    let name = "package".into();
    let artefact = Utf8Path::new("/artefact");
    let fs = InMemoryFileSystem::new();
    let warnings = WarningEmitter::null();
    let incomplete_modules = HashSet::new();
    let mut loader = make_loader(&warnings, &name, &fs, artefact, &incomplete_modules);
    loader.mode = Mode::Lsp;

    // The mtime of the source is newer than that of the cache
    write_src(&fs, TEST_SOURCE_1, "/src/main.gleam", 2);
    write_cache(&fs, TEST_SOURCE_1, "/artefact/main.cache_meta", 1);

    let file = GleamFile::new("/src".into(), "/src/main.gleam".into());
    let result = loader.load(file).unwrap();

    assert!(result.is_cached());
}

#[test]
fn cache_present_and_stale_source_is_the_same_lsp_mode_and_invalidated() {
    let name = "package".into();
    let artefact = Utf8Path::new("/artefact");
    let fs = InMemoryFileSystem::new();
    let warnings = WarningEmitter::null();
    let mut incomplete_modules = HashSet::new();
    let _ = incomplete_modules.insert("main".into());
    let mut loader = make_loader(&warnings, &name, &fs, artefact, &incomplete_modules);
    loader.mode = Mode::Lsp;

    // The mtime of the source is newer than that of the cache
    write_src(&fs, TEST_SOURCE_1, "/src/main.gleam", 2);
    write_cache(&fs, TEST_SOURCE_1, "/artefact/main.cache_meta", 1);

    let file = GleamFile::new("/src".into(), "/src/main.gleam".into());
    let result = loader.load(file).unwrap();

    assert!(result.is_new());
}

const TEST_SOURCE_1: &'static str = "const x = 1";
const TEST_SOURCE_2: &'static str = "const x = 2";

fn write_cache(fs: &InMemoryFileSystem, source: &str, path: &str, seconds: u64) {
    let line_numbers = LineNumbers::new(source);
    let cache_metadata = CacheMetadata {
        mtime: SystemTime::UNIX_EPOCH + Duration::from_secs(seconds),
        dependencies: vec![],
        fingerprint: SourceFingerprint::new(source),
        line_numbers,
    };
    let path = Utf8Path::new(path);
    fs.write_bytes(&path, &cache_metadata.to_binary()).unwrap();
}

fn write_src(fs: &InMemoryFileSystem, source: &str, path: &str, seconds: u64) {
    let path = Utf8Path::new(path);
    fs.write(&path, source).unwrap();
    fs.set_modification_time(&path, SystemTime::UNIX_EPOCH + Duration::from_secs(seconds));
}

fn make_loader<'a>(
    warnings: &'a WarningEmitter,
    package_name: &'a EcoString,
    fs: &InMemoryFileSystem,
    artefact: &'a Utf8Path,
    incomplete_modules: &'a HashSet<EcoString>,
) -> ModuleLoader<'a, InMemoryFileSystem> {
    ModuleLoader {
        warnings,
        io: fs.clone(),
        mode: Mode::Dev,
        target: Target::Wasm,
        package_name,
        artefact_directory: &artefact,
        origin: Origin::Src,
        incomplete_modules,
    }
}
