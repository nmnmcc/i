use ilang_syntax::lisp::{DatumKind, normal_source, parse, print_module, show};

#[test]
fn datums_round_trip_and_keep_spans() {
    let source =
        "// a comment\n(= fact (-> (n) _ { (* n 0x10) }))\n`(f ~x ~..(map xs (-> (y) _ `(g ~y))))";
    let datums = parse(source).unwrap();
    assert_eq!(datums.len(), 2);
    assert_eq!(
        &source[datums[0].span.start as usize..datums[0].span.end as usize],
        "(= fact (-> (n) _ { (* n 0x10) }))"
    );
    for datum in datums {
        let printed = show(&datum);
        let parsed = parse(&printed).unwrap();
        assert_eq!(show(&parsed[0]), printed);
    }
}

#[test]
fn quote_rules_are_checked() {
    assert!(parse("~x").is_err());
    assert!(parse("`(f `(g x))").is_err());
    assert!(parse("`(f ~`(g ~x))").is_ok());
    assert!(parse("`").is_err());
}

#[test]
fn invalid_datums_do_not_make_core_items() {
    for source in [
        "(@ (m) (= x 1))",
        "(@ m (other 1))",
        "(= x (1 .. 2))",
        "(= x [.. 1])",
    ] {
        let document = parse(source).unwrap();
        assert!(
            ilang_syntax::lisp::parse_module(&document).is_err(),
            "{source}"
        );
    }
}

#[test]
fn direct_decode_accepts_expression_function_bodies() {
    let source = "(= plus_one (-> (x) _ (+ x 1)))";
    let document = parse(source).unwrap();
    let module = ilang_syntax::lisp::parse_module(&document).unwrap();
    assert_eq!(module.items[0].span.start, 0);
    let normal = normal_source(&document).unwrap();
    let (again, errors) = ilang_syntax::parse_module(&normal);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        ilang_syntax::shape::canon(&module),
        ilang_syntax::shape::canon(&again)
    );
}

#[test]
fn written_return_hole_is_not_inferred_return() {
    let (module, errors) = ilang_syntax::parse_module("f = (x) -> _ { x }\n");
    assert!(errors.is_empty(), "{errors:?}");
    let lisp = print_module(&module);
    assert!(lisp.contains("(^ _ Never)"), "{lisp}");
    let back = ilang_syntax::lisp::parse_module(&parse(&lisp).unwrap()).unwrap();
    assert_eq!(
        ilang_syntax::shape::canon(&module),
        ilang_syntax::shape::canon(&back)
    );
}

#[test]
fn conversion_preserves_comments_before_items_and_statements() {
    let source = "// item\nf = () {\n  // statement\n  x = 1\n  x // trailing\n}\n";
    let (module, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let (lisp, lost) = ilang_syntax::lisp::print_with_comments(source, &module);
    assert!(lisp.contains("// item\n(= f"), "{lisp}");
    assert!(lisp.contains("// statement\n(= x 1)"), "{lisp}");
    assert_eq!(lost.len(), 1);
    assert_eq!(
        &source[lost[0].start as usize..lost[0].end as usize],
        "// trailing"
    );
    let back = ilang_syntax::lisp::parse_module(&parse(&lisp).unwrap()).unwrap();
    assert_eq!(
        ilang_syntax::shape::canon(&module),
        ilang_syntax::shape::canon(&back)
    );
    let doc = parse(&lisp).unwrap();
    let (normal, lost) = ilang_syntax::lisp::normal_with_comments(&lisp, &doc).unwrap();
    assert!(lost.is_empty(), "{lost:?}");
    assert!(normal.contains("// item\nf ="), "{normal}");
    assert!(normal.contains("// statement\nx = 1"), "{normal}");
}

#[test]
fn lisp_formatter_keeps_all_comments_and_is_idempotent() {
    let source =
        "// before\n(= f (-> () _ { (= x 1) // trailing\n (+ x // inside\n 1) }))\n// end\n";
    let formatted = ilang_syntax::lisp::format(source).unwrap();
    for comment in ["// before", "// trailing", "// inside", "// end"] {
        assert!(formatted.contains(comment), "{formatted}");
    }
    assert_eq!(ilang_syntax::lisp::format(&formatted).unwrap(), formatted);
}

#[test]
fn dots_and_spreads_are_separate_datums() {
    let xs = parse("(. a 12) [1 .. x] <<1 (: 2 u8)>>").unwrap();
    let DatumKind::Round(parts) = &xs[0].kind else {
        panic!()
    };
    assert!(matches!(&parts[2].kind, DatumKind::Int(n) if n.to_string() == "12"));
    assert_eq!(show(&xs[1]), "[1 .. x]");
    assert_eq!(show(&xs[2]), "<< 1 (: 2 u8) >>");
}

#[test]
fn core_forms_convert_between_surfaces() {
    let source = r#"
(x, Option(..))
Option(A) { Some(A) None }
value: (Int) -> Int
value = (x) -> Int { x + 1 }
choose = (x) { x |> { 0 -> "zero" _ -> "other" } }
"#;
    let (module, diagnostics) = ilang_syntax::parse_module(source);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let lisp = print_module(&module);
    let datums = parse(&lisp).unwrap();
    let normal = normal_source(&datums).unwrap();
    let (again, diagnostics) = ilang_syntax::parse_module(&normal);
    assert!(diagnostics.is_empty(), "{diagnostics:?}\n{normal}\n{lisp}");
    assert_eq!(
        ilang_syntax::print::print_module(&module),
        ilang_syntax::print::print_module(&again)
    );
}

#[test]
fn format_corpus_has_a_lisp_round_trip() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/format");
    check_corpus(&dir);
}

#[test]
fn libraries_have_a_lisp_round_trip() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    check_corpus(&root.join("lib"));
    check_corpus(&root.join("std"));
}

#[test]
fn spec_programs_have_a_lisp_round_trip() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    check_corpus(&root.join("tests/spec"));
}

fn check_corpus(dir: &std::path::Path) {
    let mut failures = Vec::new();
    let mut dirs = vec![dir.to_owned()];
    while let Some(dir) = dirs.pop() {
        for file in std::fs::read_dir(dir).unwrap() {
            let path = file.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "i") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            if source
                .lines()
                .any(|line| line.trim_start().starts_with("// expect: compile-error"))
            {
                continue;
            }
            let (module, diagnostics) = ilang_syntax::parse_module(&source);
            if !diagnostics.is_empty() {
                continue;
            }
            let lisp = print_module(&module);
            match parse(&lisp)
                .and_then(|d| ilang_syntax::lisp::parse_module(&d).map_err(|e| vec![e]))
            {
                Ok(back)
                    if ilang_syntax::shape::canon(&module) == ilang_syntax::shape::canon(&back) => {
                }
                Ok(back) => failures.push(format!(
                    "{}: direct tree differs\n{lisp}\nBACK:\n{}",
                    path.display(),
                    print_module(&back)
                )),
                Err(e) => failures.push(format!("{}: direct decode {e:?}\n{lisp}", path.display())),
            }
            let converted = parse(&lisp)
                .map_err(|e| format!("datum: {e:?}"))
                .and_then(|ds| normal_source(&ds).map_err(|e| format!("conversion: {e:?}")));
            match converted {
                Err(e) => failures.push(format!("{}: {e}\n{lisp}", path.display())),
                Ok(normal) => {
                    let (back, errors) = ilang_syntax::parse_module(&normal);
                    if !errors.is_empty()
                        || ilang_syntax::shape::canon(&module) != ilang_syntax::shape::canon(&back)
                    {
                        failures.push(format!(
                            "{}: {errors:?}\n{lisp}\n{normal}\nBACK:\n{}",
                            path.display(),
                            print_module(&back)
                        ));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
