use ilang_syntax::lisp::{
    expression_syntax, item_syntax, parse,
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
fn normal_source_encodes_to_syntax_with_original_leaf_spans() {
    let source = "f = (x) { x + 2 }\n";
    let (module, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let ilang_syntax::ast::ItemKind::Binding(binding) = &module.items[0].kind else {
        panic!()
    };
    let syntax = expression_syntax(
        &binding.value,
        &Scope {
            module: 9,
            marks: BTreeSet::new(),
        },
        source,
    );
    assert_eq!(syntax.show(), "(-> (x) _ {(+ x 2)})");
    let Node::Round(function) = syntax.node() else {
        panic!()
    };
    let Node::Curly(body) = function[3].node() else {
        panic!()
    };
    let Node::Round(sum) = body[0].node() else {
        panic!()
    };
    let x = &sum[1];
    assert_eq!(&source[x.span().start as usize..x.span().end as usize], "x");
    assert_eq!(x.scope().module, 9);
    assert_eq!(
        &source[sum[0].span().start as usize..sum[0].span().end as usize],
        "+"
    );
}

#[test]
fn normal_library_bindings_encode_without_reparsing_the_source() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut dirs = vec![root.join("lib"), root.join("std")];
    let scope = Scope {
        module: 0,
        marks: BTreeSet::new(),
    };
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "i") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            let (module, errors) = ilang_syntax::parse_module(&source);
            assert!(errors.is_empty(), "{}: {errors:?}", path.display());
            for item in module.items {
                let mut clean = item.clone();
                clean.doc.clear();
                let expected_item = ilang_syntax::ast::Module {
                    items: vec![clean],
                    ..Default::default()
                };
                assert_eq!(
                    item_syntax(&item, &scope, &source).show(),
                    ilang_syntax::lisp::print_module(&expected_item).trim_end(),
                    "{}",
                    path.display()
                );
                let ilang_syntax::ast::ItemKind::Binding(binding) = &item.kind else {
                    continue;
                };
                let syntax = expression_syntax(&binding.value, &scope, &source);
                let datum = parse(&syntax.show()).unwrap();
                let decoded = ilang_syntax::lisp::parse_expression(&datum[0]).unwrap();
                let expected = ilang_syntax::ast::Module {
                    items: vec![item.clone()],
                    ..Default::default()
                };
                let mut actual = expected.clone();
                let ilang_syntax::ast::ItemKind::Binding(b) = &mut actual.items[0].kind else {
                    unreachable!()
                };
                b.value = decoded;
                assert_eq!(
                    ilang_syntax::shape::canon(&expected),
                    ilang_syntax::shape::canon(&actual),
                    "{}: {}",
                    path.display(),
                    syntax.show()
                );
            }
        }
    }
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

#[test]
fn expansion_cannot_create_or_duplicate_foreign_authority() {
    use ilang_syntax::lisp::syntax::{Node, Syntax, validate_expansion};
    let argument = root("(@ (external \"env\" \"f\") (: f (-> () Int)))");
    let marked = argument.flip_mark(7);
    let returned = marked.flip_mark(7);
    assert!(argument.same_origin(&returned));
    validate_expansion(&returned, std::slice::from_ref(&argument)).unwrap();
    let duplicate = Syntax::make(
        Node::Curly(vec![returned.clone(), returned.clone()]),
        &returned,
    )
    .unwrap();
    assert!(validate_expansion(&duplicate, std::slice::from_ref(&argument)).is_err());
    let Node::Round(parts) = returned.node() else {
        panic!("attribute is round")
    };
    let rebuilt = Syntax::make(Node::Round(parts.clone()), &returned).unwrap();
    assert!(!argument.same_origin(&rebuilt));
    assert!(validate_expansion(&rebuilt, std::slice::from_ref(&argument)).is_err());
    assert!(validate_expansion(&root("`x"), &[]).is_err());
    assert!(validate_expansion(&root("(@ macro (= f x))"), &[]).is_err());
}

#[test]
fn marking_preserves_shared_value_identity_across_arguments() {
    use ilang_syntax::lisp::syntax::{Node, Syntax, flip_mark_all};
    let shared = root("x");
    let pair = Syntax::make(Node::Round(vec![shared.clone(), shared.clone()]), &shared).unwrap();
    let marked = flip_mark_all(&[pair, shared], 99);
    let Node::Round(parts) = marked[0].node() else {
        panic!("expected round")
    };
    assert!(parts[0].identical(&parts[1]));
    assert!(parts[0].identical(&marked[1]));
}
