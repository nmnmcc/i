//! The library modules and the prelude parse and resolve with no problems,
//! and they have the names and labels of the spec tables.

mod common;

use ilang_check::hir::{DefKind, TypeBody, TypeKind};

#[test]
fn library_resolves_cleanly() {
    let (graph, program, diags) = common::check_files(&[]);
    let rendered = graph.sources.render_all(&diags);
    assert!(rendered.is_empty(), "{rendered:#?}");
    let program = program.unwrap();
    let paths: Vec<&str> = program.modules.iter().map(|m| m.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "i/prelude",
            "i/fail",
            "i/control",
            "i/resource",
            "i/assertion",
            "i/request",
            "i/key",
            "i/core",
            "i/record",
            "i/io",
            "i/task",
            "i/clock",
            "i/random",
            "i/syntax",
            "i/fs",
            "i/process",
            "i/net",
            "i/tls",
            "i/stdio"
        ]
    );
}

/// Every registered module path has its own source in the package `i`,
/// with the text of the list.
#[test]
fn library_modules_have_sources() {
    let (graph, _, _) = common::check_files(&[]);
    let package = &graph.packages[graph.library];
    let registered = std::iter::once(ilang_package::library::PRELUDE)
        .chain(ilang_package::library::MODULES.iter().copied());
    let mut count = 0;
    for (path, text) in registered {
        let module = package
            .modules
            .iter()
            .find(|m| m.path == path)
            .unwrap_or_else(|| panic!("{path} has no module"));
        assert_eq!(graph.sources.get(module.source).text, text, "{path}");
        count += 1;
    }
    assert_eq!(package.modules.len(), count);
}

/// The exported names of each library module, as in §9.3–§9.6, §9.10 and §13.
#[test]
fn library_names() {
    let (_, program, _) = common::check_files(&[]);
    let program = program.unwrap();
    let expect: &[(&str, &[&str])] = &[
        ("i/fail", &["raise", "catch", "from_result", "map_error"]),
        (
            "i/control",
            &["boundary", "suspend", "resume", "cancel", "keep_env"],
        ),
        ("i/key", &["new", "wrap", "unwrap"]),
        (
            "i/request",
            &[
                "open_root",
                "close_root",
                "open_owner",
                "close_owner",
                "count",
                "count_root",
                "settled_owner",
                "settled_root",
                "cancel_all",
                "cancel_all_root",
                "wait_none",
                "wait_none_root",
                "within",
                "ready",
                "ticket",
                "state",
                "take",
                "settled",
                "cancel",
                "await",
                "await_owned",
                "wait_any",
                "poll",
            ],
        ),
        ("i/resource", &["ensure", "acquire"]),
        ("i/assertion", &["catch"]),
        (
            "i/core",
            &[
                "not",
                "to_float",
                "truncate",
                "floor",
                "ceiling",
                "sqrt",
                "float_to_bits",
                "float_from_bits",
                "bit_and",
                "bit_or",
                "bit_xor",
                "bit_not",
                "shift_left",
                "shift_right",
                "concat",
                "byte_size",
                "byte_slice",
                "codepoints",
                "from_codepoints",
                "codepoint_to_int",
                "int_to_codepoint",
                "bit_size",
                "bit_array_to_string",
                "to_upper",
                "to_lower",
                "index_of",
                "bit_array_concat",
                "inspect",
                "hash",
                "portable",
            ],
        ),
        ("i/prelude", &["panic", "assert"]),
        ("i/io", &[]),
        (
            "i/task",
            &[
                "top",
                "run",
                "test",
                "scope",
                "local_scope",
                "go",
                "join",
                "yield",
                "path",
                "random",
                "keep",
                "channel",
                "local_channel",
                "send",
                "receive",
                "send_event",
                "receive_event",
                "join_event",
                "always",
                "never",
                "choose",
                "map",
                "sync",
                "share",
                "get",
                "both",
                "race",
                "map_concurrently",
            ],
        ),
        ("i/clock", &["monotonic_ns", "unix_ns", "sleep_ns"]),
        (
            "i/random",
            &["from_seed", "word", "int", "float", "split", "at_path"],
        ),
        (
            "i/fs",
            &[
                "open",
                "read",
                "write",
                "flush",
                "close",
                "remove",
                "rename",
                "mkdir",
                "list",
                "metadata",
                "temp_dir",
                "read_file",
                "write_file",
                "read_lines",
            ],
        ),
        (
            "i/process",
            &[
                "args",
                "env",
                "cwd",
                "entropy",
                "spawn",
                "spawn_piped",
                "spawn_config",
                "read_stdout",
                "read_stderr",
                "write_stdin",
                "close_stdin",
                "wait",
                "terminate",
                "close",
            ],
        ),
        (
            "i/net",
            &[
                "listen",
                "local_port",
                "accept",
                "connect",
                "resolve",
                "read",
                "write",
                "shutdown_write",
                "close_stream",
                "close_listener",
                "stream_number",
                "byte_stream",
            ],
        ),
        (
            "i/tls",
            &[
                "options",
                "identity",
                "connect",
                "accept",
                "dial",
                "alpn",
                "read",
                "write",
                "shutdown_write",
                "close",
                "byte_stream",
            ],
        ),
        (
            "i/stdio",
            &[
                "read",
                "write",
                "write_error",
                "flush",
                "flush_error",
                "print",
                "print_error",
            ],
        ),
    ];
    for (path, names) in expect {
        let m = program.module(common::module(&program, path));
        let mut exported: Vec<&str> = m
            .defs
            .iter()
            .filter(|d| d.exported)
            .map(|d| d.name.as_str())
            .collect();
        let mut want = names.to_vec();
        exported.sort();
        want.sort();
        assert_eq!(exported, want, "{path}");
    }
    // Every primitive of `i/core` is an external of `ilang:core` with its
    // own name.
    for d in &program.module(common::module(&program, "i/core")).defs {
        let DefKind::External {
            module, function, ..
        } = &d.kind
        else {
            panic!("{} is not external", d.name)
        };
        assert_eq!(
            (module.as_str(), function.as_str()),
            ("ilang:core", d.name.as_str())
        );
    }
    // The functions that the spec writes in i are bindings.
    for (path, name) in [
        ("i/assertion", "catch"),
        ("i/resource", "acquire"),
        ("i/fail", "from_result"),
        ("i/fail", "map_error"),
        ("i/prelude", "assert"),
        ("i/random", "from_seed"),
        ("i/random", "word"),
        ("i/random", "int"),
        ("i/random", "float"),
        ("i/random", "split"),
        ("i/random", "at_path"),
    ] {
        let d = common::def(&program, path, name);
        assert!(matches!(d.kind, DefKind::Binding { .. }), "{path}.{name}");
    }
}

/// The labels of the parameters in the spec tables, for example
/// `control.suspend: (b: …, f: (k: …) -> …) -> A`.
#[test]
fn library_labels() {
    let (_, program, _) = common::check_files(&[]);
    let program = program.unwrap();
    let labels = |path: &str, name: &str| -> Vec<String> {
        let DefKind::External { ty, .. } = &common::def(&program, path, name).kind else {
            panic!()
        };
        let TypeKind::Fn { params, .. } = &ty.kind else {
            panic!()
        };
        params
            .iter()
            .map(|p| p.label.as_ref().map_or("_".into(), |l| l.name.clone()))
            .collect()
    };
    assert_eq!(labels("i/control", "suspend"), ["b", "f"]);
    assert_eq!(labels("i/control", "resume"), ["k", "value"]);
    assert_eq!(labels("i/fail", "catch"), ["body"]);
    assert_eq!(labels("i/resource", "ensure"), ["body", "with"]);
    assert_eq!(labels("i/core", "byte_slice"), ["_", "start", "length"]);
    assert_eq!(labels("i/core", "index_of"), ["in", "find", "from"]);
    assert_eq!(labels("i/prelude", "panic"), ["message"]);
}

/// The built-in custom types of §4.2 and §9.6 are in the prelude.
#[test]
fn prelude_types() {
    let (_, program, _) = common::check_files(&[]);
    let program = program.unwrap();
    let prelude = program.prelude.unwrap();
    let mut found: Vec<(String, Vec<String>)> = program
        .types
        .iter()
        .filter(|t| t.module == prelude)
        .map(|t| {
            let TypeBody::Custom(cs) = &t.body else {
                panic!()
            };
            (t.name.clone(), cs.iter().map(|c| c.name.clone()).collect())
        })
        .collect();
    found.sort();
    let want = [
        ("Assert", vec!["Assert"]),
        ("AssertionFailure", vec!["AssertionFailure"]),
        ("Bool", vec!["True", "False"]),
        ("Nil", vec!["Nil"]),
        ("Result", vec!["Ok", "Error"]),
    ];
    let want: Vec<(String, Vec<String>)> = want
        .iter()
        .map(|(t, cs)| (t.to_string(), cs.iter().map(|c| c.to_string()).collect()))
        .collect();
    assert_eq!(found, want);
}

#[test]
fn hello_example_checks() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello");
    let (graph, mut diags) = ilang_package::load(&dir);
    let checked = ilang_check::check(&graph);
    diags.extend(checked.diags);
    assert!(diags.is_empty(), "{:#?}", graph.sources.render_all(&diags));
    assert!(checked.program.is_some() && checked.types.is_some());
}

const RESERVED: [&str; 6] = ["i/clock", "i/fs", "i/process", "i/net", "i/tls", "i/stdio"];

/// User code imports every reserved module with no problems.
///
/// These standard paths are registered in the library (§0, §2.1).
#[test]
fn reserved_modules_import_cleanly() {
    let imports: String = RESERVED.iter().map(|p| format!("  {p}\n")).collect();
    let main = format!("(\n{imports})\n\nx = 1\n");
    assert!(ilang_syntax::parse_module(&main).1.is_empty());
    let (_, program, diags) = common::check_files(&[("src/main.i", &main)]);
    common::expect_unused(&diags, &["x"]);
    assert!(program.is_some());
}

/// A name imported from a reserved module, or used through its alias, is
/// reported as missing from that module.
#[test]
fn reserved_modules_have_no_names() {
    let imports: String = RESERVED
        .iter()
        .map(|p| format!("  (nothing) = {p}\n"))
        .collect();
    let main = format!("(\n{imports})\n\nx = 1\n");
    assert!(ilang_syntax::parse_module(&main).1.is_empty());
    let (graph, _, diags) = common::check_files(&[("src/main.i", &main)]);
    let rendered = graph.sources.render_all(&diags);
    let want: Vec<String> = RESERVED
        .iter()
        .enumerate()
        .map(|(i, p)| {
            format!(
                "src/main.i:{}:4: error[E0203]: `{p}` has no `nothing`",
                i + 2
            )
        })
        .collect();
    assert_eq!(rendered, want);

    let main = " (\n  i/task\n)\n\nx = task.spawn\n";
    let (graph, _, diags) = common::check_files(&[("src/main.i", main)]);
    let rendered = graph.sources.render_all(&diags);
    assert_eq!(
        rendered,
        ["src/main.i:5:10: error[E0203]: `i/task` has no `spawn`"]
    );
}

/// Only the registered paths are modules of `i`. Any other path in `i` is
/// still a missing module, as §2.1 says, so the reserved modules do not
/// open the whole of `i` to user imports.
#[test]
fn unknown_library_path_is_missing() {
    for path in ["i/nope", "i/tasks", "i/std", "i/fs/file"] {
        let main = format!("({path})\n\nx = 1\n");
        assert!(ilang_syntax::parse_module(&main).1.is_empty());
        let (graph, _, diags) = common::check_files(&[("src/main.i", &main)]);
        let rendered = graph.sources.render_all(&diags);
        assert_eq!(
            rendered,
            [format!(
                "src/main.i:1:2: error[E0210]: there is no module `{path}`"
            )],
            "{path}"
        );
    }
}
