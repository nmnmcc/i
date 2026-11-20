//! A constructor used as a value is a function, also one without fields;
//! a call of it makes the value.
#[path = "common/mod.rs"]
mod common;

fn accepted(source: &str) {
    let (_graph, program, diags) = common::check_files(&[("src/main.i", source)]);
    let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert!(program.is_some());
}

#[test]
fn zero_payload_constructor_as_a_function_value() {
    accepted(
        "apply = (f : () -> Bool, v : Bool) -> Bool { f() }\n\
         main = () { apply(True, True()) }\n",
    );
}

#[test]
fn zero_payload_constructor_through_a_list() {
    accepted(
        "keep = (xs : List(Int), f : (Int) -> Bool) -> List(Int) {\n\
         \txs |> ([]) { [] } & ([x, ..rest]) { f(x) |> (True()) { [x, ..keep(rest, f)] } & (False()) { keep(rest, f) } }\n\
         }\n\
         main = () { keep([1, 2], (_) { True() }) }\n",
    );
}
