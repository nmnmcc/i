//! Typed macro barriers preserve the same principal inference as ordinary calls.
use ilang_package::Diagnostic;

fn project(macros: &str, main: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::write(dir.path().join("src/macros.is"), macros).unwrap();
    std::fs::write(dir.path().join("src/main.i"), main).unwrap();
    dir
}
const ID: &str = r#"(, (= syntax (/ i syntax)))
(, identity)
(@ macro (: (identity A) (-> ((: value A)) A)))
(@ macro (= identity (-> (value) _ value)))
"#;
#[test]
fn typed_macro_identity_differential() {
    let dir = project(
        ID,
        "(m = app/macros)\nid = (x) { m.identity(x) }\nmain = () { assert(id(42) == 42) }\n",
    );
    let (mut graph, ds) = ilang_cli::load(dir.path(), false, true);
    assert!(!ds.iter().any(Diagnostic::is_error), "{ds:?}");
    let checked = ilang_macro::project::check(&mut graph);
    assert!(
        !checked.diags.iter().any(Diagnostic::is_error),
        "{}",
        graph.sources.render_all(&checked.diags).join("\n")
    );
    let typed = checked.types.unwrap();
    assert_eq!(typed.typed_expansions.len(), 1);
}

fn checked(
    macros: &str,
    source: &str,
) -> Result<
    (
        ilang_package::PackageGraph,
        ilang_macro::project::ProjectChecked,
    ),
    String,
> {
    let dir = project(macros, source);
    let (mut graph, mut ds) = ilang_cli::load(dir.path(), false, true);
    let checked = ilang_macro::project::check(&mut graph);
    ds.extend(checked.diags.clone());
    if ds.iter().any(Diagnostic::is_error) {
        Err(graph.sources.render_all(&ds).join("\n"))
    } else {
        Ok((graph, checked))
    }
}
fn declared(signature: &str, body: &str) -> String {
    format!(
        "(, (= syntax (/ i syntax)) (= fail (/ i fail)))\n(, m)\n(@ macro (: {signature}))\n(@ macro (= m {body}))\n"
    )
}
fn reject(macros: &str, source: &str, message: &str) {
    let Err(error) = checked(macros, source) else {
        panic!("expected rejection: {message}")
    };
    assert!(error.contains(message), "expected {message:?}:\n{error}");
}
#[test]
fn typed_macro_signature_checks_dropped_arguments() {
    let m = declared("m (-> (Int) Int)", "(-> (value) _ `7)");
    reject(
        &m,
        "(m = app/macros)\nmain = () { m.m(\"bad\") }",
        "not a subtype",
    );
}
#[test]
fn typed_macro_narrow_output_keeps_declared_type() {
    let m = declared("m (-> (Int) Int)", "(-> (value) _ `7)");
    let (_, c) = checked(
        &m,
        "(m = app/macros)\nf = () { m.m(1) }\nmain = () { assert(f() == 7) }",
    )
    .unwrap();
    let p = c.program.unwrap();
    let d = p
        .modules
        .iter()
        .enumerate()
        .find(|(_, m)| m.path == "app/main")
        .unwrap();
    let index = d.1.defs.iter().position(|d| d.name == "f").unwrap();
    assert_eq!(
        c.types.unwrap().schemes[&ilang_check::hir::DefId {
            module: ilang_check::hir::ModuleId(d.0 as u32),
            index: index as u32
        }],
        "() -> Int"
    );
}
#[test]
fn typed_macro_output_cannot_change_result_type() {
    let m = declared("m (-> (Int) String)", "(-> (value) _ value)");
    reject(
        &m,
        "(m = app/macros)\nmain = () { m.m(1) }",
        "invalid typed macro expansion",
    );
}
#[test]
fn typed_macro_result_only_variable_cannot_guess() {
    let m = declared("(m A) (-> () A)", "(-> () _ `\"\")");
    reject(
        &m,
        "(m = app/macros)\nf = () -> String { m.m() }\nmain = () { f() }",
        "invalid typed macro expansion",
    );
}
#[test]
fn typed_macro_labels_and_pipe() {
    let m = declared(
        "m (-> ((: left Int) (: right Int)) Int)",
        "(-> (x y) _ `(+ ~x ~y))",
    );
    checked(&m,"(m = app/macros)\nmain = () { assert(m.m(right: 2, left: 1) == 3) assert({ 1 |> m.m(right: 2) } == 3) }").unwrap();
}
#[test]
fn typed_macro_type_of_observes_literal() {
    let m = declared(
        "(m A) (-> (A) Int)",
        "(-> (value) _ { (= t ((. syntax type_of) value)) (| ((. syntax subtype) t `Int) { (-> (. syntax Yes) `7) (-> _ ((. fail raise) ((. syntax SyntaxError) \"expected integer type\" value))) }) })",
    );
    checked(&m, "(m = app/macros)\nmain = () { assert(m.m(42) == 7) }").unwrap();
}
#[test]
fn typed_macro_type_of_rejects_fabricated_syntax() {
    let m = declared(
        "m (-> (Int) Int)",
        "(-> (value) _ { ((. syntax type_of) `1) value })",
    );
    reject(
        &m,
        "(m = app/macros)\nmain = () { m.m(1) }",
        "type_of needs an original",
    );
}
#[test]
fn typed_macro_signature_query_is_reflexive() {
    let m = declared(
        "(m A) (-> (A) A)",
        "(-> (value) _ { (= s ((. syntax signature))) (| ((. syntax subtype) s s) { (-> (. syntax Yes) value) (-> _ ((. fail raise) ((. syntax SyntaxError) \"signature is not reflexive\" value))) }) })",
    );
    checked(&m, "(m = app/macros)\nmain = () { assert(m.m(42) == 42) }").unwrap();
}
#[test]
fn typed_macro_members_preserve_finite_scalar_order() {
    let m = declared(
        "m (-> (Int) Int)",
        "(-> (value) _ { (| ((. syntax members) `(| 2 10 Nil)) { (-> [a b c] (| (== ((. syntax show) a) \"10\") { (-> True value) (-> False ((. fail raise) ((. syntax SyntaxError) \"wrong member order\" value))) })) }) })",
    );
    checked(&m, "(m = app/macros)\nmain = () { m.m(1) }").unwrap();
}
#[test]
fn typed_macro_rejects_type_position() {
    reject(
        ID,
        "(m = app/macros)\nAlias = m.identity(Int)\nmain = () { Nil }",
        "only as an expression call",
    );
}
#[test]
fn typed_macro_rejects_item_position() {
    reject(
        ID,
        "(m = app/macros)\nm.identity(1)\nmain = () { Nil }",
        "only as an expression call",
    );
}
#[test]
fn typed_macro_rejects_value_position() {
    reject(
        ID,
        "(m = app/macros)\nf = m.identity\nmain = () { Nil }",
        "only as an expression call",
    );
}
#[test]
fn typed_macro_rejects_signature_holes() {
    let m = declared("m (-> (Int) _)", "(-> (x) _ x)");
    reject(
        &m,
        "(m = app/macros)\nmain = () { Nil }",
        "fully written without _",
    );
}
#[test]
fn typed_macro_rejects_implementation_arity() {
    let m = declared("m (-> (Int Int) Int)", "(-> (x) _ x)");
    reject(
        &m,
        "(m = app/macros)\nmain = () { Nil }",
        "one parameter for each",
    );
}
#[test]
fn typed_macro_forwarding_record_differential() {
    let (_,c)=checked(ID,"(m = app/macros)\nf = (x: (a: Int, b: String)) { m.identity(x).a }\nmain = () { assert(f((a: 42, b: \"b\")) == 42) }").unwrap();
    assert_eq!(c.types.unwrap().typed_expansions.len(), 1);
}
#[test]
fn typed_macro_generic_binding_expands_once() {
    let (_,c)=checked(ID,"(m = app/macros)\nid = (x) { m.identity(x) }\nmain = () { assert(id(1) == 1) assert(id(\"x\") == \"x\") }").unwrap();
    assert_eq!(c.types.unwrap().typed_expansions.len(), 1);
}
#[test]
fn typed_macro_builds_and_runs_forwarded_argument() {
    let dir = project(
        ID,
        "(m = app/macros)\nmain = () { assert(m.identity(42) == 42) }\n",
    );
    let b = ilang_cli::build(
        dir.path(),
        ilang_cli::BuildOptions {
            tests: false,
            opt: ilang_cli::Level::O0,
            offline: true,
            target: ilang_cli::Target::Native,
        },
    )
    .unwrap();
    let p = ilang_host::Program::from_bytes(&b.wasm).unwrap();
    let mut imports = ilang_host::HostImports::default();
    imports.grant_package(&b.binding_plan, &b.trusted_package);
    let mut instance = p
        .with_binding_plan(b.binding_plan)
        .unwrap()
        .instantiate(ilang_host::HostOptions::new(imports))
        .unwrap();
    assert!(matches!(
        instance.run_entry().unwrap(),
        ilang_host::Outcome::Ok(_)
    ));
}

#[test]
fn typed_macro_cannot_delay_effectful_argument() {
    let m = declared(
        "(m A) (-> (A) (-> () A))",
        "(-> (value) _ `(-> () _ ~value))",
    );
    reject(
        &m,
        "(m = app/macros)\nmain = () { m.m(assert(True)) }",
        "cannot delay an argument",
    );
}
#[test]
fn typed_macro_can_delay_pure_argument() {
    let m = declared(
        "(m A) (-> (A) (-> () A))",
        "(-> (value) _ `(-> () _ ~value))",
    );
    checked(
        &m,
        "(m = app/macros)\nmain = () { f = m.m(42) assert(f() == 42) }",
    )
    .unwrap();
}
