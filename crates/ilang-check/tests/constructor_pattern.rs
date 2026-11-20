//! A constructor is a function (`True` is `() -> True`), so a pattern
//! matches its value only through the call `True()`; the bracketless
//! `(True)` is reported.
#[path = "common/mod.rs"]
mod common;

use ilang_check::code;
use ilang_package::Diagnostic;

const BARE: &str =
    "r = (x : Bool) -> Int { x |> (True) { 1 } & (False) { 0 } }\nmain = () { r(True) }\n";
const CALLED: &str =
    "r = (x : Bool) -> Int { x |> (True()) { 1 } & (False()) { 0 } }\nmain = () { r(True) }\n";

#[test]
fn bare_constructor_pattern_reports() {
    let (graph, _, diags) = common::check_files(&[("src/main.i", BARE)]);
    let found: Vec<&Diagnostic> = diags
        .iter()
        .filter(|d| d.code == code::CONSTRUCTOR_AS_PATTERN)
        .collect();
    assert_eq!(found.len(), 2, "{:?}", graph.sources.render_all(&diags));
    for (d, name) in found.iter().zip(["True", "False"]) {
        assert_eq!(
            d.message,
            format!("`{name}` is a constructor function; write `{name}()` to match its value")
        );
        assert!(d.is_error());
        let start = (BARE.find(&format!("({name})")).unwrap() + 1) as u32;
        assert_eq!(
            (d.span.start, d.span.end),
            (start, start + name.len() as u32)
        );
    }
}

#[test]
fn called_constructor_pattern_passes() {
    let (graph, _, diags) = common::check_files(&[("src/main.i", CALLED)]);
    assert!(
        !diags.iter().any(Diagnostic::is_error),
        "{:?}",
        graph.sources.render_all(&diags)
    );
}
