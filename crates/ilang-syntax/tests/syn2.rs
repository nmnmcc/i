use ilang_syntax::{format::format, parse_module, print::print_module, shape::canon};

#[test]
fn old_and_new_records_have_the_same_tree() {
    for (old, new) in [
        ("answer = { name: 1 }", "answer = (name: 1)"),
        ("T = { name: Int }", "T = (name: Int)"),
        ("T = {..}", "T = Record"),
        (
            "answer = (x) -> Int ? String { x }",
            "answer = (x) -> Int ^ String { x }",
        ),
    ] {
        let (a, da) = parse_module(old);
        let (b, db) = parse_module(new);
        assert!(da.is_empty(), "{old}: {da:?}");
        assert!(db.is_empty(), "{new}: {db:?}");
        assert_eq!(canon(&a), canon(&b));
        let printed = print_module(&a);
        assert_eq!(canon(&parse_module(&printed).0), canon(&a));
        let formatted = format(old).unwrap();
        let (roundtrip, errors) = parse_module(&formatted);
        assert!(errors.is_empty(), "{formatted}: {errors:?}");
        assert_eq!(canon(&roundtrip), canon(&a));
    }
}

#[test]
fn pipe_and_match_keep_their_spelling() {
    let source = "answer = x |> { 0 -> 1 _ -> 2 }";
    let (tree, diagnostics) = parse_module(source);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    for printed in [format(source).unwrap(), print_module(&tree)] {
        assert!(printed.contains("|>"), "{printed}");
        assert_eq!(canon(&parse_module(&printed).0), canon(&tree));
    }
    assert!(!parse_module(/* old-syntax-check: E0101 */ "answer = x.{ _ -> 1 }").1.is_empty());
    assert!(!parse_module(/* old-syntax-check: E0101 */ "answer = x | f").1.is_empty());
}

#[test]
fn records_tuples_and_guard_are_distinct() {
    for source in [
        "answer = ()",
        "answer = (name: 1)",
        "answer = (1, 2)",
        "answer = (x) -> Int ^ String { x }",
        "answer = x |> { f: (Int) -> Int ? ready -> f(1) _ -> 0 }",
    ] {
        let (_, diagnostics) = parse_module(source);
        assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
    }
    for source in [/* old-syntax-check: E0101 */ "T = ()", /* old-syntax-check: E0101 */ "answer = (1, name: 2)", /* old-syntax-check: E0101 */ "answer = (name: 1, 2)"] {
        assert!(!parse_module(source).1.is_empty(), "{source}");
    }
}
