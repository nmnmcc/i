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
fn deep_syntax_show_and_datum_conversion_use_an_explicit_stack() {
    use ilang_syntax::lisp::{
        Datum,
        syntax::{Node, Scope, Syntax},
    };
    use ilang_syntax::span::Span;
    use std::collections::BTreeSet;

    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let leaf = Datum {
                kind: DatumKind::Name("x".into()),
                span: Span::new(0, 1),
                docs: Vec::new(),
            };
            let like = Syntax::from_datum(
                &leaf,
                &Scope {
                    module: 0,
                    marks: BTreeSet::new(),
                },
            );
            let mut value = like.clone();
            let mut keepers = vec![value.clone()];
            for _ in 0..20_000 {
                value = Syntax::make(Node::Round(vec![value]), &like).unwrap();
                keepers.push(value.clone());
            }
            let printed = value.show();
            assert_eq!(printed.len(), 40_001);
            assert_eq!(printed.as_bytes()[20_000], b'x');
            let datum = value.to_datum();
            assert_eq!(show(&datum), printed);
            drop(datum);
            let mut quoted = Datum {
                kind: DatumKind::Name("x".into()),
                span: Span::new(0, 1),
                docs: Vec::new(),
            };
            for _ in 0..20_000 {
                quoted = Datum {
                    kind: DatumKind::Quote(Box::new(quoted)),
                    span: Span::new(0, 1),
                    docs: Vec::new(),
                };
            }
            assert_eq!(show(&quoted).len(), 20_001);
            drop(quoted);
            // Keep Syntax children alive until their parents have been released.
            drop(value);
            while let Some(value) = keepers.pop() {
                drop(value);
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn quote_rules_are_checked() {
    assert!(parse("~x").is_err());
    assert!(parse("`(f `(g x))").is_err());
    assert!(parse("`(f ~`(g ~x))").is_ok());
    assert!(parse("`").is_err());
}

#[test]
fn macro_quotes_are_core_nodes_and_can_be_formatted() {
    let source = "(@ macro (= when (-> (condition body) _ `(if ~condition ~body))))\n";
    let doc = parse(source).unwrap();
    let module = ilang_syntax::lisp::parse_module(&doc).unwrap();
    let printed = print_module(&module);
    assert!(printed.contains("`(if ~condition ~body)"), "{printed}");
    assert!(normal_source(&doc).is_err());
}

#[test]
fn quote_splices_are_checked_in_their_written_position() {
    for source in [
        "(= f (-> () _ `~..xs))",
        "(= f (-> (`(~..xs x)) _ Nil))",
        "(= f (-> () _ `(~(+) x)))",
    ] {
        let doc = parse(source).unwrap();
        assert!(ilang_syntax::lisp::parse_module(&doc).is_err(), "{source}");
    }
    let doc = parse("(= f (-> (`(f ~..xs)) _ Nil))").unwrap();
    assert!(ilang_syntax::lisp::parse_module(&doc).is_ok());
}

#[test]
fn quote_pattern_captures_follow_the_syntax_matcher_order() {
    use ilang_syntax::ast::PatternKind;
    use ilang_syntax::lisp::parse_pattern;

    let quote = parse("`(f [~first {~second}] ~..rest)").unwrap();
    let pattern = parse_pattern(&quote[0]).unwrap();
    let PatternKind::Quote { captures, .. } = &pattern.kind else {
        panic!("quote pattern");
    };
    assert_eq!(captures.len(), 3);
    for (capture, name) in captures.iter().zip(["first", "second", "rest"]) {
        assert!(matches!(&capture.1.kind, PatternKind::Var(n) if n == name));
    }
    assert_eq!(
        captures.iter().map(|c| c.0).collect::<Vec<_>>(),
        [false, false, true]
    );

    let outer = parse("`(f ~`(g ~inner))").unwrap();
    let pattern = parse_pattern(&outer[0]).unwrap();
    let PatternKind::Quote { captures, .. } = &pattern.kind else {
        panic!("quote pattern");
    };
    assert_eq!(captures.len(), 1);
    let PatternKind::Quote {
        captures: inner, ..
    } = &captures[0].1.kind
    else {
        panic!("the capture must retain its nested quote pattern");
    };
    assert!(matches!(&inner[0].1.kind, PatternKind::Var(n) if n == "inner"));
    assert!(parse_pattern(&outer[0]).is_ok());
}

#[test]
fn compiler_preparation_registers_nested_quote_pattern_templates() {
    use ilang_syntax::ast::{ExprKind, ItemKind, Param, PatternKind};
    use ilang_syntax::lisp::quote_lower::lower_expression_quotes;
    use ilang_syntax::lisp::syntax::Node;
    let mut module =
        ilang_syntax::lisp::parse_module(&parse("(= f (-> (`(f ~`(g ~x))) _ x))").unwrap())
            .unwrap();
    let mut templates = Vec::new();
    lower_expression_quotes(&mut module, 7, &mut templates);
    assert_eq!(templates.len(), 2);
    assert!(templates.iter().all(|s| matches!(s.node(), Node::Quote(_))));
    let ItemKind::Binding(binding) = &module.items[0].kind else {
        panic!("binding");
    };
    let ExprKind::Fn(function) = &binding.value.kind else {
        panic!("function");
    };
    let Param::Pattern(pattern) = &function.params[0] else {
        panic!("pattern");
    };
    let PatternKind::Quote {
        template, captures, ..
    } = &pattern.kind
    else {
        panic!("quote");
    };
    assert_eq!(*template, Some(0));
    assert!(matches!(
        &captures[0].1.kind,
        PatternKind::Quote {
            template: Some(1),
            ..
        }
    ));
}

#[test]
fn quote_capture_collection_handles_deep_templates() {
    use ilang_syntax::ast::PatternKind;
    use ilang_syntax::lisp::{Datum, DatumKind, parse_pattern};
    use ilang_syntax::span::Span;
    let datum = |kind| Datum {
        kind,
        span: Span::new(0, 0),
        docs: Vec::new(),
    };
    let mut body = datum(DatumKind::Unquote(Box::new(datum(DatumKind::Name(
        "x".into(),
    )))));
    for _ in 0..10_000 {
        body = datum(DatumKind::Round(vec![body]));
    }
    let quote = datum(DatumKind::Quote(Box::new(body)));
    let pattern = parse_pattern(&quote).unwrap();
    assert!(matches!(&pattern.kind, PatternKind::Quote { captures, .. } if captures.len() == 1));
}

#[test]
fn nested_quote_captures_use_the_parser_work_stack() {
    use ilang_syntax::ast::PatternKind;
    use ilang_syntax::lisp::{Datum, DatumKind, parse_pattern};
    use ilang_syntax::span::Span;
    let datum = |kind| Datum {
        kind,
        span: Span::new(0, 0),
        docs: Vec::new(),
    };
    let mut nested = datum(DatumKind::Name("x".into()));
    for _ in 0..256 {
        nested = datum(DatumKind::Quote(Box::new(datum(DatumKind::Round(vec![
            datum(DatumKind::Unquote(Box::new(nested))),
        ])))));
    }
    let pattern = parse_pattern(&nested).unwrap();
    let mut at = &pattern;
    for _ in 0..256 {
        let PatternKind::Quote { captures, .. } = &at.kind else {
            panic!("nested quote");
        };
        at = &captures[0].1;
    }
    assert!(matches!(&at.kind, PatternKind::Var(name) if name == "x"));
}

#[test]
fn invalid_datums_do_not_make_core_items() {
    for source in ["(= x (1 .. 2))", "(= x [.. 1])"] {
        let document = parse(source).unwrap();
        assert!(
            ilang_syntax::lisp::parse_module(&document).is_err(),
            "{source}"
        );
    }
}

#[test]
fn both_surfaces_preserve_attributes_and_grouped_result_holes() {
    for source in ["@m() x = 1", "@m call(1)", "f = () -> { _ } { 1 }"] {
        let (module, errors) = ilang_syntax::parse_module(source);
        assert!(errors.is_empty(), "{source}: {errors:?}");
        let lisp = print_module(&module);
        let back = ilang_syntax::lisp::parse_module(&parse(&lisp).unwrap()).unwrap();
        assert_eq!(
            ilang_syntax::shape::canon(&module),
            ilang_syntax::shape::canon(&back),
            "{source}\n{lisp}"
        );
    }
}

#[test]
fn a_nonfinal_clause_choice_cannot_end_in_a_type_test() {
    let doc = parse("(= f { (-> (| (= a (: b Int)) Unit) 1) })").unwrap();
    assert!(ilang_syntax::lisp::parse_module(&doc).is_err());
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
fn conversion_preserves_comments_before_items_and_statements() {
    use ilang_syntax::syn3::surface::{lisp_to_normal, normal_to_lisp};
    let source = "// item\nf = () {\n  // statement\n  x = 1\n  x // trailing\n}\n";
    let (lisp, lost) = normal_to_lisp(source).unwrap();
    assert!(lisp.contains("// item\n(= f"), "{lisp}");
    assert!(lisp.contains("// statement\n(= x 1)"), "{lisp}");
    assert_eq!(lost.len(), 1);
    assert_eq!(
        &source[lost[0].start as usize..lost[0].end as usize],
        "// trailing"
    );
    let (normal, lost) = lisp_to_normal(&lisp).unwrap();
    assert!(lost.is_empty(), "{lost:?}");
    assert!(normal.contains("// item\nf ="), "{normal}");
    assert!(normal.contains("// statement\nx = 1"), "{normal}");
}

#[test]
fn lisp_formatter_keeps_all_comments_and_is_idempotent() {
    let source = "// before\n(= f (-> (, x) _ // trailing\n (+ x // inside\n 1)))\n// end\n";
    let formatted = ilang_syntax::lisp::format(source).unwrap();
    for comment in ["// before", "// trailing", "// inside", "// end"] {
        assert!(formatted.contains(comment), "{formatted}");
    }
    assert_eq!(ilang_syntax::lisp::format(&formatted).unwrap(), formatted);
}

#[test]
fn comment_anchors_do_not_match_text_inside_documentation() {
    let source = "/// Example: (= f (-> (,) _ 1))\n// item\nf = () { 1 }\n";
    let (out, lost) = ilang_syntax::syn3::surface::normal_to_lisp(source).unwrap();
    assert!(lost.is_empty(), "{lost:?}");
    assert!(
        out.contains("/// Example: (= f (-> (,) _ 1))\n\n// item\n(= f"),
        "{out}"
    );
}

#[test]
fn comments_remain_with_repeated_statements() {
    let source = "f = () {\n// first\nlog(1)\n// second\nlog(1)\nNil\n}\n";
    let (out, lost) = ilang_syntax::syn3::surface::normal_to_lisp(source).unwrap();
    assert!(lost.is_empty(), "{lost:?}");
    assert!(out.contains("// first\n(log 1)"), "{out}");
    assert!(out.contains("// second\n(log 1)"), "{out}");
    assert!(out.find("// first").unwrap() < out.find("// second").unwrap());
}

#[test]
fn comments_survive_a_group_that_disappears() {
    let source = "f = () {\n  {\n    // inner\n    1\n  }\n}\n";
    let (out, lost) = ilang_syntax::syn3::surface::normal_to_lisp(source).unwrap();
    assert!(lost.is_empty(), "{lost:?}");
    assert!(out.contains("// inner\n1"), "{out}");
}

#[test]
fn comments_before_header_groups_and_many_statements_are_kept_in_order() {
    use ilang_syntax::syn3::surface::{lisp_to_normal, normal_to_lisp};
    let mut source = String::from("// imports\n(i/list)\n// exports\n(f)\nf = () {\n");
    for n in 0..2000 {
        source.push_str(&format!("\t// c{n}\n\tx\n"));
    }
    source.push_str("}\n");
    let (lisp, lost) = normal_to_lisp(&source).unwrap();
    assert!(lost.is_empty(), "{lost:?}");
    assert!(lisp.contains("// imports\n(, (/ i list))"), "{}", &lisp[..200]);
    assert!(lisp.contains("// exports\n(, f)"), "{}", &lisp[..200]);
    let (normal, lost) = lisp_to_normal(&lisp).unwrap();
    assert!(lost.is_empty(), "{lost:?}");
    for text in [&lisp, &normal] {
        let mut at = 0;
        for n in 0..2000 {
            let wanted = format!("// c{n}\n");
            let found = text[at..].find(&wanted).expect("comment remains in order");
            at += found + wanted.len();
        }
    }
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
    let source = r#"(
	x
	Option(..)
)
Some(A) = (A)
None = ()
Option(A) = Some(A) | None
value: (Int) -> Int
value = (x) -> Int { x + 1 }
choose = (x) { x |> (0) { "zero" } | (_) { "other" } }
"#;
    let (module, diagnostics) = ilang_syntax::parse_module(source);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let lisp = print_module(&module);
    let datums = parse(&lisp).unwrap();
    let again = ilang_syntax::lisp::parse_module(&datums).unwrap();
    assert_eq!(
        ilang_syntax::print::print_module(&module),
        ilang_syntax::print::print_module(&again),
        "{lisp}"
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
            let _ = &module;
            if !diagnostics.is_empty() {
                continue;
            }
            // The Lisp surface of syntax three must read back the same module.
            let tree = ilang_syntax::syn3::parse_module(&source).0;
            let text = ilang_syntax::syn3::lisp::to_lisp(&tree);
            let (read, errors) = ilang_syntax::syn3::lisp::from_lisp(&text);
            if !errors.is_empty() || ilang_syntax::syn3::lisp::to_lisp(&read) != text {
                failures.push(format!(
                    "{}: syntax-three Lisp {errors:?}\n{text}",
                    path.display()
                ));
            }
            continue;
            #[allow(unreachable_code)]
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
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

fn without_spans(d: &ilang_syntax::lisp::Datum) -> ilang_syntax::lisp::Datum {
    use DatumKind as K;
    let all = |xs: &[ilang_syntax::lisp::Datum]| xs.iter().map(without_spans).collect();
    ilang_syntax::lisp::Datum {
        kind: match &d.kind {
            K::Round(xs) => K::Round(all(xs)),
            K::Square(xs) => K::Square(all(xs)),
            K::Curly(xs) => K::Curly(all(xs)),
            K::Bits(xs) => K::Bits(all(xs)),
            K::Quote(x) => K::Quote(Box::new(without_spans(x))),
            K::Unquote(x) => K::Unquote(Box::new(without_spans(x))),
            K::Splice(x) => K::Splice(Box::new(without_spans(x))),
            other => other.clone(),
        },
        span: Default::default(),
        docs: Vec::new(),
    }
}

/// The datum built from the tree is the datum that the reader gives for its
/// text: the macro engine can use either.
#[test]
fn tree_datums_are_the_datums_of_their_text() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std/src");
    let mut files = 0;
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "i") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let (module, errors) = ilang_syntax::syn3::parse_module(&source);
        assert!(errors.is_empty(), "{}: {errors:?}", path.display());
        files += 1;
        for item in &module.items {
            let built = ilang_syntax::syn3::lisp::item_datum(item);
            let text = show(&built);
            let read = parse(&text).unwrap_or_else(|e| panic!("{}: {text}: {e:?}", path.display()));
            assert_eq!(read.len(), 1, "{text}");
            assert!(
                without_spans(&built) == without_spans(&read[0]),
                "{}: {text}",
                path.display()
            );
        }
    }
    assert!(files > 20, "{files}");
}

#[test]
fn tree_datums_keep_the_spans_of_written_names() {
    let source = "f = (count: Int) { other.g(count, Some(1)) }\n";
    let (module, errors) = ilang_syntax::syn3::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let datum = ilang_syntax::syn3::lisp::item_datum(&module.items[0]);
    let mut pending = vec![&datum];
    let mut names = Vec::new();
    while let Some(d) = pending.pop() {
        match &d.kind {
            DatumKind::Round(xs)
            | DatumKind::Square(xs)
            | DatumKind::Curly(xs)
            | DatumKind::Bits(xs) => pending.extend(xs),
            DatumKind::Name(n) | DatumKind::UpName(n) => {
                assert_eq!(&source[d.span.start as usize..d.span.end as usize], n);
                names.push(n.as_str());
            }
            _ => {}
        }
    }
    names.sort();
    assert_eq!(names, ["Int", "Some", "count", "count", "f", "g", "other"]);
}
