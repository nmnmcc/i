//! The transition front of syntax three (docs/design/syntax-3.md §7.2,
//! stage 8): a syntax-three text lowers to the tree that today's text gives.

#[path = "../examples/support/legacy/mod.rs"]
mod legacy;

use ilang_syntax::print::print_module;
use ilang_syntax::syn3;

/// `new` (syntax three) lowers to the tree of `old` (today's syntax), as
/// today's printer shows both.
fn same(new: &str, old: &str) {
    let (module, diags) = syn3::parse_lowered(new);
    assert!(diags.is_empty(), "{new}\n{diags:#?}");
    let (expected, diags) = legacy::parser::parse_module_legacy(old);
    assert!(diags.is_empty(), "{old}\n{diags:#?}");
    assert_eq!(print_module(&module), print_module(&expected), "{new}");
}

/// The lowered tree of `new`, as today's printer shows it.
fn shown(new: &str) -> String {
    let (module, diags) = syn3::parse_lowered(new);
    assert!(diags.is_empty(), "{new}\n{diags:#?}");
    print_module(&module)
}

fn bad(new: &str, needle: &str) {
    let (_, diags) = syn3::parse_lowered(new);
    assert!(
        diags.iter().any(|d| d.message.contains(needle)),
        "{new}\nexpected an error with `{needle}`, got {diags:#?}"
    );
}

#[test]
fn type_definitions() {
    same(
        "(\n\tOption(..)\n\tShape\n\tCircle\n\tSquare\n)\n\nSome(A) = (A)\nNone = ()\nOption(A) = Some(A) | None\n\nCircle = (radius = Float)\nSquare = (Float)\nShape = Circle | Square\n\nUser = (name = String, age = Int)\n\nHandle = Opaque\n\nPair(A) = { (A, A) }\n\nName = String\n",
        "(\n\tOption(..)\n\tShape\n)\n\nOption(A) {\n\tSome(A)\n\tNone\n}\n\nShape {\n\tCircle(radius : Float)\n\tSquare(Float)\n}\n\nUser {\n\tUser(name : String, age : Int)\n}\n\nHandle {}\n\nPair(A) = (A, A)\n\nName = String\n",
    );
}

#[test]
fn constructor_named_like_its_sum() {
    same(
        "Inherit = ()\nDirectory = (String)\nDirectory = Inherit | Directory\n",
        "Directory {\n\tInherit\n\tDirectory(String)\n}\n",
    );
}

#[test]
fn matches_and_chains() {
    same(
        "f = (n : Int) -> String {\n\tn |> (0) { \"zero\" } | (1 | 2) { \"few\" } | (x ? x > 9) { \"many\" } | (_) { \"some\" }\n}\n",
        "f = (n : Int) -> String ^ _ ! _ {\n\tn |> {\n\t\t0 -> \"zero\"\n\t\t1 | 2 -> \"few\"\n\t\tx ? x > 9 -> \"many\"\n\t\t_ -> \"some\"\n\t}\n}\n",
    );
    same(
        "name = (Some(u)) { u } | (None) { \"\" }\n",
        "name = {\n\tSome(u) -> u\n\tNone -> \"\"\n}\n",
    );
}

#[test]
fn calls_data_and_patterns() {
    same(
        "u = User(name = \"ada\", age = 36)\nv = User(..u, age = 37)\nt = (1, 2)\nr = (a = 1, b = 2)\ne = ()\nx = t.0\ng = (User(name = n, ..)) { n }\nh = (User(name = name, age = age)) { name }\n",
        "u = User(name : \"ada\", age : 36)\nv = User(..u, age : 37)\nt = (1, 2)\nr = (a : 1, b : 2)\ne = ()\nx = t.0\ng = (User(n = name, ..)) { n }\nh = (User(name, age)) { name }\n",
    );
}

#[test]
fn empty_data_has_the_same_tree_on_both_surfaces() {
    // Spec §14.3: "(,) is the empty data in the positions of values,
    // types and patterns. The empty () has no position."
    let (normal, diags) = syn3::parse_lowered("empty = ()\n");
    assert!(diags.is_empty(), "{diags:#?}");
    let (lisp, diags) = syn3::parse_lisp_lowered("(= empty (,))\n");
    assert!(diags.is_empty(), "{diags:#?}");
    assert!(ilang_syntax::shape::equivalent(&normal, &lisp));
    let (_, diags) = syn3::parse_lisp_lowered("(= empty ())\n");
    assert!(!diags.is_empty());
}

#[test]
fn services_and_needs() {
    same(
        "find_user = (id : String) <- (db : Db, log : Log) {\n\tlog.write(id)\n\tdb.find(id)\n}\n\nDb = (find = (String) <- (Fail(NotFound), Log) -> User)\n",
        "find_user = (id : String, db ! Db, log ! Log) {\n\tlog.write(id)\n\tdb.find(id)\n}\n\nDb {\n\tDb(find : (String) -> User ^ NotFound ! Log)\n}\n",
    );
}

#[test]
fn provides() {
    // `! _ = e` is not written in today's syntax: only the front makes it.
    let text = shown(
        "check = () {\n\t{\n\t\tu = find_user(\"1\")\n\t\tassert(u.name == \"ada\")\n\t} <- (fake_db, quiet_log)\n}\n\nn = () { 1 + f(2) <- (s) }\n",
    );
    assert_eq!(
        text,
        "check = () {\n  ! _ = fake_db\n  ! _ = quiet_log\n  u = find_user(\"1\")\n  assert(u.name == \"ada\")\n}\n\nn = () {\n  ! _ = s\n  1 + f(2)\n}\n"
    );
}

#[test]
fn type_parameters_and_row_variables() {
    let text = shown(
        "map = <A, B, N>(xs : List(A), f : (A) <- N -> B) -> List(B) { [] }\n\ncatch : <A, E, N>(() <- (..N, Fail(E)) -> A) <- N -> Result(A, E)\n",
    );
    assert!(text.contains("map(A, B, N, N%E) ="), "{text}");
    assert!(text.contains("f : (A) -> B ^ N%E ! N"), "{text}");
    assert!(
        text.contains("catch(A, E, N): (() -> A ^ E ! N) -> Result(A, E) ! N"),
        "{text}"
    );
}

#[test]
fn several_positions() {
    let text = shown("gcd = (a, 0) { a } | (a, b) { gcd(b, a % b) }\n");
    assert!(text.contains("(%arg1, %arg2)"), "{text}");
}

/// Stage 9: a chain with several shape groups, or with a label in its
/// shape, is one hidden binding for each group and then the name, bound to
/// the transition form of the multi-shape value. The parts of a shape are
/// in canonical order: the positions, then the labels by name.
#[test]
fn shape_groups() {
    let text = shown(
        "area = (r : Float) { r } | (w : Float, h : Float) { w *. h }\n\nopen = (host : String, port = p : Int, addr = a : String) { p }\n",
    );
    let bindings: Vec<&str> = text
        .lines()
        .filter(|l| !l.starts_with(['\t', '}']) && l.contains(" = "))
        .collect();
    assert_eq!(bindings.len(), 5, "{text}");
    assert!(bindings[0].starts_with("area%g0 = (r: Float)"), "{text}");
    assert!(
        bindings[1].starts_with("area%g1 = (w: Float, h: Float)"),
        "{text}"
    );
    assert!(
        bindings[2].starts_with("area = %multi(1 : area%g0, 2 : area%g1)"),
        "{text}"
    );
    assert!(
        bindings[3].starts_with("open%g0 = (host: String, a: String, p: Int)"),
        "{text}"
    );
    assert!(
        bindings[4].starts_with("open = %multi(1,addr,port : open%g0)"),
        "{text}"
    );
    // A chain that is not bound to a name has its group functions in place.
    let text = shown("fs = [(a) { a } | (a, b) { b }]\n");
    assert!(
        text.contains("%multi(1: (a) {") && text.contains("2: (a, b) {"),
        "{text}"
    );
}

#[test]
fn forms_of_later_stages_are_errors() {
    bad("keep = (f : Int) <- (..env : N) { f }\n", "stage E6");
    bad("f = (x) { g(x) <- env }\n", "stage E6");
}

/// The Lisp surface: `std/src/test.is`, converted and read back, is the
/// tree that the file gives today (quotes included).
#[test]
fn lisp_surface_round_trip() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../std/src/test.is");
    let source = std::fs::read_to_string(path).unwrap();
    // The repository file is in today's rows until the conversion commit,
    // and in the rows of syntax three after it.
    let Ok(document) = ilang_syntax::lisp::parse(&source) else {
        return;
    };
    let Ok(old) = ilang_syntax::lisp::parse_module(&document) else {
        return;
    };
    let (converted, _) = syn3::convert::convert(&old, None, &syn3::convert::Signatures::default());
    let text = syn3::lisp::to_lisp(&converted);
    let (lowered, diags) = syn3::parse_lisp_lowered(&text);
    assert!(diags.is_empty(), "{diags:#?}");
    let shown = |m: &ilang_syntax::ast::Module| ilang_syntax::lisp::print_module(m);
    let (a, b) = (shown(&old), shown(&lowered));
    for (x, y) in a.lines().zip(b.lines()) {
        assert_eq!(x, y);
    }
    assert_eq!(a.lines().count(), b.lines().count());
}

/// A literal's written `-> R` leaves needs and failures open (inferred).
#[test]
fn literal_result_is_open() {
    let out = shown("f = () -> Int { 1 }\n");
    assert!(out.contains("-> Int ^ _ ! _"), "{out}");
}

/// The written full type of a binding closes needs and failures.
#[test]
fn written_full_type_is_closed() {
    let out = shown("f : () -> Int = () { 1 }\n");
    assert!(!out.contains('^') && !out.contains('!'), "{out}");
    assert!(out.contains("() -> Int"), "{out}");
}

/// A type parameter written twice is kept twice, for E0226.
#[test]
fn repeated_type_parameter_kept() {
    let out = shown("pair = <A, A>(x : A) { (x, x) }\n");
    assert!(out.contains("pair(A, A)"), "{out}");
}

#[test]
fn one_member_alias_keeps_a_type_written_as_a_type() {
    // `Alias(N) = T(N)` is an alias (§2.8, kind 4) when `T` is written as
    // a type; it does not take `T` in as its one constructor.
    same(
        "Dep = ()\nT(N) = (N)\nAlias(N) = T(N)\nf = (x : T(Dep)) { x }\n",
        "Dep { Dep }\nT(N) { T(N) }\nAlias(N) = T(N)\nf = (x : T(Dep)) { x }\n",
    );
    same(
        "T(N) = (N)\nAlias(N) = T(N)\ng = (x : Alias(Int)) { x }\ny : T(Int) = T(1)\n",
        "T(N) { T(N) }\nAlias(N) = T(N)\ng = (x : Alias(Int)) { x }\ny : T(Int) = T(1)\n",
    );
    // Not written as a type, the member is still the one constructor.
    same(
        "DivisionByZero = ()\nTimeError = DivisionByZero\n",
        "TimeError { DivisionByZero }\n",
    );
    same("A = (Int)\nB = ()\nAB = A | B\n", "AB { A(Int) B }\n");
}

#[test]
fn fail_in_a_function_head_is_its_error_part() {
    use ilang_syntax::ast::{ExprKind, ItemKind, Param, TypeKind};

    // `name: Fail(E)` in a head is the error part `^ E`, as in a need of a
    // function type (§2.10.6): no service parameter `Fail`.
    let text = shown(
        "load = (id : String) <- (clock : Clock, fail : Fail(String)) -> Int { 1 }\n\nkeep = () <- (fail : Fail(Int)) { 1 }\n\nboth = (0) <- (fail : Fail(Int)) { 1 } | (n : Int) -> Int { n }\n",
    );
    assert!(text.contains("clock ! Clock"), "{text}");
    assert!(!text.contains("Fail"), "{text}");
    assert!(text.contains("-> Int ^ String"), "{text}");
    assert!(text.contains("^ Int"), "{text}");
    // With no written result, keep the value and other needs inferred.
    // A later branch's Fail entry must also supply the chain's error part.
    for source in [
        "keep = () <- (fail : Fail(String)) { 1 }\n",
        "late = (0) { 1 } | (n : Int) <- (fail : Fail(String)) { n }\n",
    ] {
        let (module, diags) = syn3::parse_lowered(source);
        assert!(diags.is_empty(), "{source}\n{diags:#?}");
        let ItemKind::Binding(binding) = &module.items[0].kind else {
            panic!("expected a binding: {source}");
        };
        let ExprKind::Fn(function) = &binding.value.kind else {
            panic!("expected a function: {source}");
        };
        assert!(
            function
                .params
                .iter()
                .all(|p| !matches!(p, Param::Env { .. }))
        );
        let ret = function.ret.as_ref().expect("Fail supplies an error part");
        assert!(matches!(ret.value.kind, TypeKind::Hole));
        assert!(matches!(ret.env.as_ref().unwrap().kind, TypeKind::Hole));
        assert!(matches!(
            &ret.err.as_ref().unwrap().kind,
            TypeKind::Named { module: None, name, args }
                if name.name == "String" && args.is_empty()
        ));
    }
    bad(
        "f = () <- (a : Fail(Int), b : Fail(String)) { 1 }\n",
        "a need has one `Fail` entry",
    );
}

#[test]
fn labelled_function_type_parameters_keep_their_data_keys() {
    use ilang_syntax::ast::{ItemKind, TypeKind};
    let (module, errors) = syn3::parse_lowered("f : (x = Int, y = String) -> Int\n");
    assert!(errors.is_empty(), "{errors:?}");
    let ItemKind::Declaration { ty, .. } = &module.items[0].kind else {
        panic!()
    };
    let TypeKind::Fn { params, .. } = &ty.kind else {
        panic!()
    };
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].label.as_ref().unwrap().name, syn3::lower::DATA);
    let TypeKind::Record { fields, .. } = &params[0].ty.kind else {
        panic!()
    };
    assert_eq!(
        fields
            .iter()
            .map(|(l, _)| l.name.as_str())
            .collect::<Vec<_>>(),
        ["x", "y"]
    );
}

#[test]
fn migrated_captured_row_provider_keeps_its_written_contract() {
    // §4.7: annotations participate in inference. §9.2: the provided
    // value must satisfy the needed service by subtyping. Dropping the
    // old provider's S(Dependency) contract changes those constraints.
    let old = "S(N) { S(() -> Nil ! N) }\nDependency { Dependency }\nread = (d ! Dependency) { Nil }\nuse(N) = (s ! S(N)) { S(f) = s f() }\napply(N) = () { local : () -> Nil ! N = () { use() } local() }\nmain = () { ! S(Dependency) = S(() { read() }) apply() }";
    let (old, diags) = legacy::parser::parse_module_legacy(old);
    assert!(diags.is_empty(), "{diags:#?}");
    let (converted, notes) =
        syn3::convert::convert(&old, None, &syn3::convert::Signatures::default());
    assert!(notes.is_empty(), "{notes:#?}");
    let (lowered, diags) = syn3::lower::lower(&converted);
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(
        print_module(&lowered),
        shown(
            "S(N) = (() <- N -> Nil)\nDependency = ()\nread = () <- (d : Dependency) { Nil }\nuse = <N>() <- (s : S(N)) { S(f) = s f() }\napply = <N>() { local : () <- N -> Nil = () { use() } local() }\nmain = () {\n    _syn3_0 : S(Dependency) = S(() { read() })\n    apply() <- (_syn3_0)\n}",
        ),
    );
}

#[test]
fn a_single_use_provider_keeps_its_contract_before_generalization() {
    let source = "S(N) = (f = () <- N -> Int)\nwrap = (body) { provided : S(_) = S(f = () { 1 }) body() <- (provided) }";
    let (module, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let ilang_syntax::ast::ItemKind::Binding(binding) = &module.items[1].kind else {
        panic!()
    };
    let ilang_syntax::ast::ExprKind::Fn(function) = &binding.value.kind else {
        panic!()
    };
    assert_eq!(function.body.stmts.len(), 2);
    let ilang_syntax::ast::Stmt::Env { ty, value, .. } = &function.body.stmts[0] else {
        panic!()
    };
    assert!(
        matches!(&ty.kind, ilang_syntax::ast::TypeKind::Named { name, .. } if name.name == "S")
    );
    assert!(matches!(
        &value.kind,
        ilang_syntax::ast::ExprKind::Call { .. }
    ));

    let used = source.replace("body() <-", "body(provided) <-");
    let (module, errors) = ilang_syntax::parse_module(&used);
    assert!(errors.is_empty(), "{errors:?}");
    let ilang_syntax::ast::ItemKind::Binding(binding) = &module.items[1].kind else {
        panic!()
    };
    let ilang_syntax::ast::ExprKind::Fn(function) = &binding.value.kind else {
        panic!()
    };
    assert!(matches!(
        &function.body.stmts[0],
        ilang_syntax::ast::Stmt::Bind(_)
    ));
}

#[test]
fn a_closed_function_provider_keeps_its_generalization_boundary() {
    use ilang_syntax::ast::{ExprKind, ItemKind, Stmt, TypeKind};

    // §4.7/§4.8: preserve the closed function value's scheme and the
    // actual-provider checks deferred through it. Open contracts are
    // independently pinned by a_single_use_provider_keeps_its_contract_before_generalization.
    let source = "Reader(A) = (get = () -> A)\nwith_int = (body) { provided : Reader(Int) = Reader(get = () -> Int { 1 }) body() <- (provided) }";
    let (module, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let ItemKind::Binding(binding) = &module.items[1].kind else {
        panic!()
    };
    let ExprKind::Fn(function) = &binding.value.kind else {
        panic!()
    };
    assert_eq!(function.body.stmts.len(), 3);
    assert!(matches!(&function.body.stmts[0], Stmt::Bind(_)));
    let Stmt::Env { ty, value, .. } = &function.body.stmts[1] else {
        panic!()
    };
    assert!(matches!(ty.kind, TypeKind::Hole));
    assert!(
        matches!(&value.kind, ExprKind::Name { module: None, name } if name.name == "provided")
    );
}
