use ilang_syntax::ast::{ExprKind, TypeKind};
use ilang_syntax::lisp::{
    Datum, DatumKind as K, parse, parse_expression, parse_pattern, parse_type,
};
use ilang_syntax::span::Span;

fn one(s: &str) -> Datum {
    parse(s).unwrap().datums.remove(0)
}
fn d(kind: K, start: u32) -> Datum {
    Datum {
        kind,
        span: Span::new(start, start + 1),
        docs: vec![],
    }
}
fn sym(s: &str) -> Datum {
    d(K::Symbol(s.into()), 0)
}
fn name(s: &str) -> Datum {
    d(K::Name(s.into()), 0)
}
fn field(label: Datum, value: Datum) -> Datum {
    d(K::Round(vec![sym(":"), label, value]), 0)
}

#[test]
fn record_values_types_and_source_locations_are_preserved() {
    let empty = one("()");
    assert!(
        matches!(&parse_expression(&empty).unwrap().kind, ExprKind::Record { base: None, fields } if fields.is_empty())
    );
    // syntax-3 stage 6: `()` is the empty data type.
    assert!(parse_type(&empty).is_ok());
    assert!(parse_pattern(&empty).is_err());
    let source = one("(.. base (: name 1) (: \"x-y\" value))");
    let K::Round(parts) = &source.kind else {
        panic!()
    };
    let K::Round(named) = &parts[2].kind else {
        panic!()
    };
    let ast = parse_expression(&source).unwrap();
    let ExprKind::Record { base, fields } = &ast.kind else {
        panic!()
    };
    assert!(
        matches!(&base.as_ref().unwrap().kind, ExprKind::Name { name, .. } if name.name == "base")
    );
    assert_eq!(
        fields
            .iter()
            .map(|(n, _)| n.name.as_str())
            .collect::<Vec<_>>(),
        ["name", "x-y"]
    );
    assert_eq!(fields[0].0.span, named[1].span);
    let ty = parse_type(&one("(.. R (- K) (: K Int) (: \"x-y\" String))")).unwrap();
    let TypeKind::Record {
        base,
        fields,
        removed,
    } = &ty.kind
    else {
        panic!()
    };
    assert!(base.is_some());
    assert!(removed[0].parameter && removed[0].name == "K");
    assert!(fields[0].0.parameter && fields[0].0.name == "K");
    assert!(!fields[1].0.parameter && fields[1].0.name == "x-y");
}

#[test]
fn invalid_records_do_not_become_blocks_or_patterns() {
    for input in [
        "{ .. }",
        "{ (: K 1) }",
        "{ (: x 1) .. base }",
        "{ .. base (- x) }",
    ] {
        assert!(parse_expression(&one(input)).is_err(), "{input}");
    }
    for input in ["{ .. }", "{ (: x Int) (- y) }", "{ (: x Int) .. R }"] {
        assert!(parse_type(&one(input)).is_err(), "{input}");
    }
    assert!(parse_pattern(&one("{ (: x p) }")).is_err());
    assert!(matches!(
        &parse_expression(&one("{ x }")).unwrap().kind,
        ExprKind::Block(_)
    ));
    assert!(matches!(
        &parse_type(&one("{ Int }")).unwrap().kind,
        TypeKind::Named { .. }
    ));
    assert!(matches!(
        &parse_expression(&one("`{ (: x ~p) }")).unwrap().kind,
        ExprKind::Quote(_)
    ));
    assert!(matches!(
        &parse_pattern(&one("`{ (: x ~p) }")).unwrap().kind,
        ilang_syntax::ast::PatternKind::Quote { .. }
    ));
}

#[test]
fn record_expression_and_type_handle_ten_thousand_nested_fields_and_spreads() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let mut expr = name("x");
            let mut ty = d(K::UpName("Int".into()), 0);
            let mut expr_field = name("x");
            let mut ty_base = d(K::UpName("R".into()), 0);
            for _ in 0..10_000 {
                expr = d(
                    K::Round(vec![sym(".."), expr, field(name("x"), name("v"))]),
                    0,
                );
                ty = d(K::Round(vec![field(name("x"), ty)]), 0);
                expr_field = d(K::Round(vec![field(name("x"), expr_field)]), 0);
                ty_base = d(K::Round(vec![sym(".."), ty_base]), 0);
            }
            let value = parse_expression(&expr).unwrap();
            let mut p = &value;
            for _ in 0..10_000 {
                let ExprKind::Record {
                    base: Some(base),
                    fields,
                } = &p.kind
                else {
                    panic!()
                };
                assert_eq!(fields.len(), 1);
                p = base;
            }
            assert!(matches!(&p.kind, ExprKind::Name { name, .. } if name.name == "x"));
            let result = parse_type(&ty).unwrap();
            let mut p = &result;
            for _ in 0..10_000 {
                let TypeKind::Record { fields, .. } = &p.kind else {
                    panic!()
                };
                p = &fields[0].1;
            }
            assert!(matches!(&p.kind, TypeKind::Named { name, .. } if name.name == "Int"));
            let value_field = parse_expression(&expr_field).unwrap();
            let mut p = &value_field;
            for _ in 0..10_000 {
                let ExprKind::Record { fields, .. } = &p.kind else {
                    panic!()
                };
                p = &fields[0].1;
            }
            assert!(matches!(&p.kind, ExprKind::Name { name, .. } if name.name == "x"));
            let result_base = parse_type(&ty_base).unwrap();
            let mut p = &result_base;
            for _ in 0..10_000 {
                let TypeKind::Record {
                    base: Some(base), ..
                } = &p.kind
                else {
                    panic!()
                };
                p = base;
            }
            assert!(matches!(&p.kind, TypeKind::Named { name, .. } if name.name == "R"));
            drop((
                expr,
                ty,
                expr_field,
                ty_base,
                value,
                result,
                value_field,
                result_base,
            ));
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn expanded_record_labels_keep_their_original_syntax_identity() {
    use ilang_syntax::lisp::scoped::ScopedSource;
    use ilang_syntax::lisp::syntax::{Node, Scope, Syntax};
    use std::collections::BTreeSet;
    let scope = Scope {
        module: 42,
        marks: BTreeSet::from([7]),
    };
    let source = one("((: name value))");
    let root = Syntax::from_datum(&source, &scope);
    let Node::Round(items) = root.node() else {
        panic!()
    };
    let Node::Round(field) = items[0].node() else {
        panic!()
    };
    let label = field[1].clone();
    let value = field[2].clone();
    let mut scoped = ScopedSource::new("caller".into());
    let result = scoped.expression(&root, &root).unwrap();
    let ExprKind::Record { fields, .. } = &result.kind else {
        panic!()
    };
    assert!(
        scoped
            .origin(fields[0].0.span)
            .unwrap()
            .syntax
            .identical(&label)
    );
    assert!(
        scoped
            .origin(fields[0].1.span)
            .unwrap()
            .syntax
            .identical(&value)
    );
    assert_eq!(
        scoped
            .origin(fields[0].0.span)
            .unwrap()
            .syntax
            .scope()
            .module,
        42
    );
}

#[test]
fn normal_lisp_normal_round_trip_preserves_records() {
    let source = r#"
empty = ()
single = (1)
Single = (Int)
f = (x) -> Int ^ String { x }
first = (name: "Ada", "x-y": 7)
second = (..first, count: 2)
choose(K, R) = (key: K, r: (..R, -other, K: Int)) -> (K: Int) { r }
"#;
    let (normal, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let lisp = ilang_syntax::lisp::print_module(&normal);
    let parsed = ilang_syntax::lisp::parse(&lisp).unwrap();
    let restored = ilang_syntax::lisp::parse_module(&parsed).unwrap();
    assert!(
        ilang_syntax::shape::equivalent(&normal, &restored),
        "{lisp}"
    );
    let back = ilang_syntax::lisp::normal_source(&parsed).unwrap();
    let (again, errors) = ilang_syntax::parse_module(&back);
    assert!(errors.is_empty(), "{back}: {errors:?}");
    assert!(ilang_syntax::shape::equivalent(&normal, &again), "{back}");
}

#[test]
fn malformed_record_field_reports_the_original_label_span() {
    let label = d(K::UpName("NotAValueLabel".into()), 67);
    let source = d(K::Round(vec![field(label, name("value"))]), 1);
    let error = parse_expression(&source).unwrap_err();
    assert_eq!(error.span, Span::new(67, 68));
}
