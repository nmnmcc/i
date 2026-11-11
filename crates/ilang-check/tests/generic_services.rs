//! Finite service rows (§4.7, §4.8 rules 3–5 and 9, §9.2).
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
    // its supplied services: §4.8 retains the requirements, and §10.2 must
    // reject an entry with no provider. Specialize its value parameter to
    // Int where the fixture has one (§4.8 rule 3).
    let mut missing = source
        .lines()
        .filter(|line| !line.trim_start().starts_with('!'))
        .collect::<Vec<_>>()
        .join("\n");
    let arg = if expected.starts_with("()") { "" } else { "1" };
    missing.push_str(
        &"\n@main\nmain = () { call(argument) }\n"
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
    // Rule 4 unions the callback's N with S(N). Writing that same union
    // must not change the finite principal type (§4.7).
    for annotation in ["", "-> Nil ! N | S(N)"] {
        accepts(
            &"S(N) { S(() -> Nil ! N) }\nuse(N) = (s ! S(N)) -> Nil ! N | S(N) { S(f) = s f() }"
                .replace("-> Nil ! N | S(N)", annotation),
            "use",
            "() -> Nil ! N | S(N)",
            &["use"],
        );
    }
}

#[test]
fn generic_service_through_a_helper() {
    // Rules 3 and 5 instantiate use with N=Never; rule 9 removes S(Never).
    accepts(
        "S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(N) = (body: () -> Nil ! N) { body() }
example = () {
  ! S(Never) = S(() { Nil }, (_: Never) { Nil })
  run(() { use() })
}",
        "example",
        "() -> Nil",
        &["example"],
    );
}

#[test]
fn open_service_row_through_helpers() {
    // Rule 5 carries both requirements through calls; no closed service
    // annotation is needed to make the row finite.
    for annotation in ["", "-> Nil ! N | S(N)"] {
        accepts(
            &"S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(N) = (body: () -> Nil ! N) { body() }
forward(N) = (s ! S(N)) -> Nil ! N | S(N) { run(() { use() }) }"
                .replace("-> Nil ! N | S(N)", annotation),
            "forward",
            "() -> Nil ! N | S(N)",
            &["forward"],
        );
    }
}

#[test]
fn several_parameters_and_nested_services() {
    // Rules 4, 5 and 7 apply equally inside nested constructors and with
    // independent value/error/environment parameters. Rule 3 permits
    // independent closed instances of the resulting scheme.
    for annotation in ["", "-> A ? E ! N | Outer(A, E, N)"] {
        let source = "Inner(A, E, N) { Inner((A) -> A ? E ! N, (N) -> Nil) }
Outer(A, E, N) { Outer(Inner(A, E, N)) }
use(A, E, N) = (x: A, s ! Outer(A, E, N)) -> A ? E ! N | Outer(A, E, N) {
  Outer(Inner(f, _)) = s
  f(x)
}
run(A, E, N) = (body: () -> A ? E ! N) { body() }
integer = () {
  ! Outer(Int, Never, Never) = Outer(Inner((x: Int) { x }, (_: Never) { Nil }))
  run(() { use(1) })
}
boolean = () {
  ! Outer(Bool, Never, Never) = Outer(Inner((x: Bool) { x }, (_: Never) { Nil }))
  run(() { use(True) })
}"
        .replace("-> A ? E ! N | Outer(A, E, N)", annotation);
        accepts(&source, "integer", "() -> Int", &["integer", "boolean"]);
        accepts(&source, "boolean", "() -> Bool", &["integer", "boolean"]);
        accepts(
            &source,
            "use",
            "(x: A) -> A ^ E ! N | Outer(A, E, N)",
            &["integer", "boolean"],
        );
    }
}

#[test]
fn nonempty_stored_environment() {
    // §9.2: a callback takes its dependency at the call site; rule 9
    // removes each installed service, including a service nested in N.
    accepts(
        "Dependency { Dependency(value: Int) }
get = (d ! Dependency) { d.value }
Inner(N) { Inner(() -> Int ! N, (N) -> Nil) }
read(N) = (s ! Inner(N)) { Inner(f, _) = s f() }
Outer(N) { Outer(() -> Int ! N, (N) -> Nil) }
use(N) = (s ! Outer(N)) -> Int ! N | Outer(N) { Outer(f, _) = s f() }
run(A, N) = (body: () -> A ! N) { body() }
example = () {
  ! Dependency = Dependency(7)
  ! Inner(Dependency) = Inner(() { get() }, (_: Dependency) { Nil })
  ! Outer(Dependency | Inner(Dependency)) = Outer(() { read() }, (_: Dependency | Inner(Dependency)) { Nil })
  run(() { use() })
}",
        "example",
        "() -> Int",
        &["example"],
    );
}

#[test]
fn logging_example() {
    // Std-2 §7: E=String, N=Never. fail.catch reifies the error (§9.3),
    // and rule 9 removes Sink(String, Never).
    for annotation in ["", "-> Nil ? E ! N | Sink(E, N)"] {
        accepts(
            &"(i/fail)
Entry { Entry(message: String) }
Sink(E, N) { Sink((Entry) -> Nil ? E ! N, (N) -> Nil) }
make(E, N) = (send: (Entry) -> Nil ? E ! N) -> Sink(E, N) {
  Sink(send, (_: N) { Nil })
}
write(E, N) = (entry: Entry, sink ! Sink(E, N)) -> Nil ? E ! N | Sink(E, N) {
  Sink(send, _) = sink
  send(entry)
}
example = () {
  ! Sink(String, Never) = make((entry) { fail.raise(entry.message) })
  fail.catch(() { write(Entry(\"seen\")) })
}"
            .replace("-> Nil ? E ! N | Sink(E, N)", annotation),
            "example",
            "() -> Result(Nil, String)",
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
    // §4.7: "A type that has itself inside it, through a type variable,
    // is a compile error." These require an actual constructor cycle,
    // rather than the tautology N <: N | S(N) (§4.5 rule 2).
    for source in [
        "bad = (x) { x(x) }",
        "S(N) { S(() -> Nil ! N, (N) -> Nil) }
bad(N) = (s ! S(N)) -> Nil ! N { S(f, _) = s f() }",
        "S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
bad(N) = () {
  ! S(N) = S(() { use() }, (_: N) { Nil })
  use()
}",
    ] {
        rejects(source, "E0505");
    }
}

#[test]
fn incompatible_service_arguments_still_fail() {
    // Std-2 §7 / §4.5: the input marker keeps N invariant even when
    // Consumer's parameter is contravariant.
    rejects(
        "Entry { Entry }
Sink(E, N) { Sink((Entry) -> Nil ? E ! N, (N) -> Nil) }
Consumer(A) { Consumer((A) -> Nil) }
bad = (s: Sink(Never, Consumer(Int))) -> Sink(Never, Consumer(Never)) { s }",
        "E0515",
    );
    // §9.2: forms of one service in a row must have equal arguments,
    // including the error argument, regardless of its ordinary variance.
    rejects(
        "Sink(E, N) { Sink(() -> Nil ? E ! N, (N) -> Nil) }
one = (s ! Sink(String, Never)) { Nil }
two = (s ! Sink(Int, Never)) { Nil }
bad = () { one() two() }",
        "E0515",
    );
    // The same restriction holds when a service is hidden behind the
    // helper's inferred environment row.
    rejects(
        "Sink(E, N) { Sink(() -> Nil ? E ! N, (N) -> Nil) }
one = (s ! Sink(String, Never)) { Nil }
two = (s ! Sink(Int, Never)) { Nil }
run(N) = (body: () -> Nil ! N) { body() }
bad = () { run(() { one() }) two() }",
        "E0515",
    );
}

#[test]
fn repeated_helpers_keep_a_finite_principal_row() {
    // Repeated rule-3 instantiation and rule-5 forwarding add no service
    // and must preserve the same principal scheme (§4.7).
    let mut source = String::from(
        "S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(N) = (body: () -> Nil ! N) { body() }
forward_0 = () { run(() { use() }) }\n",
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
        "() -> Nil ! N | S(N)",
        &["forward_63"],
    );
}
