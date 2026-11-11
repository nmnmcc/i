//! A tool for refactors that must not change the generated code: with
//! `WAT_DUMP=<dir>`, writes the WAT of std, bench, examples/hello, and every
//! case of `tests/spec`, with and without tests, for both targets, into
//! `<dir>`. Run it before and after a change and compare the two
//! directories with `diff -r`. Without `WAT_DUMP` it does nothing.
//!
//! ```sh
//! WAT_DUMP=/tmp/before cargo test -p ilang-cli --test wat_dump
//! ```
use std::path::{Path, PathBuf};

#[path = "common/single_file.rs"]
mod single_file;

#[path = "common/spec_entry.rs"]
mod spec_entry;

fn dump(project: &Path, name: &str, out: &Path) {
    for (target, tag) in [
        (ilang_wasm::Target::Ilang, "ilang"),
        (ilang_wasm::Target::Wasi, "wasi"),
    ] {
        for tests in [true, false] {
            let (mut graph, _) = ilang_cli::load(project, tests, true);
            graph.entry = spec_entry::named_entry(project, tests);
            let checked = ilang_check::check(&graph);
            let (Some(p), Some(t)) = (&checked.program, &checked.types) else {
                continue;
            };
            let text = match ilang_wasm::compile_wat(
                &graph,
                p,
                t,
                ilang_wasm::Options { tests, target },
            ) {
                Ok(text) => text,
                Err(e) => format!("ERROR {e}"),
            };
            let file = out.join(format!("{}.{tag}.{tests}.wat", name.replace('/', "_")));
            std::fs::write(file, text).unwrap();
        }
    }
}

#[test]
fn wat_dump() {
    let Ok(out) = std::env::var("WAT_DUMP") else {
        return;
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let filter = std::env::var("WAT_FILTER").ok();
    for p in ["std", "bench", "examples/hello"] {
        if filter.as_ref().is_none_or(|f| p.contains(f)) {
            dump(&root.join(p), p, &out);
        }
    }
    let spec = root.join("tests/spec");
    let mut sections: Vec<_> = std::fs::read_dir(&spec)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    sections.sort();
    for section in sections {
        let mut entries: Vec<_> = std::fs::read_dir(&section)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            let name = path
                .strip_prefix(&spec)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if filter.as_ref().is_some_and(|f| !name.contains(f)) {
                continue;
            }
            if path.is_dir() {
                dump(&path, &name, &out);
            } else if path.extension().is_some_and(|e| e == "i") {
                let dir = tempfile::tempdir().unwrap();
                std::fs::write(dir.path().join("i.toml"), "name = \"spec\"\n").unwrap();

                let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
                single_file::write(dir.path(), &stem, &std::fs::read_to_string(&path).unwrap());
                dump(dir.path(), &name, &out);
            }
        }
    }
}
