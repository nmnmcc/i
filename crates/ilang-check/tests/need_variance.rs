//! Need entries are input places.
mod common;

fn errors(source: &str) -> Vec<String> {
    let (graph, _, diags) = common::check_files(&[("src/main.i", source)]);
    diags
        .iter()
        .filter(|d| d.is_error())
        .map(|d| graph.sources.render(d))
        .collect()
}

#[test]
fn reader_need_cannot_be_widened_through_a_labelled_type() {
    let source = include_str!("../../../tests/spec/4.5/need_entry_input.i");
    let errors = errors(source);
    assert!(errors.iter().any(|e| e.contains("E0501")), "{errors:?}");
}

#[test]
fn continuation_need_cannot_be_widened_through_a_labelled_type() {
    let source = include_str!("../../../tests/spec/4.5/continuation_need_input.i");
    let errors = errors(source);
    assert!(errors.iter().any(|e| e.contains("E0501")), "{errors:?}");
}

#[test]
fn reader_need_cannot_be_widened_through_a_row_parameter() {
    let source = include_str!("../../../tests/spec/4.5/nested_need_entry_input.i");
    let errors = errors(source);
    assert!(errors.iter().any(|e| e.contains("E0501")), "{errors:?}");
}

#[test]
fn event_need_cannot_be_widened_through_a_labelled_type() {
    let source = include_str!("../../../tests/spec/4.5/event_need_entry_input.i");
    let errors = errors(source);
    assert!(errors.iter().any(|e| e.contains("E0501")), "{errors:?}");
}

#[test]
fn reader_need_narrows_and_sink_need_widens() {
    let source = include_str!("../../../tests/spec/4.5/need_entry_direction.i");
    assert!(errors(source).is_empty(), "{:?}", errors(source));
}
