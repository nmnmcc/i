use ilang_syntax::{format::format, parse_module, print::print_module, shape::canon};

#[test]
fn pipe_and_match_keep_their_spelling() {
    let source = "answer = x |> { 0 -> 1 _ -> 2 }";
    let (tree, diagnostics) = parse_module(source);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    for printed in [format(source).unwrap(), print_module(&tree)] {
        assert!(printed.contains("|>"), "{printed}");
        assert_eq!(canon(&parse_module(&printed).0), canon(&tree));
    }
    assert!(
        !parse_module(/* old-syntax-check: E0101 */ "answer = x.{ _ -> 1 }")
            .1
            .is_empty()
    );
    assert!(
        !parse_module(/* old-syntax-check: E0101 */ "answer = x | f")
            .1
            .is_empty()
    );
}

#[test]
fn records_tuples_and_guard_are_distinct() {
    for source in [
        "answer = ()",
        // syntax-3 stage 6: `()` is the empty data type.
        "T = ()",
        "answer = (name: 1)",
        "answer = (1, 2)",
        "answer = (x) -> Int ^ String { x }",
        "answer = x |> { f: (Int) -> Int ? ready -> f(1) _ -> 0 }",
    ] {
        let (_, diagnostics) = parse_module(source);
        assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
    }
    for source in [
        /* old-syntax-check: E0101 */ "answer = (1, name: 2)",
        /* old-syntax-check: E0101 */ "answer = (name: 1, 2)",
    ] {
        assert!(!parse_module(source).1.is_empty(), "{source}");
    }
}
