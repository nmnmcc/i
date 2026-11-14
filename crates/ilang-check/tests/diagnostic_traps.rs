//! Diagnostic-only explanations of the readings required by the spec.
mod common;

fn diagnostics(source: &str) -> Vec<ilang_package::Diagnostic> {
    common::check_files(&[("src/main.i", source)]).2
}

#[test]
fn two_spaced_constructor_parts_are_still_one_binding() {
    let source = "Pair = (Int, Int)\nf = (g: Pair) { mk = Pair  (b, c) = g\n (mk, b, c) }\n_ = f\n";
    let ds = diagnostics(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].code, "W0506");
    assert!(!ds[0].is_error());
    assert_eq!(
        ds[0].message,
        "this is one binding, not two; to write two, put the second on its own line (§1.7)"
    );
    for source in [
        source.replace("Pair  (", "Pair("),
        source.replace("Pair  (", "Pair ("),
        "Pair = (Int, Int)\nf = (g: Pair) { mk = Pair\n (b, c) = (1, 2)\n (mk, b, c, g) }\n_ = f\n"
            .into(),
        "Pair = (Int, Int)\nf = () { Pair  (1, 2) }\n_ = f\n".into(),
    ] {
        assert!(
            diagnostics(&source).is_empty(),
            "{source}: {:?}",
            diagnostics(&source)
        );
    }
}

#[test]
fn spaced_function_call_keeps_its_error_and_explains_the_line() {
    // old-syntax-check: E0101
    let source = "f = (x, y) { x + y }\ng = () { a = f  (b, c) = (1, 2)\n a }\n_ = g\n";
    let ds = diagnostics(source);
    assert!(
        ds.iter().any(|d| d.code == "E0101"
            && d.message.ends_with(
                "this is one binding, not two; to write two, put the second on its own line (§1.7)"
            )),
        "{ds:?}"
    );
    let ds = diagnostics(&source.replace("f  (", "f("));
    assert!(ds.iter().any(|d| d.code == "E0101"), "{ds:?}");
    assert!(
        ds.iter()
            .all(|d| !d.message.contains("this is one binding")),
        "{ds:?}"
    );
}

#[test]
fn missing_known_parameter_type_explains_both_fixes() {
    let source = "User = (name = String)\nget_name = (u) { u.name }\n_ = get_name\n";
    let ds = diagnostics(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].code, "E0506");
    assert!(ds[0].is_error());
    assert_eq!(
        ds[0].message,
        "`.name` needs a known type for its value (§6.7, §4.8); write the parameter's type, for example `(u: User) { u.name }`, or give the function where a type is expected, for example `get_name: (User) -> String = (u) { u.name }`. An inferred type is solved from constraints; a known type comes from written types and earlier known facts, including the expected type, before solving (§4.8)"
    );
    for source in [
        source.replace("(u) {", "(u: User) {"),
        source.replace("get_name =", "get_name: (User) -> String ="),
        "User = (name = String)\napply = (f: (User) -> String) { f(User(name = \"Ada\")) }\n_ = apply((u) { u.name })\n".into(),
    ] {
        assert!(diagnostics(&source).is_empty(), "{source}: {:?}", diagnostics(&source));
    }
}
