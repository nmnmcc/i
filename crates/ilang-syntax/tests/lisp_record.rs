//! Data in the Lisp rows of docs/design/syntax-3.md §6.8: `(, e (= x v))`,
//! `(, .. e (= x v))`, `(,)`. These are the forms that the record rows
//! `(.. r (: x v))`, `((: x v))` and `()` became; a record type with a base
//! or a removed key, and a key parameter of a record, have no row any more.
#[path = "oracle.rs"]
mod ilang_syntax;
use ilang_syntax::ast::ExprKind as OldExprKind;
use ilang_syntax::lisp::{Datum, DatumKind as K, parse};
use ilang_syntax::span::Span;
use ilang_syntax::syn3::ast::{DItem, DPItem, ExprKind, PatternKind, TItem, TypeKind};
use ilang_syntax::syn3::lisp::{
    expr_from_datum as parse_expression, pattern_from_datum as parse_pattern,
    type_from_datum as parse_type,
};

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
    d(K::Round(vec![sym("="), label, value]), 0)
}
fn data(items: Vec<Datum>) -> Datum {
    d(
        K::Round(std::iter::once(sym(",")).chain(items).collect()),
        0,
    )
}

#[test]
fn record_values_types_and_source_locations_are_preserved() {
    // The empty Round has no position; the empty data is `(,)` everywhere.
    let round = one("()");
    assert!(parse_expression(&round).is_err());
    assert!(parse_type(&round).is_err());
    assert!(parse_pattern(&round).is_err());
    let empty = one("(,)");
    assert!(
        matches!(&parse_expression(&empty).unwrap().kind, ExprKind::Data(items) if items.is_empty())
    );
    assert!(matches!(&parse_type(&empty).unwrap().kind, TypeKind::Data(items) if items.is_empty()));
    assert!(
        matches!(&parse_pattern(&empty).unwrap().kind, PatternKind::Data(items) if items.is_empty())
    );
    let source = one("(, .. base (= name 1) (= x_y value))");
    let K::Round(parts) = &source.kind else {
        panic!()
    };
    let K::Round(named) = &parts[3].kind else {
        panic!()
    };
    let ast = parse_expression(&source).unwrap();
    let ExprKind::Data(items) = &ast.kind else {
        panic!()
    };
    let [
        DItem::Spread(base),
        DItem::Label(first, _),
        DItem::Label(second, _),
    ] = items.as_slice()
    else {
        panic!("{items:?}")
    };
    assert!(matches!(&base.kind, ExprKind::Name { name, .. } if name.name == "base"));
    assert_eq!([first.name.as_str(), second.name.as_str()], ["name", "x_y"]);
    assert_eq!(first.span.0, named[1].span);
    // A label is a name: the text labels of records (`"x-y"`) have no row.
    assert!(parse_expression(&one("(, (= \"x-y\" value))")).is_err());
    let ty = parse_type(&one("(, (= k Int) (= x_y String))")).unwrap();
    let TypeKind::Data(items) = &ty.kind else {
        panic!()
    };
    let [TItem::Label(first, _), TItem::Label(second, _)] = items.as_slice() else {
        panic!("{items:?}")
    };
    assert_eq!([first.name.as_str(), second.name.as_str()], ["k", "x_y"]);
    // A data type has no base and no removed key.
    assert!(parse_type(&one("(, .. R (= k Int))")).is_err());
    assert!(parse_type(&one("(, (- k) (= k Int))")).is_err());
    assert!(parse_type(&one("(, (= K Int))")).is_err());
}

#[test]
fn invalid_records_do_not_become_blocks_or_patterns() {
    for input in [
        "{ .. }",
        "{ (= x 1) .. base }",
        "{ .. base (= x 1) }",
        "(.. base (= x 1))",
        "((= x 1))",
    ] {
        assert!(parse_expression(&one(input)).is_err(), "{input}");
    }
    for input in [
        "{ .. }",
        "{ (= x Int) (- y) }",
        "{ (= x Int) .. R }",
        "(.. R (= x Int))",
        "((= x Int))",
    ] {
        assert!(parse_type(&one(input)).is_err(), "{input}");
    }
    assert!(parse_pattern(&one("{ (= x p) }")).is_err());
    assert!(matches!(
        &parse_expression(&one("{ x }")).unwrap().kind,
        ExprKind::Block(_)
    ));
    assert!(matches!(
        &parse_type(&one("{ Int }")).unwrap().kind,
        TypeKind::Named { .. }
    ));
    assert!(matches!(
        &parse_expression(&one("`{ (= x ~p) }")).unwrap().kind,
        ExprKind::Quote(_)
    ));
    assert!(matches!(
        &parse_pattern(&one("`{ (= x ~p) }")).unwrap().kind,
        PatternKind::Quote { .. }
    ));
}

#[test]
fn record_expression_and_type_handle_ten_thousand_nested_fields_and_spreads() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            // The reader is a plain recursive function; a deep datum is read
            // on the stack of `syn3::deep::deep`.
            use ilang_syntax::syn3::deep::deep;
            let mut expr = name("x");
            let mut ty = d(K::UpName("Int".into()), 0);
            let mut expr_field = name("x");
            let mut pattern = name("x");
            for _ in 0..10_000 {
                expr = data(vec![sym(".."), expr, field(name("x"), name("v"))]);
                ty = data(vec![field(name("x"), ty)]);
                expr_field = data(vec![field(name("x"), expr_field)]);
                pattern = data(vec![field(name("x"), pattern)]);
            }
            let value = deep(|| parse_expression(&expr)).unwrap();
            let mut p = &value;
            for _ in 0..10_000 {
                let ExprKind::Data(items) = &p.kind else {
                    panic!()
                };
                let [DItem::Spread(base), DItem::Label(..)] = items.as_slice() else {
                    panic!()
                };
                p = base;
            }
            assert!(matches!(&p.kind, ExprKind::Name { name, .. } if name.name == "x"));
            let result = deep(|| parse_type(&ty)).unwrap();
            let mut p = &result;
            for _ in 0..10_000 {
                let TypeKind::Data(items) = &p.kind else {
                    panic!()
                };
                let [TItem::Label(_, inner)] = items.as_slice() else {
                    panic!()
                };
                p = inner;
            }
            assert!(matches!(&p.kind, TypeKind::Named { name, .. } if name.name == "Int"));
            let value_field = deep(|| parse_expression(&expr_field)).unwrap();
            let mut p = &value_field;
            for _ in 0..10_000 {
                let ExprKind::Data(items) = &p.kind else {
                    panic!()
                };
                let [DItem::Label(_, inner)] = items.as_slice() else {
                    panic!()
                };
                p = inner;
            }
            assert!(matches!(&p.kind, ExprKind::Name { name, .. } if name.name == "x"));
            let result_pattern = deep(|| parse_pattern(&pattern)).unwrap();
            let mut p = &result_pattern;
            for _ in 0..10_000 {
                let PatternKind::Data(items) = &p.kind else {
                    panic!()
                };
                let [DPItem::Key(_, inner)] = items.as_slice() else {
                    panic!()
                };
                p = inner;
            }
            assert!(matches!(&p.kind, PatternKind::Var(name) if name == "x"));
            drop((
                expr,
                ty,
                expr_field,
                pattern,
                value,
                result,
                value_field,
                result_pattern,
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
    let source = one("(, (= name value))");
    let root = Syntax::from_datum(&source, &scope);
    let Node::Round(items) = root.node() else {
        panic!()
    };
    let Node::Round(field) = items[1].node() else {
        panic!()
    };
    let label = field[1].clone();
    let value = field[2].clone();
    let mut scoped = ScopedSource::new("caller".into());
    let result = scoped.expression(&root, &root).unwrap();
    let OldExprKind::Record { fields, .. } = &result.kind else {
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
    let source = r#"empty = ()
single = (1)
Single = { (Int) }
f = (x) -> Int { x }
first = (name = "Ada", x_y = 7)
second = (..first, count = 2)
choose = <K, R>(key: K, r: (k = Int)) -> (k = Int) { r }
"#;
    let (normal, errors) = ilang_syntax::syn3::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let lisp = ilang_syntax::syn3::lisp::to_lisp(&normal);
    for row in [
        "(= empty (,))",
        "(= single (, 1))",
        "(= Single {(, Int)})",
        "(= first (, (= name \"Ada\") (= x_y 7)))",
        "(= second (, .. first (= count 2)))",
    ] {
        assert!(lisp.contains(row), "{row}\n{lisp}");
    }
    let (restored, errors) = ilang_syntax::syn3::lisp::from_lisp(&lisp);
    assert!(errors.is_empty(), "{errors:?}\n{lisp}");
    assert!(normal == restored, "{lisp}");
    let (text, lost) = ilang_syntax::syn3::surface::lisp_to_normal(&lisp).unwrap();
    assert!(lost.is_empty(), "{lost:?}");
    let (again, errors) = ilang_syntax::syn3::parse_module(&text);
    assert!(errors.is_empty(), "{errors:?}\n{text}");
    assert!(normal == again, "{text}");
}

#[test]
fn malformed_record_field_reports_the_original_label_span() {
    let label = d(K::UpName("NotAValueLabel".into()), 67);
    let source = data(vec![field(label, name("value"))]);
    let error = parse_expression(&source).unwrap_err();
    assert_eq!(error.span, Span::new(67, 68));
}
