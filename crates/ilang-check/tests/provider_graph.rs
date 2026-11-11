use ilang_check::{
    Checked,
    providers::{self, ResolvedArgument, ResolvedPlan, Scope, Supply},
};
use ilang_package::{Diagnostic, PackageGraph, providers::Target};
use std::fs;

fn package(source: &str, plan: &str) -> (PackageGraph, Checked) {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.i"), source).unwrap();
    fs::write(dir.path().join("i.toml"), format!("name = \"app\"\n{plan}")).unwrap();
    let (graph, diags) = ilang_package::load(dir.path());
    assert!(!diags.iter().any(Diagnostic::is_error), "{diags:?}");
    let checked = ilang_check::check(&graph);
    (graph, checked)
}
fn resolve(g: &PackageGraph, c: &Checked, target: Target) -> Result<ResolvedPlan, Vec<Diagnostic>> {
    let mut plans = providers::resolve_plans(c, g, target);
    assert_eq!(plans.len(), 1, "expected one entry: {:?}", c.diags);
    plans.pop_first().unwrap().1
}
fn names(plan: &ResolvedPlan) -> Vec<&str> {
    plan.steps.iter().map(|s| s.service.name.as_str()).collect()
}
fn row(service: &str, factory: &str) -> String {
    format!("[providers.\"app/main.{service}\"]\nnative = \"app/main.{factory}\"\n")
}
fn error(g: &PackageGraph, c: &Checked, target: Target, mentions: &[&str]) {
    let ds = resolve(g, c, target).unwrap_err();
    assert!(ds.iter().any(|d| d.code == providers::PLAN_ERROR), "{ds:?}");
    for d in &ds {
        let _ = g.sources.render(d);
    }
    let text = ds
        .iter()
        .map(|d| d.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for name in mentions {
        assert!(text.contains(name), "missing {name:?} in {text}");
    }
}
#[test]
fn diamond_builds_shared_node_once() {
    let plan = [row("A", "a"), row("B", "b"), row("C", "c"), row("D", "d")].concat();
    let (g, c) = package(
        "A { A }\nB { B }\nC { C }\nD { D }\na = (b ! B, c ! C) { A }\nb = (d ! D) { B }\nc = (d ! D) { C }\nd = () { D }\n@main\nmain = (a ! A) { Nil }",
        &plan,
    );
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert_eq!(
        names(&p),
        ["app/main.D", "app/main.B", "app/main.C", "app/main.A"]
    );
    assert_eq!(p.steps[1].arguments, vec![ResolvedArgument::Service(0)]);
    assert_eq!(p.steps[2].arguments, vec![ResolvedArgument::Service(0)]);
}
#[test]
fn pure_bundle_entry_is_sync() {
    let (g, c) = package(
        "Bundle { Bundle }\nmake = () { Bundle }\n@main\nmain = (b ! Bundle) { Nil }",
        &row("Bundle", "make"),
    );
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert_eq!(names(&p), ["app/main.Bundle"]);
    assert!(!p.is_async);
}
#[test]
fn scheduler_and_runner_come_in_spec_order() {
    let (g, c) = package("(i/task)\n@main\nmain = () { task.yield() }", "");
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert_eq!(
        names(&p),
        ["i/request.Root", "i/task.Runner", "i/task.Scheduler"]
    );
    assert_eq!(p.steps[0].scope, Scope::Outer);
    assert_eq!(p.steps[1].scope, Scope::Outer);
    assert_eq!(p.adapter.as_ref().unwrap().enter_at, 2);
    assert!(p.is_async);
}
#[test]
fn order_is_topological() {
    let plan = [row("A", "a"), row("Z", "z"), row("B", "b")].concat();
    let (g, c) = package(
        "A { A }\nB { B }\nZ { Z }\na = (z ! Z) { A }\nz = () { Z }\nb = () { B }\n@main\nmain = (a ! A, b ! B) { Nil }",
        &plan,
    );
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert_eq!(names(&p), ["app/main.B", "app/main.Z", "app/main.A"]);
}
#[test]
fn cycle_names_entry_and_chain() {
    let (g, c) = package(
        "A { A }\nB { B }\na = (b ! B) { A }\nb = (a ! A) { B }\n@main\nmain = (a ! A) { Nil }",
        &[row("A", "a"), row("B", "b")].concat(),
    );
    error(
        &g,
        &c,
        Target::Native,
        &[
            "native",
            "app/main.main",
            "app/main.A -> app/main.B -> app/main.A",
            "recursive dependency",
        ],
    );
}
#[test]
fn missing_service_names_target_entry_service() {
    let (g, c) = package(
        "A { A }\nB { B }\na = (b ! B) { A }\n@main\nmain = (a ! A) { Nil }",
        &row("A", "a"),
    );
    error(
        &g,
        &c,
        Target::Native,
        &[
            "native",
            "app/main.main",
            "app/main.B",
            "app/main.A -> app/main.B",
        ],
    );
}
#[test]
fn missing_standard_capability_on_target() {
    let (g, c) = package("(i/fs)\n@main\nmain = (f ! fs.Fs) { Nil }", "");
    error(
        &g,
        &c,
        Target::Js,
        &["js", "app/main.main", "i/fs.Fs", "missing service"],
    );
}
#[test]
fn caught_unsupported_still_requires_provider() {
    let (g, c) = package(
        "(i/fs, i/fail)\n@main\nmain = () { fail.catch(() { fs.temp_dir() })\nNil }",
        "",
    );
    error(
        &g,
        &c,
        Target::Js,
        &["js", "app/main.main", "i/fs.Fs", "missing service"],
    );
}
#[test]
fn root_cell_overrides_shipped_cell() {
    let (g, c) = package(
        "(i/clock)\nreplacement = (c ! clock.Clock) { c }\n@main\nmain = (c ! clock.Clock) { Nil }",
        "[providers.\"i/clock.Clock\"]\nnative = \"app/main.replacement\"\n",
    );
    // An override is selected even when it creates a cycle; no shipped host
    // registration is substituted to break it.
    error(
        &g,
        &c,
        Target::Native,
        &["i/clock.Clock -> i/clock.Clock", "recursive dependency"],
    );
    let (mut g, c) = package(
        "(i/clock)\nfake = () -> clock.Clock { panic(\"fake\") }\n@main\nmain = (c ! clock.Clock) { Nil }",
        "[providers.\"i/clock.Clock\"]\nnative = \"app/main.fake\"\n",
    );
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert_eq!(names(&p), ["i/clock.Clock"]);
    assert!(matches!(p.steps[0].supply, Supply::Factory(_)));
    assert!(p.is_async);
    // Changing the shipped value after selection does not change authority.
    g.packages[g.library]
        .provider_plan
        .rows
        .iter_mut()
        .find(|r| r.service == "i/clock.Clock")
        .unwrap()
        .cells
        .remove(&Target::Native);
    assert_eq!(resolve(&g, &c, Target::Native).unwrap(), p);
}
#[test]
fn absent_root_cell_inherits_shipped() {
    let (g, c) = package(
        "(i/clock)\nunused = () -> clock.Clock { panic(\"fake\") }\n@main\nmain = (c ! clock.Clock) { Nil }",
        "[providers.\"i/clock.Clock\"]\njs = \"app/main.unused\"\n",
    );
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert_eq!(names(&p), ["i/clock.Clock"]);
    assert_eq!(p.steps[0].supply, Supply::Host);
}
#[test]
fn registered_row_without_cell_is_unsupplied() {
    let (g, c) = package(
        "@host(\"custom\", \"request\")\nRemote { Remote(read: () -> Request(Int)) }\n@main\nmain = (r ! Remote) { Nil }",
        "[providers.\"app/main.Remote\"]\n",
    );
    error(
        &g,
        &c,
        Target::Native,
        &[
            "native",
            "app/main.main",
            "app/main.Remote",
            "missing service",
        ],
    );
}
#[test]
fn duplicate_supply_is_error() {
    let (mut g, c) = package(
        "A { A }\na = () { A }\n@main\nmain = (a ! A) { Nil }",
        &row("A", "a"),
    );
    // Package aliases resolve to the same nominal identity despite distinct keys.
    let root = g.root.unwrap();
    g.packages[root]
        .dependencies
        .push(("self_alias".into(), Some(root)));
    let mut duplicate = g.packages[root].provider_plan.rows[0].clone();
    duplicate.service = "self_alias/main.A".into();
    g.packages[root].provider_plan.rows.push(duplicate);
    error(
        &g,
        &c,
        Target::Native,
        &[
            "native",
            "app/main.main",
            "duplicate supply",
            "self_alias/main.A",
        ],
    );
}
#[test]
fn factory_with_uncaught_error_is_rejected() {
    let (g, c) = package(
        "(i/fail)\nA { A }\na = () { fail.raise(\"bad\")\nA }\n@main\nmain = (a ! A) { Nil }",
        &row("A", "a"),
    );
    error(
        &g,
        &c,
        Target::Native,
        &["app/main.A", "uncaught typed errors"],
    );
}
#[test]
fn factory_must_return_its_service() {
    let (g, c) = package(
        "A { A }\na = () { 123 }\n@main\nmain = (a ! A) { Nil }",
        &row("A", "a"),
    );
    error(
        &g,
        &c,
        Target::Native,
        &["app/main.A", "must return its service"],
    );
}
#[test]
fn unchosen_factory_is_type_checked() {
    let (g, c) = package(
        "A { A }\nB { B }\na = () { A }\nb = () { 123 }\n@main\nmain = (a ! A) { Nil }",
        &[row("A", "a"), row("B", "b")].concat(),
    );
    error(
        &g,
        &c,
        Target::Native,
        &["app/main.B", "must return its service"],
    );
}
#[test]
fn unchosen_factory_needs_are_not_provisioned() {
    let (g, c) = package(
        "A { A }\nB { B }\nMissing { Missing }\na = () { A }\nb = (m ! Missing) { B }\n@main\nmain = (a ! A) { Nil }",
        &[row("A", "a"), row("B", "b")].concat(),
    );
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert_eq!(names(&p), ["app/main.A"]);
}
#[test]
fn host_cell_must_name_host_service() {
    let (g, c) = package(
        "A { A }\n@main\nmain = (a ! A) { Nil }",
        "[providers.\"app/main.A\"]\nnative = {host=true}\n",
    );
    error(
        &g,
        &c,
        Target::Native,
        &["app/main.A", "must name a declared host service"],
    );
}
#[test]
fn adapter_types_must_be_closed() {
    let (mut g, c) = package(
        "A(X) { A(X) }\nwrap = (body) { body() }\n@main\nmain = () { Nil }",
        "[provider_adapters.\"app/main.wrap\"]\nsupplies = [\"app/main.A(Int)\"]\nrequires = []\n",
    );
    g.packages[g.root.unwrap()].provider_plan.adapters[0].supplies[0] = "app/main.A(X)".into();
    error(&g, &c, Target::Native, &["adapter app/main.wrap", "closed"]);
}
#[test]
fn ill_kinded_type_argument() {
    let (g, c) = package(
        "A(X) { A(X) }\nwrap = (body) { body() }\n@main\nmain = () { Nil }",
        "[provider_adapters.\"app/main.wrap\"]\nsupplies = [\"app/main.A(Int(String))\"]\nrequires = []\n",
    );
    error(
        &g,
        &c,
        Target::Native,
        &["app/main.wrap", "ill-kinded", "Int(String)"],
    );
}
#[test]
fn entry_with_request_mode_provider_is_async() {
    let (g, c) = package(
        "Bundle { Bundle }\n@host(\"custom\", \"request\")\nRemote { Remote(read: () -> Request(Int)) }\nmake = (r ! Remote) { Bundle }\n@main\nmain = (b ! Bundle) { Nil }",
        &row("Bundle", "make"),
    );
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert!(p.is_async);
    assert_eq!(names(&p), ["app/main.Remote", "app/main.Bundle"]);
}
#[test]
fn standard_io_entry_gets_shipped_plan() {
    let (g, c) = package(
        "(i/clock, i/stdio)\n@main\nmain = () { clock.monotonic_ns()\nstdio.print(\"hello\") }",
        "",
    );
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert!(p.is_async);
    assert_eq!(
        names(&p),
        [
            "i/clock.Clock",
            "i/stdio.Stdio",
            "i/request.Await",
            "i/request.Root",
            "i/request.Owner"
        ]
    );
}
#[test]
fn providers_pending_fixture_plan() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/spec/9.11/providers_pending");
    let (g, ds) = ilang_package::load(&root);
    assert!(!ds.iter().any(Diagnostic::is_error), "{ds:?}");
    let c = ilang_check::check(&g);
    let p = resolve(&g, &c, Target::Host).unwrap();
    assert_eq!(names(&p), ["app/main.Work", "app/main.Bundle"]);
    assert!(p.is_async);
}

fn dependency_fixture(source: &str, plan: &str, dependency_plan: &str) -> (PackageGraph, Checked) {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::create_dir_all(dir.path().join("dep/src")).unwrap();
    fs::write(dir.path().join("src/main.i"), source).unwrap();
    fs::write(dir.path().join("dep/src/service.i"),"(Service(..), public)\nService { Service }\npublic = () { Service }\nprivate = () { Service }\n").unwrap();
    fs::write(
        dir.path().join("dep/i.toml"),
        format!("name = \"dep\"\n{dependency_plan}"),
    )
    .unwrap();
    fs::write(
        dir.path().join("i.toml"),
        format!("name = \"app\"\n[dependencies]\ndep = {{path=\"dep\"}}\n{plan}"),
    )
    .unwrap();
    let (g, ds) = ilang_package::load(dir.path());
    assert!(!ds.iter().any(Diagnostic::is_error), "{ds:?}");
    let c = ilang_check::check(&g);
    (g, c)
}
#[test]
fn private_dependency_path_in_root_cell_is_access_error() {
    let (g, c) = dependency_fixture(
        "(dep/service)\n@main\nmain = (s ! service.Service) { Nil }",
        "[providers.\"dep/service.Service\"]\nnative = \"dep/service.private\"\n",
        "",
    );
    error(
        &g,
        &c,
        Target::Native,
        &["dep/service.Service", "dep/service.private", "Private"],
    );
}
#[test]
fn dependency_cannot_override_root() {
    let (g, c) = dependency_fixture(
        "(dep/service)\nmake = () { service.Service }\n@main\nmain = (s ! service.Service) { Nil }",
        "[providers.\"dep/service.Service\"]\nnative = \"app/main.make\"\n",
        "[providers.\"dep/service.Service\"]\nnative = \"dep/service.private\"\n",
    );
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert_eq!(names(&p), ["dep/service.Service"]);
    let Supply::Factory(id) = p.steps[0].supply else {
        panic!("root factory not selected")
    };
    assert_eq!(c.program.as_ref().unwrap().def(id).name, "make");
}

#[test]
fn generic_instances_have_separate_nodes_and_instantiated_needs() {
    let source = "Cell(A) { Cell(A) }\nData(A) { Data(value: A) }\nLeft { Left }\nRight { Right }\ncell(A) = (d ! Data(A)) -> Cell(A) ! Data(A) { Cell(d.value) }\ndata(A) = () -> Data(A) { panic(\"data\") }\nleft = (c ! Cell(Int)) { Left }\nright = (c ! Cell(String)) { Right }\n@main\nmain = (l ! Left, r ! Right) { Nil }";
    let plan = [
        row("Cell", "cell"),
        row("Data", "data"),
        row("Left", "left"),
        row("Right", "right"),
    ]
    .concat();
    let (g, c) = package(source, &plan);
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert_eq!(
        names(&p),
        [
            "app/main.Data(Int)",
            "app/main.Cell(Int)",
            "app/main.Data(String)",
            "app/main.Cell(String)",
            "app/main.Left",
            "app/main.Right"
        ]
    );
}
#[test]
fn recursive_constructor_at_different_arguments_is_rejected() {
    let (g, c) = package(
        "Cell(A) { Cell(A) }\nmake(A) = (next ! Cell(List(A))) -> Cell(A) ! Cell(List(A)) { panic(\"no\") }\n@main\nmain = (c ! Cell(Int)) { Nil }",
        &row("Cell", "make"),
    );
    error(
        &g,
        &c,
        Target::Native,
        &[
            "recursive dependency",
            "app/main.Cell(Int) -> app/main.Cell(List(Int))",
        ],
    );
}
#[test]
fn adapter_scopes_do_not_share_instances() {
    let (g, c) = package("(i/task)\n@main\nmain = () { task.run(() { Nil }) }", "");
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert_eq!(
        names(&p),
        [
            "i/request.Root",
            "i/task.Runner",
            "i/request.Root",
            "i/task.Runner"
        ]
    );
    assert_eq!(
        p.steps.iter().map(|s| s.scope).collect::<Vec<_>>(),
        [Scope::Outer, Scope::Outer, Scope::Body, Scope::Body]
    );
    assert_eq!(p.adapter.as_ref().unwrap().enter_at, 2);
    assert!(matches!(
        p.steps[2].arguments[0],
        ResolvedArgument::Seed(Some(_))
    ));
    assert_eq!(p.steps[3].arguments[1], ResolvedArgument::Service(2));
}
#[test]
fn provider_nodes_are_preferred_to_ready_adapter_entry() {
    let (g, c) = package(
        "(i/task)\nA { A }\na = () { A }\n@main\nmain = (a ! A) { task.yield() }",
        &row("A", "a"),
    );
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert_eq!(
        names(&p),
        [
            "app/main.A",
            "i/request.Root",
            "i/task.Runner",
            "i/task.Scheduler"
        ]
    );
    assert_eq!(p.adapter.as_ref().unwrap().enter_at, 3);
}
#[test]
fn wasi_classification_keeps_the_blocking_abi() {
    let (g, c) = package(
        "@host(\"custom\", \"request\")\nRemote { Remote(read: () -> Request(Int)) }\n@main\nmain = (r ! Remote) { Nil }",
        "",
    );
    let p = resolve(&g, &c, Target::Wasi).unwrap();
    assert!(!p.is_async);
    assert!(p.steps[0].request_mode);
}
#[test]
fn completed_request_values_alone_do_not_classify_async() {
    let (g, c) = package(
        "(i/request)\nBundle { Bundle(value: Request(Int)) }\nmake = () { Bundle(request.ready(7)) }\n@main\nmain = (b ! Bundle) { Nil }",
        &row("Bundle", "make"),
    );
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert!(!p.is_async);
    assert_eq!(names(&p), ["app/main.Bundle"]);
}
#[test]
fn selected_factories_cannot_require_unavailable_seeds() {
    let (g, c) = package(
        "A { A }\nmake = (seeds: List(A)) -> A { panic(\"no\") }\n@main\nmain = (a ! A) { Nil }",
        "[providers.\"app/main.A\"]\nnative = {factory=\"app/main.make\", arguments=[\"seed\"], seed_required=true}\n",
    );
    error(
        &g,
        &c,
        Target::Native,
        &["app/main.A", "required seed unavailable"],
    );
}
#[test]
fn adapter_types_are_checked_even_without_reached_roles() {
    let (g, c) = package(
        "A { A }\nwrap = (body) { body() }\n@main\nmain = () { Nil }",
        "[provider_adapters.\"app/main.wrap\"]\nsupplies = [\"app/main.A\"]\nrequires = []\n",
    );
    error(
        &g,
        &c,
        Target::Native,
        &["adapter app/main.wrap", "generalized structured"],
    );
}
#[test]
fn discriminant_constructor_fields_are_dependency_edges() {
    let source = "Bundle { On(Part)\nOff }\nPart { Part }\npart = () { Part }\n@main\nmain = (b ! Bundle) { Nil }";
    let plan = format!(
        "{}[providers.\"app/main.Bundle\"]\ndiscriminant = {{js=\"app/main.Off\",nodejs=\"app/main.On\",native=\"app/main.On\",wasi=\"app/main.Off\",host=\"app/main.Off\"}}\n",
        row("Part", "part")
    );
    let (g, c) = package(source, &plan);
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert_eq!(names(&p), ["app/main.Part", "app/main.Bundle"]);
    assert!(matches!(p.steps[1].supply, Supply::Constructor(_)));
    assert_eq!(p.steps[1].arguments, [ResolvedArgument::Service(0)]);
}

#[test]
fn explicit_role_change_cannot_erase_inherited_async_role() {
    let (g, c) = package(
        "(i/task)\nfake = () -> task.Runner { panic(\"fake\") }\n@main\nmain = (r ! task.Runner) { Nil }",
        "[providers.\"i/task.Runner\"]\nrole = \"none\"\nnative = \"app/main.fake\"\n",
    );
    error(
        &g,
        &c,
        Target::Native,
        &["i/task.Runner", "changes inherited role"],
    );
}
#[test]
fn different_reached_adapters_are_ambiguous() {
    let source = "(i/task)\nA { A }\nmake = () { A }\nwrap(T,E,N) = (body: () -> T ? E ! N) -> T ? E ! N { body() }\n@main\nmain = (a ! A) { task.yield() }";
    let plan = "[providers.\"app/main.A\"]\nrole = \"runner\"\nadapter = \"app/main.wrap\"\nnative = \"app/main.make\"\n[provider_adapters.\"app/main.wrap\"]\nsupplies = []\nrequires = []\n";
    let (g, c) = package(source, plan);
    error(
        &g,
        &c,
        Target::Native,
        &[
            "ambiguous adapters",
            "app/main.A",
            "app/main.wrap",
            "i/task.Scheduler",
            "i/task.run",
        ],
    );
}
#[test]
fn unchosen_target_cells_still_check_their_factory_type() {
    let (g, c) = package(
        "A { A }\nmake = () { A }\nwrong = () { 7 }\n@main\nmain = (a ! A) { Nil }",
        "[providers.\"app/main.A\"]\nnative = \"app/main.make\"\njs = \"app/main.wrong\"\n",
    );
    error(
        &g,
        &c,
        Target::Native,
        &["app/main.A", "must return its service"],
    );
}

#[test]
fn adapter_service_list_order_does_not_change_its_type() {
    let (mut g, c) = package("(i/task)\n@main\nmain = () { task.yield() }", "");
    g.packages[g.library].provider_plan.adapters[0]
        .supplies
        .reverse();
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert_eq!(
        names(&p),
        ["i/request.Root", "i/task.Runner", "i/task.Scheduler"]
    );
}

#[test]
fn discriminant_constructor_aliases_resolve_before_validation() {
    let source = "Bundle { On(Part)\nOff }\nPart { Part }\npart = () { Part }\nselected = On\nalias = selected\n@main\nmain = (b ! Bundle) { Nil }";
    let plan = format!(
        "{}[providers.\"app/main.Bundle\"]\ndiscriminant = {{js=\"app/main.Off\",nodejs=\"app/main.alias\",native=\"app/main.alias\",wasi=\"app/main.Off\",host=\"app/main.Off\"}}\n",
        row("Part", "part")
    );
    let (g, c) = package(source, &plan);
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert_eq!(names(&p), ["app/main.Part", "app/main.Bundle"]);
    assert!(matches!(p.steps[1].supply, Supply::Constructor(_)));
}

#[test]
fn literal_service_arguments_use_i_string_escapes() {
    let (g, c) = package(
        "Cell(A) { Cell(A) }\nmake(A) = () -> Cell(A) { panic(\"no\") }\n@main\nmain = (c ! Cell(\"\\u{0}\" | \"x\")) { Nil }",
        &row("Cell", "make"),
    );
    let p = resolve(&g, &c, Target::Native).unwrap();
    assert_eq!(names(&p), ["app/main.Cell(\"\\u{0000}\" | \"x\")"]);
}
