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
(@ macro (: identity (-> [A] (, A) A)))
(@ macro (= identity (-> (, value) _ value)))
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
    let m = declared("m (-> (, Int) Int)", "(-> (, value) _ `7)");
    reject(
        &m,
        "(m = app/macros)\nmain = () { m.m(\"bad\") }",
        "not a subtype",
    );
}
#[test]
fn typed_macro_narrow_output_keeps_declared_type() {
    let m = declared("m (-> (, Int) Int)", "(-> (, value) _ `7)");
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
    let m = declared("m (-> (, Int) String)", "(-> (, value) _ value)");
    reject(
        &m,
        "(m = app/macros)\nmain = () { m.m(1) }",
        "cannot depend on its expansion",
    );
}
#[test]
fn typed_macro_result_only_variable_cannot_guess() {
    let m = declared("m (-> [A] (,) A)", "(-> (,) _ `\"\")");
    reject(
        &m,
        "(m = app/macros)\nf = () -> String { m.m() }\nmain = () { f() }",
        "result-only signature variable has no lower evidence",
    );
}
#[test]
fn typed_macro_labels_and_pipe() {
    let m = declared("m (-> (, Int Int) Int)", "(-> (, x y) _ `(+ ~x ~y))");
    checked(
        &m,
        "(m = app/macros)\nmain = () { assert(m.m(1, 2) == 3) assert({ 1 |> m.m(2) } == 3) }",
    )
    .unwrap();
}
#[test]
fn typed_macro_type_of_observes_literal() {
    let m = declared(
        "m (-> [A] (, A) Int)",
        "(-> (, value) _ {(= t ((. syntax type_of) value)) (|> ((. syntax subtype) t `Int) (| (-> (, (. syntax Yes)) _ `7) (-> (, _) _ ((. fail raise) ((. syntax SyntaxError) (= message \"expected integer type\") (= at value))))))})",
    );
    checked(&m, "(m = app/macros)\nmain = () { assert(m.m(42) == 7) }").unwrap();
}
#[test]
fn typed_macro_type_of_rejects_fabricated_syntax() {
    let m = declared(
        "m (-> (, Int) Int)",
        "(-> (, value) _ {((. syntax type_of) `1) value})",
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
        "m (-> [A] (, A) A)",
        "(-> (, value) _ {(= s ((. syntax signature))) (|> ((. syntax subtype) s s) (| (-> (, (. syntax Yes)) _ value) (-> (, _) _ ((. fail raise) ((. syntax SyntaxError) (= message \"signature is not reflexive\") (= at value))))))})",
    );
    checked(&m, "(m = app/macros)\nmain = () { assert(m.m(42) == 42) }").unwrap();
}
#[test]
fn typed_macro_members_preserve_finite_scalar_order() {
    let m = declared(
        "m (-> (, Int) Int)",
        "(-> (, value) _ (|> ((. syntax members) `(| 2 10 Nil)) (-> (, [a b c]) _ (|> (== ((. syntax show) a) \"10\") (| (-> (, True) _ value) (-> (, False) _ ((. fail raise) ((. syntax SyntaxError) (= message \"wrong member order\") (= at value)))))))))",
    );
    checked(&m, "(m = app/macros)\nmain = () { m.m(1) }").unwrap();
}
#[test]
fn typed_macro_rejects_type_position() {
    reject(
        ID,
        "(m = app/macros)\nAlias = m.identity(Int)\nmain = () { Nil }",
        "cannot give a type",
    );
}
#[test]
fn typed_macro_rejects_item_position() {
    reject(
        ID,
        "(m = app/macros)\nm.identity(1)\nmain = () { Nil }",
        "cannot make items",
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
    let m = declared("m (-> (, Int) _)", "(-> (, x) _ x)");
    reject(
        &m,
        "(m = app/macros)\nmain = () { Nil }",
        "fully written without _",
    );
}
#[test]
fn typed_macro_rejects_implementation_arity() {
    let m = declared("m (-> (, Int Int) Int)", "(-> (, x) _ x)");
    reject(
        &m,
        "(m = app/macros)\nmain = () { Nil }",
        "one parameter for each",
    );
}
#[test]
fn typed_macro_forwarding_record_differential() {
    let (_,c)=checked(ID,"(\n\tm = app/macros\n)\nf = (x: (a = Int, b = String)) { m.identity(x).a }\nmain = () { assert(f((a = 42, b = \"b\")) == 42) }").unwrap();
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
        "m (-> [A] (, A) (-> (,) A))",
        "(-> (, value) _ `(-> (,) _ ~value))",
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
        "m (-> [A] (, A) (-> (,) A))",
        "(-> (, value) _ `(-> (,) _ ~value))",
    );
    checked(
        &m,
        "(m = app/macros)\nmain = () { f = m.m(42) assert(f() == 42) }",
    )
    .unwrap();
}

fn same_principal(
    left: &ilang_macro::project::ProjectChecked,
    right: &ilang_macro::project::ProjectChecked,
    name: &str,
) {
    use ilang_check::{
        hir::{DefId, ModuleId, TypeRef},
        types::{SchemeArtifact, SchemeComparison},
    };
    fn get(c: &ilang_macro::project::ProjectChecked, name: &str) -> SchemeArtifact {
        let p = c.program.as_ref().unwrap();
        let (mi, m) = p
            .modules
            .iter()
            .enumerate()
            .find(|(_, m)| m.path == "app/main")
            .unwrap();
        let di = m.defs.iter().position(|d| d.name == name).unwrap();
        c.types.as_ref().unwrap().provider_scheme_artifacts[&DefId {
            module: ModuleId(mi as u32),
            index: di as u32,
        }]
            .clone()
    }
    let names = |c: &ilang_macro::project::ProjectChecked, t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().to_string(),
        TypeRef::Custom(id) => {
            let p = c.program.as_ref().unwrap();
            format!("{}/{}", p.module(p.ty(id).module).path, p.ty(id).name)
        }
    };
    let comparison = get(left, name)
        .compare(&get(right, name), &mut |t| names(left, t), &mut |t| {
            names(right, t)
        })
        .unwrap();
    assert!(
        matches!(comparison, SchemeComparison::Equivalent(_)),
        "{comparison:?}"
    );
}
#[test]
fn typed_macro_identity_principal_graph_differential() {
    let (_, typed) = checked(
        ID,
        "(m = app/macros)\nf = (x) { m.identity(x) }\nmain = () { assert(f(42) == 42) }",
    )
    .unwrap();
    let (_, plain) = checked(
        ID,
        "(m = app/macros)\nf = (x) { x }\nmain = () { assert(f(42) == 42) }",
    )
    .unwrap();
    same_principal(&typed, &plain, "f");
}
#[test]
fn typed_macro_forwarding_principal_record_graph_differential() {
    let (_,typed)=checked(ID,"(\n\tm = app/macros\n)\nf = (x: (a = Int, b = String)) { m.identity(x) }\nmain = () { f((a = 42, b = \"x\")) }").unwrap();
    let (_, plain) = checked(
        ID,
        "(\n\tm = app/macros\n)\nf = (x: (a = Int, b = String)) { x }\nmain = () { f((a = 42, b = \"x\")) }",
    )
    .unwrap();
    same_principal(&typed, &plain, "f");
}
#[test]
fn typed_macro_nested_forwarding_runs_retained_inner_call() {
    let dir = project(
        ID,
        "(m = app/macros)\nmain = () { assert(m.identity(m.identity(42)) == 42) }",
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
    assert!(matches!(
        ilang_host::Program::from_bytes(&b.wasm)
            .unwrap()
            .with_binding_plan(b.binding_plan)
            .unwrap()
            .run_entry(ilang_host::HostOptions::default())
            .unwrap(),
        ilang_host::Outcome::Ok(_)
    ));
}
#[test]
fn typed_macro_outer_drop_does_not_execute_inner_macro() {
    let macros = r#"(, (= syntax (/ i syntax)) (= fail (/ i fail)))
(, drop bad)
(@ macro (: drop (-> (, Int) Int)))
(@ macro (= drop (-> (, x) _ `7)))
(@ macro (: bad (-> (, Int) Int)))
(@ macro (= bad (-> (, x) _ ((. fail raise) ((. syntax SyntaxError) (= message "must not run") (= at x))))))
"#;
    let (_, c) = checked(
        macros,
        "(m = app/macros)\nmain = () { assert(m.drop(m.bad(1)) == 7) }",
    )
    .unwrap();
    assert_eq!(c.types.unwrap().typed_expansions.len(), 1);
}
#[test]
fn typed_macro_result_can_introduce_another_typed_call() {
    let macros = r#"(, (= syntax (/ i syntax)))
(, outer)
(@ macro (: outer (-> (, Int) Int)))
(@ macro (= outer (-> (, x) _ `(inner ~x))))
(@ macro (: inner (-> (, Int) Int)))
(@ macro (= inner (-> (, x) _ `(+ ~x 1))))
"#;
    let (_, c) = checked(
        macros,
        "(m = app/macros)\nmain = () { assert(m.outer(41) == 42) }",
    )
    .unwrap();
    assert_eq!(c.types.unwrap().typed_expansions.len(), 2);
}
#[test]
fn typed_macro_library_inside_syntax_generated_test_runs() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
        .args([
            "test",
            "--project",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/macro-tests"),
            "--opt",
            "0",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("passed, 0 failed"),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn typed_macro_forwarding_effect_rows_graph_differential() {
    let (_, typed) = checked(
        ID,
        "(m = app/macros)\nf = (g) { m.identity(g()) }\nmain = () { assert(f(() { 42 }) == 42) }",
    )
    .unwrap();
    let (_, plain) = checked(
        ID,
        "(m = app/macros)\nf = (g) { g() }\nmain = () { assert(f(() { 42 }) == 42) }",
    )
    .unwrap();
    same_principal(&typed, &plain, "f");
}
#[test]
fn typed_macro_members_give_each_closed_data_member_once() {
    // syntax-3 stage 6: a union of closed data has one member per shape
    // (§2.3, §6.2); it is no longer one record graph.
    let m = declared(
        "m (-> [A] (, A) A)",
        "(-> (, value) _ {(= t ((. syntax type_of) value)) (|> ((. syntax members) t) (| (-> (, [one two]) _ value) (-> (, _) _ ((. fail raise) ((. syntax SyntaxError) (= message \"closed data members not one per shape\") (= at value))))))})",
    );
    checked(
        &m,
        "(\n\tm = app/macros\n)\nf = (x: (a = Int) | (b = String)) { m.m(x) }\nmain = () { f((a = 42)) }",
    )
    .unwrap();
}
#[test]
fn typed_macro_frozen_variable_is_legal_in_output_annotation() {
    let m = declared(
        "m (-> [A] (, A) A)",
        "(-> (, value) _ {(= t ((. syntax type_of) value)) `{(= (: saved ~t) ~value) saved}})",
    );
    checked(
        &m,
        "(m = app/macros)\nf = (x) { m.m(x) }\nmain = () { assert(f(42) == 42) }",
    )
    .unwrap();
}

#[test]
fn typed_macro_members_accept_symbolic_difference_without_new_bounds() {
    let m = declared(
        "m (-> [A] (, A) A)",
        "(-> (, value) _ {(= t ((. syntax type_of) value)) (= difference `(- ~t Int)) (|> ((. syntax members) difference) (| (-> (, [one]) _ value) (-> (, _) _ ((. fail raise) ((. syntax SyntaxError) (= message \"symbolic difference split\") (= at value))))))})",
    );
    checked(
        &m,
        "(m = app/macros)\nf = (x) { m.m(x) }\nmain = () { assert(f(42) == 42) }",
    )
    .unwrap();
}

#[test]
fn typed_macro_forwarding_keeps_original_value_type_reads() {
    checked(ID,"(m = app/macros)\nf = (x: Int) { m.identity(() -> x { x })() }\nmain = () { assert(f(42) == 42) }").unwrap();
}
#[test]
fn typed_macro_argument_binder_cannot_escape() {
    let m = declared(
        "m (-> (, (-> (, Int) Int)) Int)",
        "(-> (, value) _ (|> value (| (-> (, `(-> (, ~parameter) ~ret {~body})) _ body) (-> (, _) _ value))))",
    );
    reject(
        &m,
        "(m = app/macros)\nmain = () { m.m((x) { x }) }",
        "cannot escape an argument binder",
    );
}
#[test]
fn typed_macro_partial_argument_rebuild_retains_original_binders() {
    let m = declared(
        "m (-> (, (-> (, Int) Int)) (-> (, Int) Int))",
        "(-> (, value) _ (|> value (| (-> (, `(-> (, ~parameter) ~ret {~body})) _ `(-> (, ~parameter) ~ret {~body})) (-> (, _) _ value))))",
    );
    checked(
        &m,
        "(m = app/macros)\nmain = () { f = m.m((x) { x }) assert(f(42) == 42) }",
    )
    .unwrap();
}

#[test]
fn typed_macro_nested_queries_keep_parent_type_handles() {
    let macros = r#"(, (= syntax (/ i syntax)))
(, outer)
(@ macro (: outer (-> [A] (, A) A)))
(@ macro (= outer (-> (, value) _ {(= t ((. syntax type_of) value)) `(inner {(= (: saved ~t) ~value) saved})})))
(@ macro (: inner (-> [A] (, A) A)))
(@ macro (= inner (-> (, value) _ (|> value (| (-> (, `{(= (: ~name ~annotation) ~arg) ~last}) _ {(= t ((. syntax type_of) arg)) (|> ((. syntax subtype) annotation t) (| (-> (, (. syntax Yes)) _ value) (-> (, _) _ value)))}) (-> (, _) _ value))))))
"#;
    checked(
        macros,
        "(m = app/macros)\nf = (x) { m.outer(x) }\nmain = () { assert(f(42) == 42) }",
    )
    .unwrap();
}

#[test]
fn typed_macro_generic_choice_reports_restriction() {
    let m = declared(
        "m (-> [A] (, A) Int)",
        "(-> (, value) _ {(= t ((. syntax type_of) value)) (|> ((. syntax subtype) t `Int) (| (-> (, (. syntax Yes)) _ `7) (-> (, _) _ ((. fail raise) ((. syntax SyntaxError) (= message \"no choice\") (= at value))))))})",
    );
    reject(
        &m,
        "(m = app/macros)\nf = (x) { m.m(x) }\nmain = () { f(1) }",
        "cannot choose by the type inside a binding with type variables",
    );
}

/// Expansion and the built program are pure functions of the project: the same
/// input expanded and built twice gives identical expanded modules and identical
/// Wasm. Several modules expand in order, so the macro index is brought up to
/// date between them, and a typed macro runs during checking.
#[test]
fn expansion_and_wasm_are_deterministic() {
    let macros = format!(
        "{}(@ macro (= twice (-> (, e) _ `(+ ~e ~e))))\n",
        ID.replacen("(, identity)", "(, identity twice)", 1)
    );
    let dir = project(
        &macros,
        "(m = app/macros, mid = app/mid)\nmain = () { assert(mid.f(m.identity(21)) == 84) }\n",
    );
    std::fs::write(
        dir.path().join("src/mid.i"),
        "(m = app/macros)\n(f)\nf = (x) { m.twice(m.twice(x)) }\n",
    )
    .unwrap();
    let expand = || {
        let (mut graph, ds) = ilang_cli::load(dir.path(), false, true);
        assert!(!ds.iter().any(Diagnostic::is_error), "{ds:?}");
        let expanded = match ilang_macro::project::expand(&mut graph) {
            Ok(expanded) => expanded,
            Err(ds) => panic!("{}", graph.sources.render_all(&ds).join("\n")),
        };
        expanded
            .parsed
            .iter()
            .map(|p| {
                format!(
                    "{}\n{}\n{:?}",
                    p.path,
                    graph.sources.get(p.source).text,
                    p.ast
                )
            })
            .collect::<Vec<_>>()
    };
    let first = expand();
    assert!(first.iter().any(|m| m.contains("app/mid")));
    assert_eq!(first, expand(), "expansion differs between two runs");
    let build = || {
        ilang_cli::build(
            dir.path(),
            ilang_cli::BuildOptions {
                tests: false,
                opt: ilang_cli::Level::O0,
                offline: true,
                target: ilang_cli::Target::Host,
            },
        )
        .unwrap_or_else(|_| panic!("project builds"))
        .wasm
    };
    let wasm = build();
    assert!(wasm == build(), "wasm differs between two builds");
}
