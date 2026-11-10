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
    let mut failures = Vec::new();
    for file in std::fs::read_dir(dir).unwrap() {
        let path = file.unwrap().path();
        if path.extension().is_none_or(|ext| ext != "i") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let (module, diagnostics) = ilang_syntax::parse_module(&source);
        if !diagnostics.is_empty() {
            continue;
        }
        let lisp = print_module(&module);
        let converted = parse(&lisp)
            .map_err(|e| format!("datum: {e:?}"))
            .and_then(|ds| normal_source(&ds).map_err(|e| format!("conversion: {e:?}")));
        match converted {
            Err(e) => failures.push(format!("{}: {e}\n{lisp}", path.display())),
            Ok(normal) => {
                let (back, errors) = ilang_syntax::parse_module(&normal);
                if !errors.is_empty() || print_module(&module) != print_module(&back) {
                    failures.push(format!(
                        "{}: {errors:?}\n{lisp}\n{normal}\nBACK:\n{}",
                        path.display(),
                        print_module(&back)
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
