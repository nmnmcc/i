//! The six library modules and the prelude parse and resolve with no
//! problems, and they have the names and labels of the spec tables.

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
            "i/core",
            "i/io"
        ]
    );
}

/// The exported names of each library module, as in §9.3–§9.6 and §13.
#[test]
fn library_names() {
    let (_, program, _) = common::check_files(&[]);
    let program = program.unwrap();
    let expect: &[(&str, &[&str])] = &[
        ("i/fail", &["raise", "catch", "from_result", "map_error"]),
        ("i/control", &["boundary", "suspend", "resume", "cancel"]),
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
            ],
        ),
        ("i/prelude", &["panic", "assert"]),
        ("i/io", &[]),
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
    ] {
        let d = common::def(&program, path, name);
        assert!(matches!(d.kind, DefKind::Binding { .. }), "{path}.{name}");
    }
}

/// The labels of the parameters in the spec tables, for example
/// `control.suspend: (b: …, f: (k: …) -> …) -> a`.
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
