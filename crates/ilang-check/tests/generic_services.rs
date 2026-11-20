//! Finite service rows.
mod common;

use ilang_check::Checked;

fn check(source: &str) -> (ilang_package::PackageGraph, Checked) {
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    (graph, checked)
}

fn accepts(source: &str, name: &str, expected: &str, unused: &[&str]) {
    let (_, checked) = check(source);
    common::expect_unused(&checked.diags, unused);
    let program = checked.program.unwrap();
    let types = checked.types.unwrap();
    let module = common::module(&program, "app/main");
    assert!(
        types
            .lines(&program, module)
            .contains(&format!("app/main.{name} : {expected}")),
        "{:?}",
        types.lines(&program, module)
    );

    // Every positive K4 fixture also runs through entry validation. Remove
    // its supplied services: inference retains the requirements, and an entry must
    // reject an entry with no provider. Specialize its value parameter to
    // Int where the fixture has one.
    let mut missing = source
        .lines()
        .filter(|line| !line.trim_start().starts_with('!'))
        .collect::<Vec<_>>()
        .join("\n");
    let arg = if expected.starts_with("()") { "" } else { "1" };
    missing.push_str(
        &"\n\nmain = () { call(argument) }\n"
            .replace("call", name)
            .replace("argument", arg),
    );
    let (graph, checked) = check(&missing);
    let errors: Vec<_> = checked.diags.iter().filter(|d| d.is_error()).collect();
    assert!(
        errors.is_empty(),
        "{name}: {:?}",
        graph.sources.render_all(&checked.diags)
    );
    let program = checked.program.unwrap();
    let types = checked.types.unwrap();
    assert!(
        types
            .lines(&program, common::module(&program, "app/main"))
            .iter()
            .any(|line| line.starts_with("app/main.main : ")),
        "an entry type must not disappear"
    );
}

#[test]
fn annotated_and_inferred_service_rows() {
    // Function inference unions the callback's N with S(N). Writing that same union
    // must not change the finite principal type.
    for annotation in ["{", "-> () {"] {
        accepts(
            &"S(N) = (() <- N -> ())\nuse = <N>() <- (s : S(N)) -> () {\n\tS(f) = s\n\tf()\n}"
                .replace("-> () {", annotation),
            "use",
            "() <- (S(N), ..N) -> ()",
            &["use"],
        );
    }
}

#[test]
fn generic_service_through_a_helper() {
    // Instantiation with N=Never lets provision remove S(Never).
    accepts(
        "S(N) = (() <- N -> (), (N) -> ())\nuse = <N>() <- (s : S(N)) {\n\tS(f, _) = s\n\tf()\n}\nrun = <N>(body : () <- N -> ()) { body() }\nexample = () { run(() { use() }) <- (S(() { () }, (_ : Never) { () })) }",
        "example",
        "() -> ()",
        &["example"],
    );
}

#[test]
fn open_service_row_through_helpers() {
    // Calls carry both requirements through calls; no closed service
    // annotation is needed to make the row finite.
    for annotation in ["{", "-> () {"] {
        accepts(
            &"S(N) = (() <- N -> (), (N) -> ())\nuse = <N>() <- (s : S(N)) {\n\tS(f, _) = s\n\tf()\n}\nrun = <N>(body : () <- N -> ()) { body() }\nforward = <N>() <- (s : S(N)) -> () { run(() { use() }) }"
                .replace("-> () {", annotation),
            "forward",
            "() <- (S(N), ..N) -> ()",
            &["forward"],
        );
    }
}

#[test]
fn several_parameters_and_nested_services() {
    // Inference constraints apply equally inside nested constructors and with
    // independent value/error/environment parameters. Instantiation permits
    // independent closed instances of the resulting scheme.
    for annotation in ["{", "-> A {"] {
        let source = "Inner(A, E, N) = ((A) <- (Fail(E), ..N) -> A, (N) -> ())\nOuter(A, E, N) = (Inner(A, E, N))\nuse = <A, E, N>(x : A) <- (s : Outer(A, E, N)) -> A {\n\tOuter(Inner(f, _)) = s\n\tf(x)\n}\nrun = <A, N>(body : () <- N -> A) { body() }\ninteger = () {\n\trun(() { use(1) }) <- (Outer(Inner((x : Int) { x }, (_ : Never) { () })))\n}\nboolean = () {\n\trun(() { use(True()) }) <- (Outer(Inner((x : Bool) { x }, (_ : Never) { () })))\n}"
        .replace("-> A {", annotation);
        accepts(
            &source,
            "integer",
            "() <- (Fail(Never)) -> Int",
            &["integer", "boolean"],
        );
        accepts(
            &source,
            "boolean",
            "() <- (Fail(Never)) -> Bool",
            &["integer", "boolean"],
        );
        accepts(
            &source,
            "use",
            "(A) <- (Fail(B), Outer(A, B, N), ..N) -> A",
            &["integer", "boolean"],
        );
    }
}

#[test]
fn nonempty_stored_environment() {
    // A callback takes its dependency at the call site; provision
    // removes each installed service, including a service nested in N.
    accepts(
        "Dependency = (value = Int)\nget = () <- (d : Dependency) { d.value }\nInner(N) = (() <- N -> Int, (N) -> ())\nread = <N>() <- (s : Inner(N)) {\n\tInner(f, _) = s\n\tf()\n}\nOuter(N) = (() <- N -> Int, (N) -> ())\nuse = <N>() <- (s : Outer(N)) -> Int {\n\tOuter(f, _) = s\n\tf()\n}\nrun = <A, N>(body : () <- N -> A) { body() }\nexample = () {\n\trun(() { use() }) <- (\n\t\tDependency(value = 7),\n\t\tInner(() { get() }, (_ : Dependency) { () }),\n\t\tOuter(() { read() }, (_ : { Dependency | Inner(Dependency) }) { () }),\n\t)\n}",
        "example",
        "() -> Int",
        &["example"],
    );
}

#[test]
fn logging_example() {
    // E=String, N=Never. fail.catch reifies the error,
    // and provision removes Sink(String, Never).
    for annotation in ["{", "-> () {"] {
        accepts(
            &"(\n\ti/fail\n)\nEntry = (message = String)\nSink(E, N) = ((Entry) <- (Fail(E), ..N) -> (), (N) -> ())\nmake = <E, N>(send : (Entry) <- (Fail(E), ..N) -> ()) -> Sink(E, N) {\n\tSink(send, (_ : N) { () })\n}\nwrite = <E, N>(entry : Entry) <- (sink : Sink(E, N)) -> () {\n\tSink(send, _) = sink\n\tsend(entry)\n}\nexample = () {\n\tfail.catch(() { write(Entry(message = \"seen\")) }) <- (\n\t\tmake((entry) { fail.raise(entry.message) }),\n\t)\n}"
            .replace("-> () {", annotation),
            "example",
            "() -> Result((), String)",
            &["example"],
        );
    }
}

fn rejects(source: &str, code: &str) {
    let (graph, checked) = check(source);
    assert!(
        checked.diags.iter().any(|d| d.code == code),
        "expected {code}: {:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn recursive_types_still_fail() {
    // "A type that has itself inside it, through a type variable,
    // is a compile error." These require an actual constructor cycle,
    // rather than the tautology N <: N | S(N).
    for source in [
        "bad = (x) { x(x) }",
        // Constructor cycles must still fail when callback requirements
        // pass through explicitly supplied services.
        "S(N) = (() <- N -> (), (N) -> ())\nuse = <N>() <- (s : S(N)) {\n\tS(f, _) = s\n\tf()\n}\nbad = <N>() { use() <- (S(() { use() }, (_ : N) { () })) }",
    ] {
        rejects(source, "E0505");
    }
}

#[test]
fn incompatible_service_arguments_still_fail() {
    // The input marker keeps N invariant even when
    // Consumer's parameter is contravariant.
    rejects(
        "Entry = ()\nSink(E, N) = ((Entry) <- (Fail(E), ..N) -> (), (N) -> ())\nConsumer(A) = ((A) -> ())\nbad = (s : Sink(Never, Consumer(Int))) -> Sink(Never, Consumer(Never)) { s }",
        "E0515",
    );
    // Sink's covariant E can meet at Never because its stored
    // callback is inhabited. Adding a required E value makes that same
    // intersection empty; retain both direct and helper forms.
    for source in [
        "Sink(E, N) = (() <- (Fail(E), ..N) -> (), (N) -> ())\none = () <- (s : Sink(String, Never)) { () }\ntwo = () <- (s : Sink(Int, Never)) { () }\nbad = () {\n\tone()\n\ttwo()\n}",
        "Sink(E, N) = (() <- (Fail(E), ..N) -> (), (N) -> ())\none = () <- (s : Sink(String, Never)) { () }\ntwo = () <- (s : Sink(Int, Never)) { () }\nrun = <N>(body : () <- N -> ()) { body() }\nbad = () {\n\trun(() { one() })\n\ttwo()\n}",
    ] {
        let (graph, checked) = check(source);
        assert!(
            !checked.diags.iter().any(|d| d.is_error()),
            "{:?}",
            graph.sources.render_all(&checked.diags)
        );
        let program = checked.program.unwrap();
        let shown = checked
            .types
            .unwrap()
            .lines(&program, common::module(&program, "app/main"));
        assert!(
            shown
                .iter()
                .any(|s| s.starts_with("app/main.bad :") && s.contains("Sink(Never, Never)")),
            "{shown:?}"
        );
        rejects(
            &source.replace("Sink(E, N) = (()", "Sink(E, N) = (E, ()"),
            "E0515",
        );
    }
}

#[test]
fn repeated_helpers_keep_a_finite_principal_row() {
    // Repeated rule-3 instantiation and rule-5 forwarding add no service
    // and must preserve the same principal scheme.
    let mut source = String::from(
        "S(N) = (() <- N -> (), (N) -> ())\nuse = <N>() <- (s : S(N)) {\n\tS(f, _) = s\n\tf()\n}\nrun = <N>(body : () <- N -> ()) { body() }\nforward_0 = () { run(() { use() }) }\n",
    );
    for n in 1..64 {
        source.push_str(
            &"forward_next = () { run(() { forward_previous() }) }\n"
                .replace("forward_next", &format!("forward_{n}"))
                .replace("forward_previous", &format!("forward_{}", n - 1)),
        );
    }
    accepts(
        &source,
        "forward_63",
        "() <- (S(N), ..N) -> ()",
        &["forward_63"],
    );
}

#[test]
fn catching_an_unknown_callback_keeps_its_error_cases_reachable() {
    let (_, checked) = check(
        "(i/fail)\nprobe = (handler) { fail.catch(() { handler() }) |> (Ok(value)) { value } & (Error(_: String)) { 0 } & (Error(_)) { 1 } }",
    );
    common::expect_unused(&checked.diags, &["probe"]);
    let (_, checked) = check(
        "(i/fail)\npure = () { fail.catch(() { 1 }) |> (Ok(value)) { value } & (Error(_)) { 0 } }",
    );
    assert!(
        checked.diags.iter().any(|d| d.code == "W0501"),
        "a pure callback's error branch is still unreachable"
    );
}
