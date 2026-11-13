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
        let printed =
            ilang_syntax::syn3::lisp::to_lisp(&ilang_syntax::syn3::parse_module(source).0);
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
        (r##"#"""#"##, "\""),
        (r###"##""""##"###, "\"\""),
        ("\"\"\"\n  one\n    two\n  \"\"\"", "one\n  two"),
        ("\"\"\"\r\n\tα\\\r\n\tβ\r\n\t\"\"\"", "αβ"),
        ("\"\"\"\n  a\n \n  b\n  \"\"\"", "a\n\nb"),
        ("\"\"\"\n  a\n    \n  b\n  \"\"\"", "a\n  \nb"),
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
        r#"value = (s) { s |> ("a" <> r <> "b") { r } | (r <> "x") { r } | (_) { "" } }"#,
        r#"value = (s) {
	s |>
		("a" <> r <> "b") {
			// prefix
			// suffix
			r
		}
		| (_) { "" }
}"#,
        r#"value = (s) {
	s |>
		(r <> "b") {
			// suffix
			r
		}
		| (_) { "" }
}"#,
    ] {
        let (a, ds) = parse_module(source);
        assert!(ds.is_empty(), "{source}: {ds:?}");
        let out = format(source).unwrap();
        let (b, ds) = parse_module(&out);
        assert!(ds.is_empty(), "{out}: {ds:?}");
        assert!(shape::equivalent(&a, &b), "{source}\n{out}");
        assert_eq!(out, format(&out).unwrap());
        assert_eq!(
            lex(source).comments.len(),
            lex(&out).comments.len(),
            "{out}"
        );
        if source.contains("\\(") || source.contains("\\#(") {
            assert!(out.contains("\\(") || out.contains("\\#("), "{out}");
        }
        let tree = ilang_syntax::syn3::parse_module(source).0;
        let text = ilang_syntax::syn3::lisp::to_lisp(&tree);
        let (read, ds) = ilang_syntax::syn3::lisp::from_lisp(&text);
        assert!(ds.is_empty(), "{text}: {ds:?}");
        assert_eq!(ilang_syntax::syn3::lisp::to_lisp(&read), text);
        // The datum built from the tree (a macro's argument) is the datum of
        // the Lisp text.
        let scope = lisp::syntax::Scope {
            module: 0,
            marks: Default::default(),
        };
        let built = ilang_syntax::syn3::lisp::item_datum(&tree.items[0]);
        assert_eq!(
            lisp::syntax::Syntax::from_datum(&built, &scope).show(),
            lisp::show(&lisp::parse(&text).unwrap()[0])
        );
    }
}

#[test]
fn interpolation_and_multiline_boundaries_are_diagnosed() {
    for source in [
        /* old-syntax-check: E0101 */ r#"value = (s) { s |> ("\(x)") { 1 } }"#,
        /* old-syntax-check: E0101 */ r#"value = (s) { s |> (a <> "x" <> b) { 1 } }"#,
        /* old-syntax-check: E0003 */ "value = \"\"\"bad\n\"\"\"",
        /* old-syntax-check: E0003 */ "value = \"\"\"\na\n  \"\"\"",
        /* old-syntax-check: E0003 */ "value = \"\"\"\n  a\"\"\"",
        /* old-syntax-check: E0002 */ r##"value = #"bad"##,
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
