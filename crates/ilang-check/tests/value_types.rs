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
        "n: Int = 1\nA = n\n@host(\"env\")\nService = A",
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

#[test]
fn expanded_places_and_constant_head_alias_parts() {
    // K2: checking only each Of misses the union surrounding two local holders.
    bad(
        "pick(A,B) = (a: A, b: B, c: Bool) { result = c |> { True -> a  False -> b }\n (result,a,b) }\nuse = (f: pick) { f }",
        code::TYPE_PLACE,
    );
    bad(
        "f = (a,b) { g = (x: List(a | b)) { x }  g }",
        code::TYPE_PLACE,
    );
    // P2: exact constants are observations; they cannot fill a source hole.
    good("Tail(A) = (A,Int)\np: (String,Int) = (\"a\",1)\nTail(Text) = p");
    bad(
        "Tail(A) = (A,Int)\np: (String,String) = (\"a\",\"b\")\nTail(Text) = p",
        code::TYPE_PATTERN_MISMATCH,
    );
    bad(
        "Dup(A) = (A,A)\np: (Int,Int) = (1,2)\nDup(Text) = p",
        code::TYPE_PATTERN_FORM,
    );
    bad(
        "Fn(A) = (A) -> A\nf = (x: Int) {x}\nFn(Text) = f",
        code::TYPE_PATTERN_FORM,
    );
}

#[test]
fn imported_pattern_binders_are_never_reclassified() {
    // P1/P3: an imported User changes the collision, never the item kind.
    let files = [
        ("src/types.i", "(User)\nUser {}"),
        (
            "src/main.i",
            "((User) = app/types)\nBox(A) {Box(A)}\nv: Box(Int) = Box(1)\nBox(User) = v",
        ),
    ];
    let (g, c) = checked(&files);
    assert!(
        c.diags
            .iter()
            .any(|d| d.code == code::TYPE_PATTERN_BINDER_IS_TYPE),
        "{:?}",
        g.sources.render_all(&c.diags)
    );
    bad("Pair(A,_) = macro_type()", code::TYPE_PARAM_FORM);
    bad(
        "(m = i/core)\nf = () {m = {x: m.inspect = 1  x}  m}",
        code::NOT_A_MODULE,
    );
}

#[test]
fn eager_alias_dependencies_and_reverse_ties() {
    let src = "Wrap(A) {Wrap(A)}\nComposite = List(Item)\nv: Wrap(Int) = Wrap(1)\nWrap(Item) = v\nleft: Int = 1\nright: String = \"x\"\nL = left\nR = right\nuse = (x: Composite) { x }";
    let (g, c) = good(src);
    let mut p = c.program.unwrap();
    let first = c.types.unwrap().alias_types;
    order::reverse_type_ties(&mut p);
    let (second, ds) = types::check(&p, &g.sources);
    assert!(
        !ds.iter().any(Diagnostic::is_error),
        "{:?}",
        g.sources.render_all(&ds)
    );
    assert_eq!(first, second.alias_types);
    let (second, ds) = types::check_keeping_locals_with_errors(&p, &g.sources, true);
    assert!(
        !ds.iter().any(Diagnostic::is_error),
        "{:?}",
        g.sources.render_all(&ds)
    );
    assert_eq!(first, second.alias_types);
}

#[test]
fn r7_closed_source_annotations_and_direct_roots() {
    let base = "(Codec,size,LocalSize)\nCodec(D,E) {Codec(decode:(String)->D,encode:(E)->String)}\nmake(D,E) = (d:(String)->D,e:(E)->String) {Codec(d,e)}\nsize = make((s:String)->Int {1},(n:Int)->String {\"\"})\nCodec(LocalSize,_) = size\nlater = () {size.encode(2)}";
    let client = "((Codec) = app/lib)\nCodec(RemoteSize,_) = lib.size";
    // R7 gates both a direct constructor-shaped root and a weak wrapped root.
    let (g, c) = checked(&[("src/lib.i", base), ("src/client.i", client)]);
    assert!(
        c.diags.iter().any(|d| d.code == code::VALUE_TYPE_NOT_FIXED),
        "{:?}",
        g.sources.render_all(&c.diags)
    );
    let closed = base.replace("size = make", "size: Codec(Int,Int) = make");
    let (g, c) = checked(&[("src/lib.i", &closed), ("src/client.i", client)]);
    assert!(
        !c.diags.iter().any(Diagnostic::is_error),
        "{:?}",
        g.sources.render_all(&c.diags)
    );
    assert_eq!(alias(&c, "RemoteSize"), "Int");
}

#[test]
fn exported_aliases_cannot_leak_private_nominal_types() {
    // X2: the source value being public does not export a private nominal type.
    bad(
        "(Alias)\nSecret {Secret}\nvalue = Secret\nAlias = value",
        code::EXPORT_HIDDEN_TYPE,
    );
    bad(
        "(use)\nSecret {Secret}\nvalue = Secret\nA = value\nuse: (A) -> A = (x) {x}",
        code::EXPORT_HIDDEN_TYPE,
    );
    good("(A)\nvalue: Int = 1\nA = value");
}

#[test]
fn query_sources_heads_binders_and_outline() {
    // X4: each written role has a different navigation target.
    let src = "Pair(A,B) = (A,B)\nvalue: (Int,String) = (1,\"a\")\nPair(Left,Right) = value\nuse = (x: Left) {x}";
    let (g, c) = good(src);
    let p = c.program.as_ref().unwrap();
    let m = common::module(p, "app/main");
    let q = query::Query::new(&g, p, c.types.as_ref());
    let head = src.find("Pair(Left").unwrap() as u32;
    assert_eq!(q.definition_at(m, head).unwrap().span.start, 0);
    let source = src.find("= value").unwrap() as u32 + 2;
    assert_eq!(
        q.definition_at(m, source).unwrap().span.start,
        src.find("value:").unwrap() as u32
    );
    let left = src.find("Left,Right").unwrap() as u32;
    assert!(q.type_at(m, left).unwrap().code.contains("Left = Int"));
    let symbols = q.symbols(m);
    assert!(symbols.iter().any(|s| s.name == "Left"));
    assert!(symbols.iter().any(|s| s.name == "Right"));
}

#[test]
fn a_consumer_cannot_fix_the_library() {
    // R3/X3: adding only a consumer Of demand leaves the library interface intact.
    let lib = "(Cell,r,reset)\nCell(A) {Cell(get:()->A,set:(A)->Nil)}\ncell(A) = (x:A) {Cell(get:(){x},set:(y:A){Nil})}\nr = cell(1)\nput(A) = (c:Cell(A),value:A) {c.set(value)}\nreset = () {put(r,\"s\")}";
    let (g, base) = checked(&[("src/lib.i", lib)]);
    assert!(
        !base.diags.iter().any(Diagnostic::is_error),
        "{:?}",
        g.sources.render_all(&base.diags)
    );
    let (g, consumer) = checked(&[
        ("src/lib.i", lib),
        ("src/main.i", "(app/lib)\nRead = lib.r"),
    ]);
    assert!(
        consumer
            .diags
            .iter()
            .any(|d| d.code == code::VALUE_TYPE_NOT_FIXED),
        "{:?}",
        g.sources.render_all(&consumer.diags)
    );
    fn library_schemes(c: &Checked) -> Vec<(String, String)> {
        let p = c.program.as_ref().unwrap();
        let m = common::module(p, "app/lib");
        p.module(m)
            .defs
            .iter()
            .enumerate()
            .map(|(i, d)| {
                (
                    d.name.clone(),
                    c.types.as_ref().unwrap().schemes[&DefId {
                        module: m,
                        index: i as u32,
                    }]
                        .clone(),
                )
            })
            .collect()
    }
    assert_eq!(library_schemes(&base), library_schemes(&consumer));
}

#[test]
fn fixation_keeps_constraints_and_rejects_earlier_weak_parts() {
    // R5: an ordinary generic helper avoids a narrowed known-field fact.
    let prefix = "Cell(A) {Cell(get:()->A,set:(A)->Nil)}\ncell(A) = (x:A) {Cell(get:(){x},set:(y:A){Nil})}\nput(A) = (c:Cell(A),value:A) {c.set(value)}\nr = cell(1)\n";
    good(&[prefix, "later = () {put(r,\"s\")}"].concat());
    let src = [prefix, "Read = r\nlater = () {put(r,\"s\")}"].concat();
    let (g, c) = checked(&[("src/main.i", &src)]);
    assert!(
        c.diags
            .iter()
            .any(|d| d.code == code::TYPE_MISMATCH && d.message.contains("fixed here")),
        "{:?}",
        g.sources.render_all(&c.diags)
    );
    let wide = src.replace("r = cell(1)", "r: Cell(Int | String) = cell(1)");
    good(&wide);
    let earlier = [
        prefix,
        "get = () {r}\nRead = get\nlater = () {put(r,\"s\")}",
    ]
    .concat();
    let (g, c) = checked(&[("src/main.i", &earlier)]);
    assert!(
        c.diags.iter().any(|d| d.code == code::VALUE_TYPE_NOT_FIXED),
        "{:?}",
        g.sources.render_all(&c.diags)
    );
    assert!(!c.diags.iter().any(|d| d.message.contains("fixed here")));
    let mut p = c.program.unwrap();
    order::reverse_type_ties(&mut p);
    let (_, ds) = types::check(&p, &g.sources);
    assert!(ds.iter().any(|d| d.code == code::VALUE_TYPE_NOT_FIXED));
    assert!(!ds.iter().any(|d| d.message.contains("fixed here")));
}

#[test]
fn every_bad_dependency_component_recovers_without_publishing_aliases() {
    for cycle in [
        "A = f\nf = (x:A) {x}",
        "a = (x:b) {x}\nb = (x:a) {x}",
        "a = (x:b) {x}\nb = a(1)",
    ] {
        let src = [cycle, "\nanswer: Int = 42"].concat();
        let (g, _) = checked(&[("src/main.i", &src)]);
        let c = query::check_for_queries(&g);
        assert!(c.diags.iter().any(|d| d.code == code::VALUE_TYPE_CIRCLE));
        let typed = c.types.as_ref().unwrap();
        let p = c.program.as_ref().unwrap();
        let m = common::module(p, "app/main");
        let answer = p
            .module(m)
            .defs
            .iter()
            .position(|d| d.name == "answer")
            .unwrap();
        assert_eq!(
            typed.schemes[&DefId {
                module: m,
                index: answer as u32
            }],
            "Int"
        );
        for id in &p.module(m).types {
            assert!(!typed.alias_types.contains_key(id));
        }
    }
}

#[test]
fn alias_formals_and_written_holes_are_instantiated_per_use() {
    // R2 closes the Of part; ordinary written formals/holes keep their rules.
    good("port: Int = 1\nT = (port,_)\nf = () {a: T = (1,\"s\")  b: T = (2,3)\n (a,b)}");
    let prefix =
        "Cell(A) {Cell(get:()->A,set:(A)->Nil)}\nport: Int = 1\nT(A) = Cell(A) | Cell(port)\n";
    // Invariant same-head union still requires equal arguments after substitution.
    bad(
        &[prefix, "f = (x: T(String)) {x}"].concat(),
        code::TYPE_INVARIANT,
    );
    good(&[prefix, "f = (x: T(Int)) {x}"].concat());
}

#[test]
fn places_keep_local_declaration_identity_before_allocation() {
    bad(
        "port: Int = 1\nuse = () {g(A,B): (A | B,port) -> Int = (x,y) {1}  g}",
        code::TYPE_PLACE,
    );
}

#[test]
fn standard_library_reverse_order_and_recovery_preserve_types() {
    // O2 and checklist 9.11 require measuring the ordinary, no-feature corpus.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let options = ilang_package::LoadOptions {
        write_lock: false,
        offline: true,
        ..Default::default()
    };
    let (g, ds) = ilang_package::load_with(&root, &options);
    assert!(
        !ds.iter().any(Diagnostic::is_error),
        "{:?}",
        g.sources.render_all(&ds)
    );
    let c = check(&g);
    assert!(
        !c.diags.iter().any(Diagnostic::is_error),
        "{:?}",
        g.sources.render_all(&c.diags)
    );
    let mut p = c.program.unwrap();
    assert!(p.of_sites.is_empty());
    let before = p.order.groups.clone();
    let init = p.order.init.clone();
    let needs = p
        .modules
        .iter()
        .enumerate()
        .map(|(i, _)| order::needs(&p, ModuleId(i as u32)))
        .collect::<Vec<_>>();
    order::reverse_type_ties(&mut p);
    assert_ne!(before, p.order.groups);
    assert_eq!(init, p.order.init);
    assert_eq!(
        needs,
        p.modules
            .iter()
            .enumerate()
            .map(|(i, _)| order::needs(&p, ModuleId(i as u32)))
            .collect::<Vec<_>>()
    );
    let (reversed, ds) = types::check_keeping_locals_with_errors(&p, &g.sources, true);
    assert!(
        !ds.iter().any(Diagnostic::is_error),
        "{:?}",
        g.sources.render_all(&ds)
    );
    let normal = c.types.unwrap();
    assert_eq!(normal.schemes, reversed.schemes);
    assert_eq!(normal.scheme_types, reversed.scheme_types);
    assert!(reversed.alias_types.is_empty());
}

#[test]
fn spec_corpus_reverse_order_and_recovery() {
    // Checklist 9.11: compare real corpus inference, including recovery paths.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/spec");
    let options = ilang_package::LoadOptions {
        write_lock: false,
        offline: true,
        ..Default::default()
    };
    let mut count = 0;
    for section in std::fs::read_dir(root).unwrap() {
        let section = section.unwrap().path();
        if !section.is_dir() {
            continue;
        }
        for path in std::fs::read_dir(section).unwrap() {
            let path = path.unwrap().path();
            let temp = tempfile::tempdir().unwrap();
            let project = if path.is_dir() {
                path.clone()
            } else if path.extension().is_some_and(|x| x == "i") {
                std::fs::write(temp.path().join("i.toml"), "name = \"spec\"\n").unwrap();
                std::fs::create_dir(temp.path().join("test")).unwrap();
                std::fs::copy(
                    &path,
                    temp.path().join("test").join(path.file_name().unwrap()),
                )
                .unwrap();
                temp.path().to_path_buf()
            } else {
                continue;
            };
            let (g, load_diags) = ilang_package::load_with(&project, &options);
            if load_diags.iter().any(Diagnostic::is_error) {
                continue;
            }
            let (p, resolve_diags) = ilang_check::resolve_program(&g);
            // Parser-rejected cases have no inference graph to permute.
            let Some(mut p) = p else {
                continue;
            };
            let prior_errors = resolve_diags.iter().any(Diagnostic::is_error);
            let (normal, ds) =
                types::check_keeping_locals_with_errors(&p, &g.sources, prior_errors);
            let init = p.order.init.clone();
            order::reverse_type_ties(&mut p);
            let (reverse, rs) =
                types::check_keeping_locals_with_errors(&p, &g.sources, prior_errors);
            assert_eq!(init, p.order.init, "{}", path.display());
            let diagnostics = |ds: Vec<Diagnostic>| {
                let mut ds = ds
                    .into_iter()
                    .map(|d| format!("{:?}", d))
                    .collect::<Vec<_>>();
                ds.sort();
                ds
            };
            assert_eq!(diagnostics(ds), diagnostics(rs), "{}", path.display());
            assert_eq!(normal.schemes, reverse.schemes, "{}", path.display());
            assert_eq!(
                normal.alias_types,
                reverse.alias_types,
                "{}",
                path.display()
            );
            count += 1;
        }
    }
    assert!(count > 0, "no resolved spec cases");
    eprintln!("Compared all {count} resolved spec cases in normal/reversed recovery inference");
}
