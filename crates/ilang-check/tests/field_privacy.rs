//! §2.4: exporting a nominal type without constructors permits using its
//! values, but not taking them apart. §6.7 applies this to field selection
//! and update; §6.4 limits numeric projection to tuples.

mod common;

const SECRET: &str = include_str!("../../../tests/spec/2.4/field_privacy_allowed/src/secret.i");
const FACADE: &str = include_str!("../../../tests/spec/2.4/field_privacy_allowed/src/facade.i");

#[test]
fn private_fields_cannot_be_selected_through_other_names_or_values() {
    // Each selection takes apart the same nominal value. Aliases and
    // forwarded values do not change its defining module (§2.4, §6.7).
    for source in [
        "(app/secret)\nf(A) = (v: secret.Secret(A)) { v.value }\n",
        "(app/secret)\nf(A) = (v: secret.Secret(A)) { v.missing }\n",
        "(app/secret)\nf(A) = (v: secret.Secret(A)) { v.parts.0 }\n",
        "(app/secret)\nf(A) = (v: secret.Secret(A)) { v.read() }\n",
        "(app/secret)\nf(A) = (v: secret.Secret(A)) { v.read }\n",
        "(app/secret)\nf = (v: secret.Choice) { v.value }\n",
        "(app/secret)\nf = (v: secret.Secret(Int) | secret.Open) { v.value }\n",
        "(app/secret)\nAlias(A) = secret.Secret(A)\nf(A) = (v: Alias(A)) { v.value }\n",
        "(app/secret)\n(Alias(..))\nAlias(A) = secret.Secret(A)\nf(A) = (v: Alias(A)) { v.value }\n",
        "((Secret) = app/secret)\nf(A) = (v: Secret(A)) { v.value }\n",
        "(renamed = app/secret)\nf(A) = (v: renamed.Secret(A)) { v.value }\n",
        "(app/facade)\nf(A) = (v: facade.Alias(A)) { v.value }\n",
        "(app/facade)\nx = facade.value.value\n",
        "(app/facade)\nx = facade.raw.read\n",
        "(app/secret)\nWrapper(A) { Wrapper(inner: secret.Secret(A)) }\nf(A) = (v: Wrapper(A)) { v.inner.value }\n",
        "(app/facade)\nx = facade.constructor(42).value\n",
        "(app/facade)\nf = (raw ! facade.Alias(Int)) { raw.value }\n",
        "(app/secret)\nf = (raw ! secret.Raw) { raw.read() }\n",
    ] {
        let (graph, _, diags) = common::check_files(&[
            ("src/secret.i", SECRET),
            ("src/facade.i", FACADE),
            ("src/main.i", source),
        ]);
        // Retaining the ordinary field type avoids cascading errors on
        // `.parts.0`. A missing label still fails §4.8 rule 13 too.
        let expected: &[&str] = if source.contains("v.missing") {
            &["E0204", "E0512"]
        } else {
            &["E0204"]
        };
        assert_eq!(
            diags.iter().map(|d| d.code).collect::<Vec<_>>(),
            expected,
            "{source}: {:?}",
            graph.sources.render_all(&diags)
        );
        let d = &diags[0];
        assert_eq!(d.code, "E0204", "{source}: {d:?}");
        assert!(d.message.contains("cannot select field"), "{d:?}");
        assert!(d.message.contains("app/secret"), "{d:?}");
        let text = &graph.sources.get(d.source.unwrap()).text;
        assert!(
            ["value", "parts", "read", "missing"]
                .contains(&&text[d.span.start as usize..d.span.end as usize]),
            "{d:?}"
        );
    }
}

#[test]
fn constructor_forms_and_numeric_projection_do_not_bypass_privacy() {
    for (source, code) in [
        (
            "(app/secret)\nx = secret.Pack(1, (1, 7), () { 1 })\n",
            "E0204",
        ),
        (
            "(app/secret)\nf = (v) { v |> { secret.Pack(value, ..) -> 0 } }\n",
            "E0204",
        ),
        (
            "(app/secret)\nf = (v) { secret.Pack(value, ..) = v  0 }\n",
            "E0204",
        ),
        (
            "(app/secret)\nx = secret.Pack(..secret.make(1), value: 2)\n",
            "E0204",
        ),
        (
            "((Secret) = app/secret)\nf(A) = (v: Secret(A)) { Pack(..v, value: 2) }\n",
            "E0201",
        ),
        (
            "(app/secret)\nf(A) = (v: secret.Secret(A)) { v.0 }\n",
            "E0512",
        ),
        (
            "(app/secret)\nf(A) = (v: secret.Secret(A)) { (a, b, c) = v  a }\n",
            "E0501",
        ),
    ] {
        let (graph, _, diags) =
            common::check_files(&[("src/secret.i", SECRET), ("src/main.i", source)]);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(
            !errors.is_empty() && errors.iter().all(|d| d.code == code),
            "{source}: {:?}",
            graph.sources.render_all(&diags)
        );
    }
}

#[test]
fn generic_value_use_and_public_constructor_fields_keep_their_types() {
    let (graph, _, diags) = common::check_files(&[
        ("src/secret.i", SECRET),
        ("src/facade.i", FACADE),
        (
            "src/main.i",
            include_str!("../../../tests/spec/2.4/field_privacy_allowed/src/main.i"),
        ),
    ]);
    assert!(diags.is_empty(), "{:?}", graph.sources.render_all(&diags));
    let checked = ilang_check::check(&graph);
    let program = checked.program.unwrap();
    let types = checked.types.unwrap();
    let lines = types.lines(&program, common::module(&program, "app/main"));
    // §4.7 requires principal types; §4.8 rules 3/4 give identity its
    // general type. §2.4 permits using Secret, and §6.7 gives public
    // Open.value the field type Int, also through its alias (§4.3).
    assert_eq!(
        lines,
        [
            "app/main.id : (value: A) -> A",
            "app/main.pass : (value: secret.Secret(A)) -> secret.Secret(A)",
            "app/main.public : (value: secret.Open) -> Int",
            "app/main.main : () -> Nil ! Assert",
        ]
    );
}
