//! Collection handles are tool roots, separate from source entries and exports.
use ilang_cli::{BuildOptions, CollectionResult, Level, Target};
use ilang_host::Outcome;
use std::path::Path;

fn options(level: Level) -> BuildOptions {
    BuildOptions {
        tests: false,
        opt: level,
        offline: true,
        target: Target::Ilang,
    }
}
fn project(path: &Path) {
    std::fs::write(path.join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(path.join("src")).unwrap();
    std::fs::write(path.join("src/macros.is"), "(, (= syntax (/ i syntax)))\n(, emit)\n(@ macro (= emit (-> (c x body (! it (. syntax Items))) _ { ((. it add) c x [] x) `(= ~x (-> () _ { ~body })) })))\n").unwrap();
    std::fs::write(path.join("src/main.i"), "(m = app/macros\nsyntax = i/syntax\ntask = i/task)\n(anchor)\n@external(\"test:unselected\", \"missing\")\nmissing: () -> Nil\nanchor = Nil\nm.emit(anchor, good, assert(True))\nm.emit(anchor, bad, assert(False))\nm.emit(anchor, number, 42)\nm.emit(anchor, compiler_only, syntax.show)\nm.emit(anchor, needs_foreign, missing())\nm.emit(anchor, concurrent, assert(task.both(() { 1 }, () { 2 }) == (1, 2)))\n").unwrap();
}

fn run_javascript(dir: &Path, built: &ilang_cli::BuiltCollection) {
    let path = dir.join("collection.wasm");
    std::fs::write(&path, &built.built.wasm).unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../bindings/javascript/test/collection-runner.mjs");
    let result = std::process::Command::new("node")
        .arg(script)
        .arg(path)
        .arg(&built.selected.handle)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "JavaScript host: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn generated_contributions_are_selected_independently() {
    let dir = tempfile::tempdir().unwrap();
    project(dir.path());
    let manifest = ilang_cli::collections(dir.path(), false, true).unwrap();
    assert_eq!(manifest.entries.len(), 6);
    assert!(
        manifest
            .entries
            .iter()
            .all(|entry| entry.collection == "app/main.anchor")
    );
    let encoded = serde_json::to_vec(&manifest).unwrap();
    assert_eq!(
        serde_json::from_slice::<ilang_cli::CollectionManifest>(&encoded).unwrap(),
        manifest
    );
    for level in [Level::O0, Level::O2] {
        let good = ilang_cli::build_collection(
            dir.path(),
            options(level),
            &manifest.entries[0].handle,
            CollectionResult::Nil,
        )
        .unwrap();
        run_javascript(dir.path(), &good);
        let artifact = ilang_cli::CollectionArtifact::from_wasm(&good.built.wasm)
            .unwrap()
            .unwrap();
        assert_eq!(artifact.selected, manifest.entries[0].handle);
        assert_eq!(artifact.manifest, manifest);
        let program = good.built.host_program().unwrap();
        assert!(program.abi().entry.is_none());
        assert!(program.abi().tests.is_empty());
        assert!(program.abi().exports.is_empty());
        assert!(
            program.abi().externals.is_empty(),
            "an unselected target must not pull foreign imports"
        );
        assert!(matches!(
            good.run_with(good.built.host_options()).unwrap(),
            Outcome::Ok(_)
        ));
        let mut instance = program.instantiate(good.built.host_options()).unwrap();
        assert!(
            instance.run("i.tool.1").is_err(),
            "unselected target must not be exported"
        );
        let concurrent = ilang_cli::build_collection(
            dir.path(),
            options(level),
            &manifest.entries[5].handle,
            CollectionResult::Nil,
        )
        .unwrap();
        run_javascript(dir.path(), &concurrent);
        assert!(concurrent.is_async);
        assert!(matches!(
            concurrent
                .run_with(concurrent.built.host_options())
                .unwrap(),
            Outcome::Ok(_)
        ));
        let bad = ilang_cli::build_collection(
            dir.path(),
            options(level),
            &manifest.entries[1].handle,
            CollectionResult::Nil,
        )
        .unwrap();
        assert!(matches!(
            bad.run_with(bad.built.host_options()).unwrap(),
            Outcome::Stop(_)
        ));
    }
    assert!(
        ilang_cli::build_collection(
            dir.path(),
            options(Level::O0),
            &manifest.entries[2].handle,
            CollectionResult::Nil
        )
        .is_err()
    );
    assert!(
        ilang_cli::build_collection(
            dir.path(),
            options(Level::O0),
            &manifest.entries[2].handle,
            CollectionResult::Any
        )
        .is_ok()
    );
    let reject = |_: &ilang_check::hir::Program, _: &ilang_check::types::ShownType| {
        Err("consumer requires another result".to_owned())
    };
    assert!(matches!(
        ilang_cli::build_collection(
            dir.path(),
            options(Level::O0),
            &manifest.entries[2].handle,
            CollectionResult::Checked(&reject)
        ),
        Err(ilang_cli::BuildError::Selection(_))
    ));
    assert!(
        ilang_cli::build_collection(
            dir.path(),
            options(Level::O0),
            &manifest.entries[3].handle,
            CollectionResult::Any,
        )
        .is_err(),
        "a selected compiler capability must be rejected"
    );
    let foreign = ilang_cli::build_collection(
        dir.path(),
        options(Level::O0),
        &manifest.entries[4].handle,
        CollectionResult::Nil,
    )
    .unwrap();
    assert!(
        !foreign
            .built
            .host_program()
            .unwrap()
            .abi()
            .externals
            .is_empty()
    );
    std::fs::write(dir.path().join("src/main.i"), "(anchor)\nanchor = Nil\n").unwrap();
    assert!(matches!(
        ilang_cli::build_collection(
            dir.path(),
            options(Level::O0),
            &manifest.entries[0].handle,
            CollectionResult::Any
        ),
        Err(ilang_cli::BuildError::Selection(_))
    ));
}
