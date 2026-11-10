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

#[test]
fn expansion_runs_mark_arguments_and_freshen_templates() {
    use ilang_syntax::lisp::expansion::ExpansionSession;
    use ilang_syntax::lisp::syntax::{Node, Syntax};
    let argument = root("x");
    let template = root("y");
    let mut session = ExpansionSession::default();
    let result = session
        .run(
            std::slice::from_ref(&argument),
            &argument,
            |args, _| {
                Syntax::make(
                    Node::Round(vec![args[0].clone(), template.clone()]),
                    &template,
                )
            },
            |_, result| Ok(result),
        )
        .unwrap();
    let Node::Round(parts) = result.node() else {
        panic!("expected round")
    };
    assert_eq!(parts[0].scope(), argument.scope());
    assert_eq!(parts[1].scope().marks.len(), 1);
    let another = session
        .run(
            &[],
            &argument,
            |_, _| Ok(template.clone()),
            |_, result| Ok(result),
        )
        .unwrap();
    assert_ne!(parts[1].scope().marks, another.scope().marks);
}

#[test]
fn expansion_depth_is_bounded_and_restored_after_failure() {
    use ilang_syntax::lisp::expansion::ExpansionSession;
    use ilang_syntax::lisp::syntax::{Syntax, SyntaxError};
    fn nested(
        session: &mut ExpansionSession,
        value: &Syntax,
        n: usize,
    ) -> Result<Syntax, SyntaxError> {
        session.run(
            &[],
            value,
            |_, _| Ok(value.clone()),
            |session, result| {
                if n == 0 {
                    Ok(result)
                } else {
                    nested(session, value, n - 1)
                }
            },
        )
    }
    let value = root("x");
    let mut session = ExpansionSession::default();
    assert!(
        nested(&mut session, &value, 256)
            .unwrap_err()
            .message
            .contains("256")
    );
    nested(&mut session, &value, 1).unwrap();
}

#[test]
fn expansion_marks_compiler_answers_with_the_same_run_mark() {
    use ilang_syntax::lisp::expansion::ExpansionSession;
    let caller = root("call");
    let answer = root("Definition").as_answer();
    let mut session = ExpansionSession::default();
    let result = session
        .run(
            &[],
            &caller,
            |_, mark| Ok(answer.flip_mark(mark)),
            |_, result| Ok(result),
        )
        .unwrap();
    assert_eq!(result.scope(), answer.scope());
    assert_eq!(result.provenance(), answer.provenance());
}

#[test]
fn macro_item_results_accept_only_a_single_item_or_a_flat_curly_list() {
    use ilang_syntax::lisp::{parse, parse_items};
    assert_eq!(parse_items(&parse("{}").unwrap()[0]).unwrap().len(), 0);
    assert_eq!(
        parse_items(&parse("{ (= a 1) (= b 2) }").unwrap()[0])
            .unwrap()
            .len(),
        2
    );
    assert_eq!(parse_items(&parse("(= a 1)").unwrap()[0]).unwrap().len(), 1);
    assert!(parse_items(&parse("{ { (= a 1) } }").unwrap()[0]).is_err());
    assert!(parse_items(&parse("(, a b)").unwrap()[0]).is_err());
}

#[test]
fn macro_declarations_have_a_lisp_binding_and_named_parameters() {
    use ilang_syntax::lisp::{expansion::definitions, parse_source};
    let (module, errors) = parse_source("(@ macro (= m (-> (x y) _ x)))");
    assert!(errors.is_empty(), "{errors:?}");
    let found = definitions(&module, true).unwrap();
    assert_eq!(found[0].name, "m");
    assert_eq!(found[0].parameters, ["x", "y"]);
    assert!(definitions(&module, false).is_err());
    let (typed, errors) = parse_source("(@ macro (= (: m (-> (Syntax) Syntax)) (-> (x) _ x)))");
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(definitions(&typed, true).unwrap()[0].name, "m");
    for source in [
        "(@ macro (= m 1))",
        "(@ macro (= m (-> ((, x y)) _ x)))",
        "(@ (macro) (= m (-> (x) _ x)))",
        "(: m Int) (@ macro (= m (-> (x) _ x)))",
    ] {
        let (module, errors) = parse_source(source);
        assert!(errors.is_empty(), "{source}: {errors:?}");
        assert!(definitions(&module, true).is_err(), "{source}");
    }
}

#[test]
fn macro_arguments_match_labels_and_attribute_items() {
    use ilang_syntax::lisp::expansion::{MacroArgument, MacroDefinition};
    let m = MacroDefinition {
        item: 0,
        name: "m".into(),
        parameters: vec!["first".into(), "second".into(), "item".into()],
    };
    let argument = |label: Option<&str>, text: &str| MacroArgument {
        label: label.map(str::to_string),
        syntax: root(text),
    };
    let call = root("call");
    let supplied = [argument(Some("second"), "b"), argument(Some("first"), "a")];
    let ordered = m.arguments(&supplied, Some(root("body")), &call).unwrap();
    assert_eq!(
        ordered.iter().map(|s| s.show()).collect::<Vec<_>>(),
        ["a", "b", "body"]
    );
    assert!(
        m.arguments(
            &[argument(Some("first"), "a"), argument(None, "b")],
            Some(root("body")),
            &call
        )
        .is_err()
    );
    assert!(
        m.arguments(
            &[argument(None, "a"), argument(Some("first"), "b")],
            Some(root("body")),
            &call
        )
        .is_err()
    );
    assert!(
        m.arguments(
            &[argument(None, "a"), argument(Some("item"), "b")],
            Some(root("body")),
            &call
        )
        .is_err()
    );
    assert!(
        m.arguments(
            &[argument(None, "a"), argument(Some("unknown"), "b")],
            Some(root("body")),
            &call
        )
        .is_err()
    );
}

#[test]
fn compiler_answer_names_keep_their_own_identity() {
    use ilang_syntax::lisp::syntax::{Node, ScopeIdentity, Syntax, validate_expansion};
    let parameter = root("Key").with_scope_identity(ScopeIdentity::DefinitionParameter {
        answer: 12,
        parameter: 0,
        name: "Key".into(),
    });
    let copied = parameter.flip_mark(7).flip_mark(7);
    assert_eq!(copied.scope_identity(), parameter.scope_identity());
    let same = Syntax::make(Node::Up("Key".into()), &parameter).unwrap();
    assert_eq!(same.scope_identity(), parameter.scope_identity());
    validate_expansion(&same, &[]).unwrap();
    let changed = Syntax::make(Node::Up("Other".into()), &parameter).unwrap();
    assert!(validate_expansion(&changed, &[]).is_err());
    let other_answer = root("Key").with_scope_identity(ScopeIdentity::DefinitionParameter {
        answer: 13,
        parameter: 0,
        name: "Key".into(),
    });
    assert!(parameter.same_code(&other_answer));
    assert_ne!(parameter.scope_identity(), other_answer.scope_identity());
}

#[test]
fn builtin_definitions_preserve_builtins_and_fresh_parameter_binders() {
    use ilang_syntax::lisp::definitions::builtin_definition;
    use ilang_syntax::lisp::syntax::{Node, Position, ScopeIdentity};
    let like = root("x");
    for name in ["Bool", "Nil", "Result", "Assert", "AssertionFailure"] {
        let result = builtin_definition(name, like.scope(), like.span(), 3).unwrap();
        result.show_as(Position::Item).unwrap();
    }
    assert!(builtin_definition("Int", like.scope(), like.span(), 3).is_none());
    let first = builtin_definition("Result", like.scope(), like.span(), 3).unwrap();
    let second = builtin_definition("Result", like.scope(), like.span(), 4).unwrap();
    let Node::Round(parts) = first.node() else {
        panic!("item")
    };
    let Node::Round(head) = parts[0].node() else {
        panic!("head")
    };
    assert!(
        matches!(head[0].scope_identity(), Some(ScopeIdentity::Builtin { name }) if name == "Result")
    );
    assert!(matches!(
        head[1].scope_identity(),
        Some(ScopeIdentity::DefinitionParameter {
            answer: 3,
            parameter: 0,
            ..
        })
    ));
    let Node::Curly(constructors) = parts[1].node() else {
        panic!("constructors")
    };
    let Node::Round(ok) = constructors[0].node() else {
        panic!("constructor")
    };
    assert_eq!(head[1].scope_identity(), ok[1].scope_identity());
    let Node::Round(parts2) = second.node() else {
        panic!("item")
    };
    let Node::Round(head2) = parts2[0].node() else {
        panic!("head")
    };
    assert_ne!(head[1].scope_identity(), head2[1].scope_identity());
}

#[test]
fn written_definition_answers_give_parameters_their_own_binder() {
    use ilang_syntax::lisp::definitions::source_definition;
    use ilang_syntax::lisp::syntax::{Node, ScopeIdentity};
    let source = "Pair(A, B) { Pair(left: A, right: List(B)) }";
    let (module, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let like = root("x");
    let result = source_definition(&module.items[0], source, like.scope(), 17);
    let Node::Round(parts) = result.node() else {
        panic!("item")
    };
    let Node::Round(head) = parts[0].node() else {
        panic!("head")
    };
    let Node::Curly(body) = parts[1].node() else {
        panic!("body")
    };
    let Node::Round(ctor) = body[0].node() else {
        panic!("constructor")
    };
    let Node::Round(field) = ctor[1].node() else {
        panic!("field")
    };
    assert!(matches!(
        head[1].scope_identity(),
        Some(ScopeIdentity::DefinitionParameter {
            answer: 17,
            parameter: 0,
            ..
        })
    ));
    assert_eq!(head[1].scope_identity(), field[2].scope_identity());
    assert!(ctor[0].scope_identity().is_none());
}

#[test]
fn copied_definition_freshens_its_binders_and_uses_together() {
    use ilang_syntax::lisp::definitions::{builtin_definition, freshen_parameters};
    use ilang_syntax::lisp::syntax::{Node, ScopeIdentity};
    let like = root("x");
    let original = builtin_definition("Result", like.scope(), like.span(), 3).unwrap();
    let copy = freshen_parameters(&original, 3, 8);
    let Node::Round(parts) = copy.node() else {
        panic!("item")
    };
    let Node::Round(head) = parts[0].node() else {
        panic!("head")
    };
    let Node::Curly(constructors) = parts[1].node() else {
        panic!("body")
    };
    let Node::Round(ok) = constructors[0].node() else {
        panic!("constructor")
    };
    assert!(matches!(
        head[1].scope_identity(),
        Some(ScopeIdentity::DefinitionParameter {
            answer: 8,
            parameter: 0,
            ..
        })
    ));
    assert_eq!(head[1].scope_identity(), ok[1].scope_identity());
    assert!(matches!(
        head[0].scope_identity(),
        Some(ScopeIdentity::Builtin { .. })
    ));
    assert!(copy.same_code(&original));
}

#[test]
fn definition_query_accepts_only_type_names_and_keeps_atom_scopes() {
    use ilang_syntax::lisp::definitions::type_query;
    use ilang_syntax::lisp::syntax::{Node, ScopeIdentity, Syntax};
    let like = root("x");
    let dot = Syntax::make(Node::Symbol(".".into()), &like).unwrap();
    let module = root("mod").flip_mark(9);
    let ty = root("Result").with_scope_identity(ScopeIdentity::Builtin {
        name: "Result".into(),
    });
    let qualified =
        Syntax::make(Node::Round(vec![dot, module.clone(), ty.clone()]), &like).unwrap();
    let query = type_query(&qualified).unwrap();
    assert_eq!(query.module.unwrap().scope(), module.scope());
    assert_eq!(query.name.scope_identity(), ty.scope_identity());
    assert!(type_query(&root("(List Int)")).is_err());
    assert!(type_query(&root("value")).is_err());
    assert!(type_query(&root("(. mod value)")).is_err());
}

#[test]
fn definition_schedule_retries_fresh_runs_after_a_contribution() {
    use ilang_syntax::lisp::definition_schedule::{Attempt, schedule};
    use std::collections::BTreeMap;
    let mut state = BTreeMap::new();
    let mut attempts = Vec::new();
    let mut session = ilang_syntax::lisp::expansion::ExpansionSession::default();
    let mut marks = Vec::new();
    let like = root("call");
    let problems: Vec<ilang_syntax::lisp::definition_schedule::Problem<_, _, _, ()>> = schedule(
        &mut state,
        ["uses", "defines"],
        |state, call, _| {
            attempts.push(*call);
            session
                .run(
                    &[],
                    &like,
                    |_, mark| {
                        marks.push(mark);
                        Ok(like.clone())
                    },
                    |_, result| Ok(result),
                )
                .unwrap();
            match *call {
                "uses" if !state.contains_key("T") => Attempt::Deferred {
                    query: "T",
                    span: ilang_syntax::span::Span::new(4, 5),
                },
                "uses" => Attempt::Complete {
                    product: ("U", 2),
                    answers: vec![],
                },
                _ => Attempt::Complete {
                    product: ("T", 1),
                    answers: vec![],
                },
            }
        },
        |state, _, (name, value)| {
            state.insert(name, value);
            vec![]
        },
        |state, query| state.get(query).copied(),
    );
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(attempts, ["uses", "defines", "uses"]);
    assert_eq!(marks, [0, 1, 2]);
    assert_eq!(state.get("U"), Some(&2));
}

#[test]
fn definition_schedule_reports_waiters_and_changed_fallback_answers() {
    use ilang_syntax::lisp::definition_schedule::{Attempt, Mode, Problem, schedule};
    use ilang_syntax::span::Span;
    use std::collections::BTreeMap;
    let mut state = BTreeMap::new();
    let problems: Vec<Problem<_, _, _, ()>> = schedule(
        &mut state,
        ["left", "right"],
        |_, call, _| Attempt::Deferred {
            query: if *call == "left" { "B" } else { "A" },
            span: if *call == "left" {
                Span::new(1, 2)
            } else {
                Span::new(3, 4)
            },
        },
        |_, _, ()| vec![],
        |state: &BTreeMap<&str, i32>, query| state.get(query).copied(),
    );
    assert_eq!(
        problems,
        vec![
            Problem::Waiting {
                call: "left",
                query: "B",
                span: Span::new(1, 2)
            },
            Problem::Waiting {
                call: "right",
                query: "A",
                span: Span::new(3, 4)
            }
        ]
    );
    let mut state = BTreeMap::new();
    let problems: Vec<Problem<_, _, _, ()>> = schedule(
        &mut state,
        ["ask", "define"],
        |_, call, mode| match (*call, mode) {
            ("ask", Mode::Fallback) => Attempt::Complete {
                product: None,
                answers: vec![("T", 1)],
            },
            ("define", Mode::Normal) => Attempt::Deferred {
                query: "T",
                span: Span::new(5, 6),
            },
            ("define", Mode::Fallback) => Attempt::Complete {
                product: Some(("T", 2)),
                answers: vec![],
            },
            _ => Attempt::Deferred {
                query: "T",
                span: Span::new(2, 3),
            },
        },
        |state, _, product| {
            if let Some((name, value)) = product {
                state.insert(name, value);
            }
            vec![]
        },
        |state, query| state.get(query).copied(),
    );
    assert_eq!(
        problems,
        vec![Problem::Changed {
            call: "ask",
            query: "T",
            before: 1,
            after: Some(2)
        }]
    );
}

#[test]
fn definition_schedule_expands_new_items_before_later_siblings() {
    use ilang_syntax::lisp::definition_schedule::{Attempt, Problem, schedule};
    let mut state = Vec::new();
    let problems: Vec<Problem<_, (), (), ()>> = schedule(
        &mut state,
        ["outer", "sibling"],
        |_, call, _| Attempt::Complete {
            product: *call,
            answers: vec![],
        },
        |state, _, name| {
            state.push(name);
            if name == "outer" {
                vec!["inner1", "inner2"]
            } else {
                vec![]
            }
        },
        |_, _| None,
    );
    assert!(problems.is_empty());
    assert_eq!(state, ["outer", "inner1", "inner2", "sibling"]);
}

#[test]
fn alias_pattern_query_preserves_the_written_definition() {
    use ilang_syntax::lisp::definitions::source_definition;
    use ilang_syntax::lisp::syntax::{Node, Position};
    let source = "Shape(Left, Right) = value";
    let (module, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let like = root("x");
    let result = source_definition(&module.items[0], source, like.scope(), 9);
    let Node::Round(parts) = result.node() else {
        panic!("alias")
    };
    assert!(matches!(parts[0].node(), Node::Symbol(s) if s == "="));
    let Node::Round(head) = parts[1].node() else {
        panic!("alias head")
    };
    assert!(head[1].scope_identity().is_none());
    assert!(head[2].scope_identity().is_none());
    assert!(result.show_as(Position::Item).unwrap().contains("value"));
}
