//! The library modules and the prelude parse and resolve with no problems,
//! and they have the names and labels that the library documents.

mod common;

use ilang_check::hir::{DefKind, TypeBody};

/// The module paths of the standard library `i` (`stdlib/src/`), in the
/// order of its package: by file name.
fn stdlib_paths() -> Vec<String> {
    let src = common::stdlib_dir().join("src");
    let mut names: Vec<String> = std::fs::read_dir(&src)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
        .iter()
        .map(|n| format!("i/{}", n.rsplit_once('.').unwrap().0))
        .collect()
}

#[test]
fn library_resolves_cleanly() {
    let (graph, _, _) = common::check_all_modules(&[]);
    let (parsed, parse_diags) = ilang_check::parse_all(&graph);
    let rendered = graph.sources.render_all(&parse_diags);
    assert!(rendered.is_empty(), "{rendered:#?}");
    // Every module of the package `i` is parsed, including the Lisp macro
    // module `i/test`, whose quotes the macro driver expands before
    // resolution; this checker-only test leaves it out of that pass.
    let mut parsed_paths: Vec<&str> = parsed.iter().map(|p| p.path.as_str()).collect();
    parsed_paths.sort();
    let mut all: Vec<String> = vec!["$prelude".to_string()];
    all.extend(stdlib_paths());
    all.sort();
    assert_eq!(parsed_paths, all);
    assert_eq!(all.len(), 1 + 54);
    let checked = parsed
        .into_iter()
        .filter(|p| p.path != "i/test")
        .collect::<Vec<_>>();
    let checked = ilang_check::check_parsed(&graph, &checked);
    let rendered = graph.sources.render_all(&checked.diags);
    assert!(rendered.is_empty(), "{rendered:#?}");
    let program = checked.program.unwrap();
    // Resolution orders the modules by their imports; compare as sets.
    let mut paths: Vec<&str> = program.modules.iter().map(|m| m.path.as_str()).collect();
    paths.sort();
    let mut want = vec!["$prelude".to_string()];
    want.extend(stdlib_paths().into_iter().filter(|p| p != "i/test"));
    want.sort();
    assert_eq!(paths, want);
    assert_eq!(want.len() + 1, 1 + 54);
    // The modules that the compiler names are all there.
    for path in ilang_package::library::RUNTIME
        .iter()
        .chain(
            ilang_check::CODEGEN_LIBRARY
                .iter()
                .filter(|p| **p != "i/test"),
        )
        .chain(&["i/syntax", "i/core"])
    {
        assert!(paths.contains(path), "{path}");
    }
}

/// The built-in package has only the prelude, with the text of
/// `lib/prelude.i`; every library module is in the package `i`, from its
/// file in `stdlib/src/`.
#[test]
fn library_modules_have_sources() {
    let (graph, _, _) = common::check_all_modules(&[]);
    let builtin = &graph.packages[graph.library];
    assert_eq!(builtin.modules.len(), 1);
    let (path, text) = ilang_package::library::PRELUDE;
    assert_eq!(builtin.modules[0].path, path);
    assert_eq!(graph.sources.get(builtin.modules[0].source).text, text);
    let id = graph.standard_library().expect("the dependency `i`");
    let root = &graph.packages[graph.root.unwrap()];
    assert_eq!(root.dependencies, [("i".to_string(), Some(id))]);
    let package = &graph.packages[id];
    assert_eq!(package.name, "i");
    let dir = common::stdlib_dir();
    assert_eq!(package.dir.as_deref(), Some(dir.as_path()));
    let mut count = 0;
    for path in stdlib_paths() {
        let module = package
            .modules
            .iter()
            .find(|m| m.path == path)
            .unwrap_or_else(|| panic!("{path} has no module"));
        let file = graph.sources.get(module.source);
        let rest = path.strip_prefix("i/").unwrap();
        let on_disk = ["i", "is"]
            .iter()
            .map(|e| dir.join("src").join(rest).with_extension(e))
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("{path} has no file"));
        assert_eq!(
            file.text,
            std::fs::read_to_string(on_disk).unwrap(),
            "{path}"
        );
        count += 1;
    }
    assert_eq!(package.modules.len(), count);
    assert_eq!(count, 54);
}

/// The exported names of each library module.
#[test]
fn library_names() {
    let (_, program, _) = common::check_all_modules(&[]);
    let program = program.unwrap();
    let expect: &[(&str, &[&str])] = &[
        ("i/fail", &["raise", "catch", "from_result", "map_error"]),
        (
            "i/control",
            &["boundary", "suspend", "escape", "resume", "cancel", "keep"],
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
                "wait",
                "await",
                "await_owned",
                "wait_any",
                "poll_requests",
            ],
        ),
        ("i/resource", &["acquire", "ensure", "scoped"]),
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
        ("$prelude", &["panic", "assert"]),
        ("i/float_text", &["to_string"]),
        ("i/bits", &["read_int", "from_int"]),
        (
            "i/big",
            &[
                "compare",
                "add",
                "subtract",
                "multiply",
                "divide",
                "remainder",
                "to_text",
            ],
        ),
        ("i/io", &["make_io"]),
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
                "set",
                "ref",
                "update",
                "both",
                "race",
                "map_concurrently",
            ],
        ),
        (
            "i/clock",
            &["monotonic_ns", "unix_ns", "sleep_ns"],
        ),
        (
            "i/random",
            &["from_seed", "word", "int", "float", "split", "at_path"],
        ),
        (
            "i/fs",
            &[
                "make_fs",
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
                "copy",
                "walk",
            ],
        ),
        (
            "i/process",
            &[
                "make_arguments",
                "make_entropy",
                "make_environment",
                "make_process",
                "make_working_directory",
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
                "collect",
                "wait",
                "terminate",
                "close",
            ],
        ),
        (
            "i/net",
            &[
                "make_net",
                "bind",
                "send_to",
                "receive_from",
                "local_address",
                "close_datagram",
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
                "make_tls",
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
            "i/fetch",
            &["make_fetch", "open", "status", "headers", "read", "close"],
        ),
        (
            "i/stdio",
            &[
                "make_stdio",
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
        let (module, function) = match &d.kind {
            DefKind::External {
                module, function, ..
            }
            | DefKind::Runtime {
                module, function, ..
            } => (module, function),
            _ => panic!("{} is not external", d.name),
        };
        assert_eq!(
            (module.as_str(), function.as_str()),
            ("ilang:core", d.name.as_str())
        );
    }
    // The functions that the library writes in i are bindings.
    for (path, name) in [
        ("i/assertion", "catch"),
        ("i/control", "keep"),
        ("i/resource", "acquire"),
        ("i/fail", "from_result"),
        ("i/fail", "map_error"),
        ("$prelude", "assert"),
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

/// The parameters of the external functions of the library have no
/// labels, for example
/// `control.suspend: (Boundary(…), (Continuation(…)) -> …) -> A`.
#[test]
fn library_labels() {
    let (graph, _, _) = common::check_all_modules(&[]);
    let (parsed, _) = ilang_check::parse_all(&graph);
    let parsed: Vec<_> = parsed.into_iter().filter(|p| p.path != "i/test").collect();
    let checked = ilang_check::check_parsed(&graph, &parsed);
    let errors: Vec<_> = checked.diags.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "{errors:?}");
    let program = checked.program.unwrap();
    let types = checked.types.unwrap();
    // Catch and keep are ordinary bindings. Inspect the public parameter
    // data shape, not the legacy HIR's diagnostic parameter-name hints.
    let labels = |path: &str, name: &str| -> Vec<String> {
        let module = common::module(&program, path);
        let index = program
            .module(module)
            .defs
            .iter()
            .position(|d| d.name == name)
            .unwrap();
        let id = ilang_check::hir::DefId {
            module,
            index: index as u32,
        };
        let ilang_check::types::ShownType::Fn(f) = &types.scheme_types[&id] else {
            panic!()
        };
        f.params
            .iter()
            .map(|(label, _)| label.clone().unwrap_or("_".into()))
            .collect()
    };
    assert_eq!(labels("i/control", "suspend"), ["_", "_"]);
    assert_eq!(labels("i/control", "resume"), ["_", "_"]);
    assert_eq!(labels("i/fail", "catch"), ["_"]);
    assert_eq!(labels("i/control", "keep"), ["_"]);
    assert_eq!(labels("i/resource", "ensure"), ["_", "_"]);
    assert_eq!(labels("i/core", "byte_slice"), ["_", "_", "_"]);
    assert_eq!(labels("i/core", "index_of"), ["_", "_", "_"]);
    assert_eq!(labels("$prelude", "panic"), ["_"]);
}

/// The built-in custom types are in the prelude.
#[test]
fn prelude_types() {
    let (_, program, _) = common::check_all_modules(&[]);
    let program = program.unwrap();
    let prelude = program.prelude.unwrap();
    let mut found: Vec<(String, Vec<String>)> = program
        .types
        .iter()
        .filter(|t| t.module == prelude)
        .filter_map(|t| {
            // `Bool` and `Result` are names of unions .
            let TypeBody::Custom(cs) = &t.body else {
                return None;
            };
            Some((t.name.clone(), cs.iter().map(|c| c.name.clone()).collect()))
        })
        .collect();
    found.sort();
    let want = [
        ("Assert", vec!["Assert"]),
        ("AssertionFailure", vec!["AssertionFailure"]),
        ("Error", vec!["Error"]),
        ("Fail", vec!["Fail"]),
        ("False", vec!["False"]),
        ("Ok", vec!["Ok"]),
        ("True", vec!["True"]),
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
    // Macro tests run through the CLI's expansion pipeline; this checks source.
    let (graph, mut diags) = ilang_package::load_with(
        &dir,
        &ilang_package::LoadOptions {
            tests: false,
            ..Default::default()
        },
    );
    let checked = ilang_check::check(&graph);
    diags.extend(checked.diags);
    assert!(diags.is_empty(), "{:#?}", graph.sources.render_all(&diags));
    assert!(checked.program.is_some() && checked.types.is_some());
}

const RESERVED: [&str; 7] = [
    "i/clock",
    "i/fs",
    "i/process",
    "i/net",
    "i/tls",
    "i/fetch",
    "i/stdio",
];

/// User code imports every reserved module with no problems.
///
/// These standard paths are registered in the library.
#[test]
fn reserved_modules_import_cleanly() {
    let imports: String = RESERVED.iter().map(|p| format!("  {p}\n")).collect();
    let main = format!("(\n{imports})\n\nx = 1\n");
    assert!(ilang_syntax::parse_module(&main).1.is_empty());
    let (_, program, diags) = common::check_files(&[("src/main.i", &main)]);
    common::expect_unused(&diags, &["x"]);
    assert!(program.is_some());
}

/// Missing names are reported for imports and qualified references.
#[test]
fn missing_library_names_are_reported() {
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
/// still a missing module, so the reserved modules do not
/// open the whole of `i` to user imports.
/// The prelude is built in: no import names it, and its names are in every
/// module without one.
#[test]
fn the_prelude_is_not_importable() {
    for main in ["(i/prelude)\n\nx = 1\n", "(prelude = i/prelude)\n\nx = 1\n"] {
        assert!(ilang_syntax::parse_module(main).1.is_empty());
        let (graph, _, diags) = common::check_files(&[("src/main.i", main)]);
        let found: Vec<_> = diags
            .iter()
            .filter(|d| d.code == ilang_check::code::NO_MODULE)
            .cloned()
            .collect();
        assert_eq!(
            graph.sources.render_all(&found),
            [
                "src/main.i:1:2: error[E0210]: there is no module `i/prelude`: \
                 the prelude is built in; its names need no import"
            ],
            "{main}"
        );
    }
    // The built-in names need no import.
    let main = "check = (x : Result(Int, ())) -> Bool {\n\tx |>\n\t\t(Ok(_)) { True() }\n\t\t& (Error(_)) { panic(\"no\") }\n}\n";
    let (graph, program, diags) = common::check_files(&[("src/main.i", main)]);
    let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).cloned().collect();
    let rendered = graph.sources.render_all(&errors);
    assert!(rendered.is_empty(), "{rendered:#?}");
    let program = program.unwrap();
    let prelude = program.prelude.unwrap();
    assert_eq!(program.module(prelude).path, "$prelude");
}

#[test]
fn unknown_library_path_is_missing() {
    // The removed `i/record` module must remain unavailable. Keep the corpus probe
    // import rejection independent of its earlier no-type-spread parse error.
    for path in ["i/nope", "i/tasks", "i/std", "i/fs/file", "i/record"] {
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
