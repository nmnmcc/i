//! STR surface spelling, desugaring and formatter round trips.
use ilang_syntax::{
    format::format, lex, lisp, parse_module, print::print_module, shape, token::TokenKind,
};

#[test]
fn interpolation_is_concat_before_the_core_tree() {
    for (source, core) in [
        (r#"value = "a\(x)b\(y)""#, r#"value = "a" <> x <> "b" <> y"#),
        (r#"value = "\(x)""#, r#"value = "" <> x"#),
        (r#"value = "\(x)\(y)""#, r#"value = x <> y"#),
        (r##"value = #"a\#(x)b"#"##, r#"value = "a" <> x <> "b""#),
    ] {
        let (a, ds) = parse_module(source);
        assert!(ds.is_empty(), "{source}: {ds:?}");
        let (b, ds) = parse_module(core);
        assert!(ds.is_empty());
        assert!(shape::equivalent(&a, &b), "{source}\n{}", print_module(&a));
        let printed = lisp::print_module(&a);
        let (back, ds) = lisp::parse_source(&printed);
        assert!(ds.is_empty(), "{printed}: {ds:?}");
        assert!(shape::equivalent(&a, &back));
    }
}

#[test]
fn raw_and_multiline_values() {
    for (source, expected) in [
        (r##"#"raw \(name) \n"#"##, r"raw \(name) \n"),
        (r###"##"\#n\##n"##"###, "\\#n\n"),
        ("\"\"\"\n  one\n    two\n  \"\"\"", "one\n  two"),
        ("\"\"\"\r\n\tα\\\r\n\tβ\r\n\t\"\"\"", "αβ"),
        ("\"\"\"\n  a\n \n  b\n  \"\"\"", "a\n\nb"),
        ("#\"\"\"\n  \\n\n  \"\"\"#", "\\n"),
        ("\"\"\"\n\"\"\"", ""),
        ("\"\"\"\n  \\n\n  \"\"\"", "\n"),
    ] {
        let result = lex(source);
        assert!(
            result.diagnostics.is_empty(),
            "{source}: {:?}",
            result.diagnostics
        );
        assert_eq!(
            result.tokens[0].kind,
            TokenKind::Str(expected.into()),
            "{source}"
        );
    }
}

#[test]
fn formatter_preserves_interpolation_with_nested_expressions() {
    for source in [
        r#"value="a\(f("b\(x)"))c""#,
        r##"value=#"a\#(x)b"# <> "c\(y)""##,
        r#"value="\(x)" <> "b""#,
        "value = \"\"\"\n  a \\(f(\"x\"))\n  b\n  \"\"\"",
        r#"value = "a\({ // inner comment
x })b" // outside"#,
        r#"value = (s) { s |> { "a" <> r <> "b" -> r  r <> "x" -> r  _ -> "" } }"#,
    ] {
        let (a, ds) = parse_module(source);
        assert!(ds.is_empty(), "{source}: {ds:?}");
        let out = format(source).unwrap();
        let (b, ds) = parse_module(&out);
        assert!(ds.is_empty(), "{out}: {ds:?}");
        assert!(shape::equivalent(&a, &b), "{source}\n{out}");
        assert_eq!(out, format(&out).unwrap());
        if source.contains("\\(") || source.contains("\\#(") {
            assert!(out.contains("\\(") || out.contains("\\#("), "{out}");
        }
        let printed = print_module(&a);
        let (back, ds) = parse_module(&printed);
        assert!(ds.is_empty(), "{printed}: {ds:?}");
        assert!(shape::equivalent(&a, &back));
        let printed = lisp::print_module(&a);
        let (back, ds) = lisp::parse_source(&printed);
        assert!(ds.is_empty(), "{printed}: {ds:?}");
        assert!(shape::equivalent(&a, &back), "{printed}");
    }
}

#[test]
fn interpolation_and_multiline_boundaries_are_diagnosed() {
    for source in [
        r#"value = ("\(x)": 1)"#,
        r#"Value = ("\(x)": Int)"#,
        r#"Value = "\(x)""#,
        r#"value = (s) { s |> { "\(x)" -> 1 } }"#,
        r#"@deprecated("\(x)")
value = 1"#,
        r#"("\(x)")
value = 1"#,
        r#"value = (s) { s |> { a <> "x" <> b -> 1 } }"#,
        "value = \"\"\"bad\n\"\"\"",
        "value = \"\"\"\na\n  \"\"\"",
        "value = \"\"\"\n  a\"\"\"",
        r##"value = #"bad"##,
    ] {
        let (_, ds) = parse_module(source);
        assert!(!ds.is_empty(), "accepted: {source}");
        if source.contains("\\(x)") {
            assert!(
                ds.iter().any(|d| d.message.contains("interpolation")),
                "{source}: {ds:?}"
            );
        }
    }
    let (_, ds) = lisp::parse_source(r#"(= value "\(x)")"#);
    assert!(
        ds.iter()
            .any(|d| d.message.contains("no string interpolation"))
    );
}
