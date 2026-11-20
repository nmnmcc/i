//! The package hash that each host binding carries. The prelude and the
//! standard library `i` (a dependency, `stdlib/`) are one trusted package,
//! the language: their bindings carry `PackageGraph::language_hash`, the
//! hash that the macro runner and `i run` grant. A binding declared in any
//! other package carries that package's own content hash.

use ilang_package::lock::{FileKind, PackageFile, hash};
use ilang_wasm::{Options, compile_with_bindings};

const MAIN: &str = "(\n\t(Io) = i/io\n)\n\n@external(\"app:math/v1\", \"sqrt\")\nsqrt : (Float) -> Float\n\nmain = () <- (io : Io) { sqrt(2.0)  io.print(\"hi\") }\n";

#[test]
fn library_bindings_carry_the_language_hash_and_others_their_own() {
    let stdlib = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../stdlib")
        .canonicalize()
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let manifest = format!(
        "name = \"app\"\n\n[dependencies]\ni = {{ path = {:?} }}\n",
        stdlib.display().to_string()
    );
    std::fs::write(dir.path().join("i.toml"), &manifest).unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), MAIN).unwrap();
    let (graph, diags) = ilang_package::load(dir.path());
    assert!(diags.is_empty(), "{:?}", graph.sources.render_all(&diags));
    let checked = ilang_check::check(&graph);
    let errors: Vec<_> = checked.diags.iter().filter(|d| d.is_error()).collect();
    assert!(
        errors.is_empty(),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let (_, bindings) = compile_with_bindings(
        &graph,
        checked.program.as_ref().unwrap(),
        checked.types.as_ref().unwrap(),
        Options::default(),
    )
    .unwrap();
    let binding = |namespace: &str, name: &str| {
        bindings
            .iter()
            .find(|b| b["namespace"] == namespace && b["name"] == name)
            .unwrap_or_else(|| panic!("no binding {namespace} {name}: {bindings:#?}"))
    };

    // `Io.print`, declared in `i/io`: the language hash, the same value the
    // CLI and the macro runner grant (prelude + `i`), not `i`'s own hash.
    let print = binding("i:io/v1", "print");
    assert_eq!(print["module"], "i/io");
    let language = graph.language_hash();
    assert_eq!(print["package"], language.as_str());
    let library = graph.standard_library().unwrap();
    let own_library_hash = {
        let p = &graph.packages[library];
        let mut files: Vec<_> = p
            .modules
            .iter()
            .map(|m| PackageFile {
                path: m.path.clone(),
                kind: FileKind::Regular,
                bytes: graph.sources.get(m.source).text.as_bytes().to_vec(),
            })
            .collect();
        files.push(PackageFile {
            path: "i.toml".into(),
            kind: FileKind::Regular,
            bytes: graph
                .sources
                .get(p.manifest_source.unwrap())
                .text
                .as_bytes()
                .to_vec(),
        });
        hash(&files)
    };
    assert_ne!(print["package"], own_library_hash.as_str());
    // Every binding of the prelude or `i` carries it.
    for b in &bindings {
        let module = b["module"].as_str().unwrap();
        if module.starts_with("i/") || module == ilang_package::library::PRELUDE_PATH {
            assert_eq!(b["package"], language.as_str(), "{b:#}");
        }
    }

    // `sqrt`, declared in `app/main`: the hash of `app` itself, its modules
    // and its `i.toml`, unchanged.
    let sqrt = binding("app:math/v1", "sqrt");
    assert_eq!(sqrt["module"], "app/main");
    let root = &graph.packages[graph.root.unwrap()];
    let mut files: Vec<_> = root
        .modules
        .iter()
        .map(|m| PackageFile {
            path: m.path.clone(),
            kind: FileKind::Regular,
            bytes: graph.sources.get(m.source).text.as_bytes().to_vec(),
        })
        .collect();
    files.push(PackageFile {
        path: "i.toml".into(),
        kind: FileKind::Regular,
        bytes: manifest.into_bytes(),
    });
    let app_hash = hash(&files);
    assert_eq!(sqrt["package"], app_hash.as_str());
    assert_ne!(app_hash, language);
}
