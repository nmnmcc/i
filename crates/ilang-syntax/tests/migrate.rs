//! Frozen migration inputs and interpretation checks for A1 R1-R12.
mod common;
use ilang_syntax::{
    format::{format, format_from_old},
    migrate::{self, Env},
    parse_module, parse_module_old,
};

#[test]
fn frozen_inputs_and_the_independent_write_check() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/migrate");
    let mut paths: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect();
    paths.sort();
    for path in paths {
        let source = std::fs::read_to_string(&path).unwrap();
        let (old, errors) = parse_module_old(&source);
        let old_mode = errors.is_empty();
        let module = if old_mode {
            old
        } else {
            let (m, d) = parse_module(&source);
            assert!(d.is_empty(), "{} {d:?}", path.display());
            m
        };
        let plan = migrate::plan(&module, &Env::new(&module));
        let reason = match path.file_stem().unwrap().to_str().unwrap() {
            "unsafe_split" => Some("tyvar-owner"),
            "known" => Some("one-clause-known"),
            "expected_function" => Some("fn-expected-type"),
            _ => None,
        };
        if let Some(reason) = reason {
            assert_eq!(
                plan.refused.as_ref().map(|r| r.reason),
                Some(reason),
                "{}",
                path.display()
            );
            assert!(format_from_old(&source).is_err());
            continue;
        }
        assert!(
            plan.refused.is_none(),
            "{} {:?}",
            path.display(),
            plan.refused
        );
        let out = if old_mode {
            format_from_old(&source)
        } else {
            format(&source)
        }
        .unwrap_or_else(|d| panic!("{} {d:?}", path.display()));
        migrate::check_written(&module, &plan, &out)
            .unwrap_or_else(|d| panic!("{} {d:?}\n{out}", path.display()));
        assert_eq!(format(&out).unwrap(), out, "{}", path.display());
        assert!(
            migrate::plan(&parse_module(&out).0, &Env::new(&parse_module(&out).0)).is_empty(),
            "{}",
            path.display()
        );
        if path.file_stem().unwrap() == "kept_expected_function" {
            assert!(!out.contains("g ="));
        }
        if path.file_stem().unwrap() == "mixed_outer" {
            assert!(out.contains("f(A1)"), "{out}");
        }
        if path.file_stem().unwrap() == "mixed_inner" {
            assert!(out.contains("g(A1)"), "{out}");
        }
        if old_mode {
            let spliced = migrate::splice(&source, &plan)
                .unwrap_or_else(|d| panic!("{} {d:?}", path.display()));
            assert_eq!(
                source.bytes().filter(|b| *b == b'\r').count(),
                spliced.text.bytes().filter(|b| *b == b'\r').count()
            );
            assert_eq!(
                spliced.offset(source.len() as u32),
                spliced.text.len() as u32
            );
        }
    }
}

#[test]
fn every_frozen_old_golden_that_parses_can_migrate_or_is_explicitly_refused() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden_old");
    for path in std::fs::read_dir(dir)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "i"))
    {
        let source = std::fs::read_to_string(&path).unwrap();
        let (m, d) = parse_module_old(&source);
        if !d.is_empty() {
            continue;
        }
        let plan = migrate::plan(&m, &Env::new(&m));
        if plan.refused.is_none() {
            format_from_old(&source).unwrap_or_else(|d| panic!("{} {d:?}", path.display()));
            migrate::splice(&source, &plan).unwrap_or_else(|d| panic!("{} {d:?}", path.display()));
        }
    }
}

#[test]
fn review_write_check_preserves_string_and_documentation_payloads() {
    for source in [
        "f = () { \"@1..2\" }\n",
        "//// @1..2\nf = 1\n",
        "@deprecated(\"@1..2\")\nf = 1\n",
        "f = (s) { case s { \"@1..2\" -> 1 _ -> 0 } }\n",
    ] {
        let (m, d) = parse_module_old(source);
        assert!(d.is_empty());
        let plan = migrate::plan(&m, &Env::new(&m));
        let out = format_from_old(source).unwrap();
        assert!(migrate::check_written(&m, &plan, &out.replace("@1..2", "@3..4")).is_err());
    }
}

#[test]
fn review_extraction_keeps_distinct_braces_and_reaches_a_fixed_point() {
    let source = "f = () { case (x) -> Int { x }(42) { _ -> 42 } * 10 }\n@main\nmain = () { assert(f() == 420) Nil }\n";
    let (m, d) = parse_module_old(source);
    assert!(d.is_empty());
    let plan = migrate::plan(&m, &Env::new(&m));
    assert_eq!(plan.extractions.len(), 1);
    let layout = migrate::splice(source, &plan).unwrap();
    assert!(parse_module(&layout.text).1.is_empty());
    migrate::check_written(&m, &plan, &layout.text).unwrap();
    let out = format_from_old(source).unwrap();
    assert_eq!(format(&out).unwrap(), out);
    assert_eq!(format(&layout.text).unwrap(), out);
}
