//! Requirements of value-types V/R/P/O/K/X, independent of generated goldens.
mod common;
use ilang_check::{Checked, check, code, hir::*, order, query, types};
use ilang_package::{Diagnostic, PackageGraph};

fn checked(files: &[(&str, &str)]) -> (PackageGraph, Checked) {
    let (g, _, _) = common::check_files(files);
    let c = check(&g);
    (g, c)
}
fn good(src: &str) -> (PackageGraph, Checked) {
    let (g, c) = checked(&[("src/main.i", src)]);
    assert!(
        !c.diags.iter().any(Diagnostic::is_error),
        "{:?}",
        g.sources.render_all(&c.diags)
    );
    (g, c)
}
fn bad(src: &str, code: &str) {
    let (g, c) = checked(&[("src/main.i", src)]);
    assert!(
        c.diags.iter().any(|d| d.code == code),
        "{src}: expected {code}: {:?}",
        g.sources.render_all(&c.diags)
    );
}
fn alias(c: &Checked, name: &str) -> String {
    let p = c.program.as_ref().unwrap();
    let (i, t) = p
        .types
        .iter()
        .enumerate()
        .find(|(_, t)| t.name == name)
        .unwrap();
    c.types.as_ref().unwrap().alias_types[&TypeId(i as u32)].text(&types::type_names(p, t.module))
}

#[test]
fn annotation_instances_and_local_sharing() {
    // R1: each completed generalized source occurrence is a fresh instance.
    good(
        "id = (x) { x }\ntwice = (f: id, n: Int) { f(f(n)) }\nuse = () { (twice(id, 3), twice(id, 4)) }",
    );
    // R1: a source parameter's one stored monotype is shared in its body.
    good(
        "f = (xs: List(Int)) { acc: xs = []  acc }\nfitted = (s: String) { x = \"a\" = s  y: x = \"b\"  y }",
    );
    bad("f = (x: Int) { y: x = \"s\"  y }", code::TYPE_MISMATCH);
    bad("id = (x) { x }\nSame = id", code::VALUE_TYPE_OPEN);
    bad(
        "id = (x) { x }\ng = id(id)\nG = g",
        code::VALUE_TYPE_NOT_FIXED,
    );
}

#[test]
fn pending_scopes_and_identity_shadowing() {
    // V3 reserves the entire header and pattern, even when an outer exists.
    for src in [
        "x = 1\nf = (x, y: x) { y }",
        "f = (x) -> x { x }",
        "f = (g: f) { g }",
        "f = (x) { (a, b: a) = (1, 2)  b }",
    ] {
        bad(src, code::VALUE_TYPE_OWN_SCOPE);
    }
    good("f = () { inner = (f: Int) { y: f = f  y }  inner(1) }");
    bad("f = () { f = (x: f) { x }  f }", code::VALUE_TYPE_OWN_SCOPE);
    bad(
        "(m = i/core)\nf = (m, x: m.inspect) { x }",
        code::NOT_A_MODULE,
    );
    bad("(i/core)\nT = core", code::MODULE_AS_VALUE);
    bad("(i/core)\nT = core.missing", code::NO_MEMBER);
}

#[test]
fn closed_aliases_and_patterns_complete_eagerly() {
    // R2/P4: a closed scalar is read exactly; permanent binders are aliases.
    let (_, c) = good(
        "value: Int = 1\nRead = value\nPair(A,B) = (A,B)\np: (Int,String) = (1,\"a\")\nPair(Left,Right) = p\nComposite(A) = (A, value)\nuse = (x: Read, y: Left) { x + y }",
    );
    assert_eq!(alias(&c, "Read"), "Int");
    assert_eq!(alias(&c, "Left"), "Int");
    assert_eq!(alias(&c, "Right"), "String");
    assert_eq!(alias(&c, "Composite"), "(A, Int)");
    // P5 checks even a pattern binding nothing.
    bad(
        "Box(A) { Box(A) }\nv: Int = 1\nBox(_) = v",
        code::TYPE_PATTERN_MISMATCH,
    );
    bad(
        "Box(A) { Box(A) }\nv: Box(Int) = Box(1)\nBox(Int) = v",
        code::TYPE_PATTERN_BINDER_IS_TYPE,
    );
    bad(
        "Box(A) { Box(A) }\nv = Box(1)\nBox(Foo,Foo) = v",
        code::DEFINED_TWICE,
    );
}

#[test]
fn type_edges_do_not_change_runtime_needs_or_generalization() {
    // O2: a depends on b only for inference; b still has no runtime need a.
    let (_, c) = good("a = (x: b) { x }\nb: Int = 1");
    let p = c.program.as_ref().unwrap();
    let m = common::module(p, "app/main");
    assert!(order::needs(p, m)[0].0.is_empty());
    assert_eq!(
        p.order
            .groups
            .iter()
            .filter(|g| g[0].module == m)
            .map(|g| g[0].index)
            .collect::<Vec<_>>(),
        vec![1, 0]
    );
    for src in [
        "a = (n: b) { n }\nb = a(1)",
        "A = f\nf = (x: A) { x }",
        "a = (x: b) { x }\nb = (x: a) { x }",
    ] {
        bad(src, code::VALUE_TYPE_CIRCLE);
    }
    bad("A = List(A)", code::ALIAS_CIRCLE);
}

#[test]
fn early_contexts_preserve_dependency_provenance() {
    // K3/J6: even a completed Int alias remains forbidden at early uses.
    for src in [
        "n: Int = 1\nA = n\nBox { Box(A) }",
        "n: Int = 1\nA = n\n@external(\"env\",\"f\")\nf: (A) -> Int",
        "n: Int = 1\nA = n\nf = (x ! A) { x }",
        "n: Int = 1\nA = n\nf = () -> Int ! A { 1 }",
    ] {
        bad(src, code::VALUE_TYPE_PLACE);
    }
}

#[test]
fn source_module_alias_is_the_foreign_closed_read_boundary() {
    // R7: library completion may fix decode while leaving encode live.
    let lib = "(Codec, size)\nCodec(D,E) { Codec(decode: (String) -> D, encode: (E) -> String) }\nmake(D,E) = (d: (String) -> D, e: (E) -> String) { Codec(decode:d,encode:e) }\nid = (x) { x }\nsize = id(make((s: String) -> Int { 1 }, (n: Int) -> String { \"\" }))\nCodec(LocalSize,_) = size";
    let client = "((Codec) = app/lib)\nCodec(RemoteSize,_) = lib.size";
    let (g, c) = checked(&[("src/lib.i", lib), ("src/client.i", client)]);
    assert!(
        c.diags.iter().any(|d| d.code == code::VALUE_TYPE_NOT_FIXED
            && d.message.contains("LocalSize")
            && d.message.contains("export")),
        "{:?}",
        g.sources.render_all(&c.diags)
    );
    let lib = lib.replacen("(Codec, size)", "(Codec, size, LocalSize)", 1);
    let (g, c) = checked(&[
        ("src/lib.i", &lib),
        (
            "src/client.i",
            "((LocalSize) = app/lib)\nRemoteSize = LocalSize",
        ),
    ]);
    assert!(
        !c.diags.iter().any(Diagnostic::is_error),
        "{:?}",
        g.sources.render_all(&c.diags)
    );
    // An ordinary alias copied from a completed alias has no new Of site.
    let p = c.program.as_ref().unwrap();
    let (_, ds) = types::check(p, &g.sources);
    assert!(!ds.iter().any(Diagnostic::is_error));
}

#[test]
fn query_recovery_keeps_unrelated_results() {
    // O4/X4: explicit recovery permits failed reads, never invented aliases.
    let (g, _) = checked(&[("src/main.i", "a = (x: b) { x }\nb = a(1)\nanswer: Int = 42")]);
    let c = query::check_for_queries(&g);
    assert!(c.types.is_some());
    assert!(c.diags.iter().any(|d| d.code == code::VALUE_TYPE_CIRCLE));
}
