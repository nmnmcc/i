use ilang_syntax::lisp::{
    parse,
    syntax::{Node, Position, Provenance, Scope, Substitution, Syntax},
};
use std::collections::BTreeSet;

fn root(source: &str) -> Syntax {
    let document = parse(source).unwrap();
    Syntax::from_datum(
        &document[0],
        &Scope {
            module: 7,
            marks: BTreeSet::new(),
        },
    )
}

#[test]
fn templates_splice_and_patterns_capture_without_losing_origin() {
    let template = root("`(f ~x ~..xs)");
    let x = root("item");
    let y = root("another");
    let expanded = template
        .instantiate(&mut |expr| match expr.show().as_str() {
            "x" => Ok(Substitution::One(x.clone())),
            "xs" => Ok(Substitution::Many(vec![y.clone()])),
            _ => unreachable!(),
        })
        .unwrap();
    assert_eq!(expanded.provenance(), Provenance::Written);
    assert_eq!(expanded.show(), "(f item another)");
    let Node::Round(parts) = expanded.node() else {
        panic!()
    };
    assert!(parts[1].identical(&x));
    let pattern = root("`(f ~first ~..rest)");
    let captures = pattern.match_quote(&expanded).unwrap().unwrap();
    assert_eq!(captures.len(), 2);
    assert_eq!(captures[0].pattern.show(), "first");
    assert!(matches!(&captures[0].value, Substitution::One(value) if value.identical(&x)));
    assert!(
        matches!(&captures[1].value, Substitution::Many(values) if values.len() == 1 && values[0].identical(&y))
    );
    assert!(root("`(g ~x)").match_quote(&expanded).unwrap().is_none());
    assert!(root("`(~..xs x)").match_quote(&expanded).is_err());
}

#[test]
fn show_as_checks_the_requested_syntactic_position() {
    assert_eq!(
        root("(+ 1 2)").show_as(Position::Expression).unwrap(),
        "1 + 2"
    );
    assert!(root("`(f x)").show_as(Position::Expression).is_err());
    assert!(root("(,)").show_as(Position::Type).is_err());
}

#[test]
fn marks_flip_recursively_and_code_comparison_ignores_them() {
    let original = root("(f x)");
    let marked = original.flip_mark(42);
    assert!(original.same_code(&marked));
    assert!(!original.identical(&marked));
    assert!(marked.scope().marks.contains(&42));
    let Node::Round(children) = marked.node() else {
        panic!()
    };
    assert!(children.iter().all(|c| c.scope().marks.contains(&42)));
    let unmarked = marked.flip_mark(42);
    assert_eq!(unmarked.scope(), original.scope());
    assert!(unmarked.same_code(&original));
    assert!(original.identical(&original.clone()));
}

#[test]
fn made_atoms_are_one_valid_token_and_keep_origin() {
    let like = root("f");
    let made = Syntax::make(Node::Name("fresh".into()), &like).unwrap();
    assert_eq!(made.scope(), like.scope());
    assert_eq!(made.span(), like.span());
    for node in [
        Node::Name("Foo".into()),
        Node::Name("a b".into()),
        Node::Discard("x".into()),
        Node::Symbol("//".into()),
        Node::Float(-0.0),
        Node::Float(f64::INFINITY),
    ] {
        assert!(Syntax::make(node, &like).is_err());
    }
    assert!(Syntax::make(Node::Symbol("^".into()), &like).is_ok());
    let from_answer = like.as_answer();
    let made = Syntax::make(Node::Name("x".into()), &from_answer).unwrap();
    assert_eq!(made.provenance(), Provenance::Made { from_answer: true });
    let again = Syntax::make(Node::Name("y".into()), &made).unwrap();
    assert_eq!(
        again.flip_mark(4).provenance(),
        Provenance::Made { from_answer: true }
    );
}
