use ilang_syntax::lisp::{
    expression_syntax, parse,
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
        Node::Symbol("< -".into()),
        Node::Float(-0.0),
        Node::Float(f64::INFINITY),
    ] {
        assert!(Syntax::make(node, &like).is_err());
    }
    // Spec §1.5 cuts `<-` as one token on both surfaces (§14.1).
    assert!(Syntax::make(Node::Symbol("<-".into()), &like).is_ok());
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
fn macro_arguments_match_labels_and_attribute_items() {
    use ilang_syntax::lisp::expansion::{MacroArgument, MacroDefinition};
    let m = MacroDefinition {
        signature: None,
        variants: Vec::new(),
        item: 0,
        name: "m".into(),
        parameters: vec!["first".into(), "second".into(), "item".into()],
        labels: vec![Some("first".into()), Some("second".into()), None],
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
    let source = "Pair(A, B) = (left = A, right = List(B))";
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

#[test]
fn definition_table_distinguishes_waits_catchable_and_compile_errors() {
    use ilang_syntax::lisp::definition_schedule::Mode;
    use ilang_syntax::lisp::definition_table::{Definition, DefinitionTable, Lookup, QueryResult};
    let mut table = DefinitionTable::default();
    let source = "Box(A) = { (A) }";
    let (module, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty());
    let of = root("Box");
    table.insert(
        1,
        Definition::Written {
            item: Box::new(module.items[0].clone()),
            source: source.into(),
            scope: of.scope().clone(),
            ready: false,
        },
    );
    table.insert(2, Definition::Builtin { name: "Int".into() });
    table.insert(
        3,
        Definition::Builtin {
            name: "Result".into(),
        },
    );
    assert!(matches!(
        table.query(&of, Mode::Normal, 5, |_, _| Lookup::Found(1)),
        QueryResult::Deferred { .. }
    ));
    table.set_ready(&1, true);
    assert!(matches!(
        table.query(&of, Mode::Normal, 6, |_, _| Lookup::Found(1)),
        QueryResult::Answer { key: 1, .. }
    ));
    assert!(matches!(
        table.query(&root("Int"), Mode::Normal, 7, |_, _| Lookup::Found(2)),
        QueryResult::Error(_)
    ));
    assert!(matches!(
        table.query(&root("Result"), Mode::Normal, 8, |_, _| Lookup::Found(3)),
        QueryResult::Answer { key: 3, .. }
    ));
    assert!(matches!(
        table.query(&of, Mode::Normal, 9, |_, _| Lookup::TypeParameter),
        QueryResult::Error(_)
    ));
    assert!(matches!(
        table.query(&of, Mode::Normal, 10, |_, _| Lookup::ConstructorsHidden),
        QueryResult::Error(_)
    ));
    assert!(matches!(
        table.query(&of, Mode::Normal, 11, |_, _| Lookup::Missing),
        QueryResult::CompileError(_)
    ));
    assert!(matches!(
        table.query(&root("1"), Mode::Normal, 12, |_, _| Lookup::Found(1)),
        QueryResult::Error(_)
    ));
}

#[test]
fn definition_table_keeps_scopes_from_expanded_type_items() {
    use ilang_syntax::lisp::definition_schedule::Mode;
    use ilang_syntax::lisp::definition_table::{Definition, DefinitionTable, Lookup, QueryResult};
    use ilang_syntax::lisp::definitions::builtin_definition;
    use ilang_syntax::lisp::syntax::{Node, ScopeIdentity};
    let like = root("Result").flip_mark(11);
    let syntax = builtin_definition("Result", like.scope(), like.span(), 1).unwrap();
    let mut table = DefinitionTable::default();
    table.insert(
        1,
        Definition::Expanded {
            syntax,
            parameter_answer: Some(1),
            ready: true,
        },
    );
    let QueryResult::Answer { syntax, .. } =
        table.query(&like, Mode::Normal, 2, |_, _| Lookup::Found(1))
    else {
        panic!("expected answer")
    };
    let Node::Round(parts) = syntax.node() else {
        panic!("item")
    };
    let Node::Round(head) = parts[0].node() else {
        panic!("head")
    };
    assert!(head[1].scope().marks.contains(&11));
    assert!(matches!(
        head[1].scope_identity(),
        Some(ScopeIdentity::DefinitionParameter {
            answer: 2,
            parameter: 0,
            ..
        })
    ));
}

#[test]
fn expanded_definition_identifies_only_parameters_with_the_same_scope() {
    use ilang_syntax::lisp::definitions::expanded_definition;
    use ilang_syntax::lisp::syntax::{Node, ScopeIdentity, Syntax};
    let template = root("((Box A) { (Box A) })");
    let Node::Round(parts) = template.node() else {
        panic!("type item")
    };
    let Node::Curly(constructors) = parts[1].node() else {
        panic!("constructors")
    };
    let Node::Round(fields) = constructors[0].node() else {
        panic!("constructor")
    };
    let other_scope = fields[1].flip_mark(22);
    let constructor = Syntax::make(
        Node::Round(vec![fields[0].clone(), other_scope.clone()]),
        &constructors[0],
    )
    .unwrap();
    let body = Syntax::make(Node::Curly(vec![constructor]), &parts[1]).unwrap();
    let item = Syntax::make(Node::Round(vec![parts[0].clone(), body]), &template).unwrap();
    let answer = expanded_definition(&item, 41).unwrap();
    let Node::Round(parts) = answer.node() else {
        panic!("item")
    };
    let Node::Round(head) = parts[0].node() else {
        panic!("head")
    };
    let Node::Curly(body) = parts[1].node() else {
        panic!("body")
    };
    let Node::Round(fields) = body[0].node() else {
        panic!("constructor")
    };
    assert!(matches!(
        head[1].scope_identity(),
        Some(ScopeIdentity::DefinitionParameter {
            answer: 41,
            parameter: 0,
            ..
        })
    ));
    assert!(fields[1].scope_identity().is_none());
    assert_eq!(fields[1].scope(), other_scope.scope());
}

#[test]
fn template_rebuild_preserves_origin_and_cannot_forge_foreign_attributes() {
    use ilang_syntax::lisp::syntax::{Node, validate_expansion};
    let template = root("(f x)");
    let Node::Round(original) = template.node() else {
        panic!("round template")
    };
    let inserted = root("y").flip_mark(7);
    let rebuilt = template.replace_children(vec![original[0].clone(), inserted.clone()]);
    assert!(template.same_origin(&rebuilt));
    assert_eq!(rebuilt.scope(), template.scope());
    assert_eq!(rebuilt.span(), template.span());
    assert_eq!(rebuilt.provenance(), template.provenance());
    let Node::Round(children) = rebuilt.node() else {
        panic!("rebuilt")
    };
    assert!(children[1].identical(&inserted));
    let attribute = root("(@ (external \"env\" \"f\") (: f (-> () Int)))");
    let Node::Round(parts) = attribute.node() else {
        panic!("attribute")
    };
    let altered = attribute.replace_children(vec![
        parts[0].clone(),
        parts[1].clone(),
        root("(: g (-> () Int))"),
    ]);
    assert!(altered.same_origin(&attribute));
    assert!(validate_expansion(&altered, std::slice::from_ref(&attribute)).is_err());
    validate_expansion(&attribute.flip_mark(8), &[attribute]).unwrap();
}

#[test]
fn expanded_definition_preserves_attributes_and_parameter_identities() {
    use ilang_syntax::lisp::definitions::expanded_definition;
    use ilang_syntax::lisp::syntax::{Node, ScopeIdentity};
    let item = root("(@ (deprecated \"old\") ((Box A) { (Box A) }))");
    let answer = expanded_definition(&item, 90).unwrap();
    let Node::Round(outer) = answer.node() else {
        panic!("attribute")
    };
    assert!(outer[1].same_code(&root("(deprecated \"old\")")));
    let Node::Round(core) = outer[2].node() else {
        panic!("definition")
    };
    let Node::Round(head) = core[0].node() else {
        panic!("head")
    };
    assert_eq!(core[0].definition_head(), Some(90));
    assert!(matches!(
        head[1].scope_identity(),
        Some(ScopeIdentity::DefinitionParameter { answer: 90, .. })
    ));
}

#[test]
fn deeply_nested_definition_answer_uses_bounded_host_stack() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let prototype = root("(= (Box A) A)");
            let Node::Round(parts) = prototype.node() else {
                panic!("alias");
            };
            let mut ty = parts[2].clone();
            for _ in 0..10_000 {
                ty = Syntax::make(Node::Curly(vec![ty.clone()]), &ty).unwrap();
            }
            let item = prototype.replace_children(vec![parts[0].clone(), parts[1].clone(), ty]);
            let answer = ilang_syntax::lisp::definitions::expanded_definition(&item, 77).unwrap();
            assert_eq!(answer.show(), "(= (Box A) A)");
            let wrapper = root("(@ deprecated x)");
            let Node::Round(parts) = wrapper.node() else {
                panic!("attribute");
            };
            let mut item = answer;
            for _ in 0..10_000 {
                item = wrapper.replace_children(vec![parts[0].clone(), parts[1].clone(), item]);
            }
            drop(ilang_syntax::lisp::definitions::expanded_definition(&item, 78).unwrap());
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn definition_answer_preserves_a_single_field_record_type() {
    let definition = root("(= (Box A) ((: x A)))");
    let answer = ilang_syntax::lisp::definitions::expanded_definition(&definition, 91).unwrap();
    assert_eq!(answer.show(), "(= (Box A) ((: x A)))");
}

#[test]
fn scoped_conversion_separates_repeated_origins_and_keeps_sliceable_text() {
    use ilang_syntax::ast::ExprKind;
    use ilang_syntax::lisp::scoped::ScopedSource;
    use ilang_syntax::lisp::syntax::Node;
    let template = root("(+ x x)");
    let Node::Round(parts) = template.node() else {
        panic!("template")
    };
    let original = parts[1].clone();
    let marked = original.flip_mark(41);
    let result =
        template.replace_children(vec![parts[0].clone(), original.clone(), marked.clone()]);
    let call = root("call");
    let mut source = ScopedSource::new("original source".into());
    let expr = source.expression(&result, &call).unwrap();
    let ExprKind::Binary { lhs, rhs, .. } = &expr.kind else {
        panic!("binary")
    };
    assert_ne!(lhs.span, rhs.span);
    assert_eq!(
        source
            .text()
            .get(lhs.span.start as usize..lhs.span.end as usize),
        Some("x")
    );
    assert_eq!(
        source
            .text()
            .get(rhs.span.start as usize..rhs.span.end as usize),
        Some("x")
    );
    assert_eq!(
        source.origin(lhs.span).unwrap().syntax.scope(),
        original.scope()
    );
    assert_eq!(
        source.origin(rhs.span).unwrap().syntax.scope(),
        marked.scope()
    );
    assert_eq!(
        source.origin(rhs.span).unwrap().syntax.span(),
        original.span()
    );
    assert!(source.origin(rhs.span).unwrap().call_site.same_code(&call));
    assert!(source.is_generated(lhs.span));
    assert!(source.text().starts_with("original source\n"));
    let again = source.expression(&result, &call).unwrap();
    assert_ne!(again.span, expr.span);
}

#[test]
fn scoped_conversion_reports_the_actual_scoped_bad_node() {
    use ilang_syntax::lisp::scoped::ScopedSource;
    let bad = root("(f (,))");
    let call = root("call");
    let mut source = ScopedSource::new("text".into());
    let error = source.expression(&bad, &call).unwrap_err();
    assert_eq!(error.origin.syntax.show(), "(,)");
    assert!(error.origin.call_site.same_code(&call));
}

#[test]
fn definition_errors_retain_the_exact_name_node_and_scope() {
    use ilang_syntax::lisp::definition_schedule::Mode;
    use ilang_syntax::lisp::definition_table::{DefinitionTable, Lookup, QueryResult};
    use ilang_syntax::lisp::syntax::{Node, Syntax};
    let module = root("mod");
    let name = root("Hidden").flip_mark(31);
    let dot = root(".");
    let query = Syntax::make(Node::Round(vec![dot, module, name.clone()]), &name).unwrap();
    let table: DefinitionTable<u32> = DefinitionTable::default();
    let QueryResult::Error(error) =
        table.query(&query, Mode::Normal, 1, |_, _| Lookup::ConstructorsHidden)
    else {
        panic!("expected catchable error")
    };
    assert!(error.at.identical(&name));
    assert_eq!(error.at.scope(), name.scope());
}

#[test]
fn scoped_virtual_source_uses_utf8_byte_boundaries() {
    use ilang_syntax::ast::ExprKind;
    use ilang_syntax::lisp::scoped::ScopedSource;
    let call = root("call");
    let syntax = root("(f \"你好\")");
    let mut source = ScopedSource::new("// 原文件".into());
    let expr = source.expression(&syntax, &call).unwrap();
    let ExprKind::Call { args, .. } = &expr.kind else {
        panic!("call")
    };
    let span = args[0].value.span;
    assert_eq!(
        source.text().get(span.start as usize..span.end as usize),
        Some("\"你好\"")
    );
    assert_eq!(source.origin(span).unwrap().syntax.show(), "\"你好\"");
}

#[test]
fn host_identity_is_weak_and_does_not_keep_a_prior_run_alive() {
    use std::sync::Arc;
    let syntax = root("x");
    let first = syntax.host_identity();
    let same = syntax.host_identity();
    assert!(Arc::ptr_eq(&first, &same));
    let weak = Arc::downgrade(&first);
    drop(first);
    drop(same);
    assert!(weak.upgrade().is_none());
    let next_run = syntax.host_identity();
    assert!(
        next_run
            .downcast_ref::<ilang_syntax::lisp::syntax::Syntax>()
            .unwrap()
            .identical(&syntax)
    );
}

#[test]
fn deeply_nested_syntax_marks_and_drops_without_host_stack_growth() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            use ilang_syntax::lisp::syntax::{Node, Syntax};
            let mut syntax = root("x");
            for _ in 0..20_000 {
                syntax = Syntax::make(Node::Round(vec![syntax.clone()]), &syntax).unwrap();
            }
            let datum = syntax.to_datum();
            let rebuilt = Syntax::from_datum(&datum, syntax.scope());
            assert!(syntax.same_code(&rebuilt));
            drop(datum);
            let quote = Syntax::make(Node::Quote(syntax.clone()), &syntax).unwrap();
            assert!(quote.match_quote(&rebuilt).unwrap().is_some());
            drop(quote);
            drop(rebuilt);
            let marked = syntax.flip_mark(7);
            assert!(marked.scope().marks.contains(&7));
            ilang_syntax::lisp::syntax::validate_expansion(&marked, &[]).unwrap();
            let fresh = ilang_syntax::lisp::definitions::freshen_parameters(&marked, 1, 2);
            drop(fresh);
            let answer = marked.as_answer();
            drop(answer);
            drop(marked);
            drop(syntax);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn rewriting_a_definition_head_cannot_forge_parameter_binding_authority() {
    use ilang_syntax::lisp::definitions::builtin_definition;
    use ilang_syntax::lisp::syntax::{Node, Syntax};
    let like = root("x");
    let answer = builtin_definition("Result", like.scope(), like.span(), 30).unwrap();
    let Node::Round(item) = answer.node() else {
        panic!("item")
    };
    let Node::Round(head) = item[0].node() else {
        panic!("head")
    };
    assert_eq!(item[0].definition_head(), Some(30));
    let unchanged = item[0].replace_children(head.clone());
    assert_eq!(unchanged.definition_head(), Some(30));
    let replacement = Syntax::make(Node::Up("A".into()), &head[1]).unwrap();
    let changed = item[0].replace_children(vec![head[0].clone(), replacement, head[2].clone()]);
    assert_eq!(changed.definition_head(), None);
}

#[test]
fn duplicate_definition_heads_are_freshened_with_all_uses() {
    use ilang_syntax::lisp::definitions::{builtin_definition, freshen_duplicate_heads};
    use ilang_syntax::lisp::syntax::{Node, ScopeIdentity, Syntax};
    let like = root("x");
    let definition = builtin_definition("Result", like.scope(), like.span(), 50).unwrap();
    let items = Syntax::make(
        Node::Curly(vec![definition.clone(), definition.clone()]),
        &like,
    )
    .unwrap();
    let mut next = 51;
    let output = freshen_duplicate_heads(&items, || {
        let id = next;
        next += 1;
        id
    });
    let Node::Curly(items) = output.node() else {
        panic!("items")
    };
    for (item, id) in items.iter().zip([50, 51]) {
        let Node::Round(parts) = item.node() else {
            panic!("item")
        };
        let Node::Round(head) = parts[0].node() else {
            panic!("head")
        };
        let Node::Curly(body) = parts[1].node() else {
            panic!("body")
        };
        let Node::Round(ctor) = body[0].node() else {
            panic!("ctor")
        };
        assert_eq!(parts[0].definition_head(), Some(id));
        assert!(
            matches!(head[1].scope_identity(), Some(ScopeIdentity::DefinitionParameter { answer, parameter: 0, .. }) if *answer == id)
        );
        assert_eq!(head[1].scope_identity(), ctor[1].scope_identity());
    }
}

#[test]
fn definition_table_charges_before_freshening_written_and_expanded_answers() {
    use ilang_syntax::lisp::definition_schedule::Mode;
    use ilang_syntax::lisp::definition_table::{
        Definition, DefinitionCost, DefinitionTable, Lookup,
    };
    let of = root("Box");
    let source = "Box(A) = { (A) }";
    let (module, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty());
    let mut table = DefinitionTable::default();
    table.insert(
        1,
        Definition::Written {
            item: Box::new(module.items[0].clone()),
            source: source.into(),
            scope: of.scope().clone(),
            ready: true,
        },
    );
    let expanded =
        ilang_syntax::lisp::definitions::builtin_definition("Result", of.scope(), of.span(), 9)
            .unwrap();
    table.insert(
        2,
        Definition::Expanded {
            syntax: expanded,
            parameter_answer: Some(9),
            ready: true,
        },
    );
    for key in [1, 2] {
        let mut charged = false;
        let result = table.query_metered(
            &of,
            Mode::Normal,
            10,
            |_, _| Lookup::Found(key),
            |cost| {
                assert!(matches!(
                    (key, cost),
                    (1, DefinitionCost::Event(_)) | (2, DefinitionCost::Syntax(_))
                ));
                charged = true;
                Err(Box::new(std::io::Error::other("allocation limit")))
            },
        );
        assert!(charged);
        assert!(matches!(result, Err(error) if error.to_string() == "allocation limit"));
    }
}

#[test]
fn records_parse_from_lisp_with_fields_spread_and_removal() {
    use ilang_syntax::ast::{ExprKind, TypeKind};
    let empty = ilang_syntax::lisp::parse("()").unwrap();
    let empty = ilang_syntax::lisp::parse_expression(&empty[0]).unwrap();
    // The transitional AST adapter keeps empty data as an empty tuple.
    // The source reader requires `(,)` (spec §14.3).
    assert!(matches!(&empty.kind, ExprKind::Tuple(items) if items.is_empty()));
    let values = ilang_syntax::lisp::parse("(.. r (: x 1) (: \"a b\" 2))").unwrap();
    let value = ilang_syntax::lisp::parse_expression(&values[0]).unwrap();
    assert!(
        matches!(&value.kind, ExprKind::Record { base: Some(_), fields } if fields.len() == 2 && fields[1].0.name == "a b")
    );
    let types = ilang_syntax::lisp::parse("(.. R (- old) (: K { Int }))").unwrap();
    let ty = ilang_syntax::lisp::parse_type(&types[0]).unwrap();
    assert!(
        matches!(&ty.kind, TypeKind::Record { base: Some(_), fields, removed } if fields[0].0.parameter && removed[0].ident.name == "old")
    );
}

#[test]
fn show_as_groups_a_binary_operand_under_negation() {
    assert_eq!(
        root("(- (+ x y))").show_as(Position::Expression).unwrap(),
        "-{ x + y }"
    );
}

#[test]
fn show_as_distinguishes_postfix_and_standalone_clauses() {
    assert!(
        root("{ (-> (= a (, x)) (-> (y) Int y)) }")
            .show_as(Position::Expression)
            .is_err()
    );
    assert!(
        root("(. s { (-> (= a (, x)) (-> (y) Int y)) })")
            .show_as(Position::Expression)
            .is_err()
    );
}

#[test]
fn show_as_deep_type_and_pattern_use_bounded_host_stack() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let like = root("x");
            let head = Syntax::make(Node::Up("T".into()), &like).unwrap();
            let mut tree = like.clone();
            for _ in 0..10000 {
                tree = Syntax::make(Node::Round(vec![head.clone(), tree]), &like).unwrap();
            }
            let expected = format!("{}x{}", "T(".repeat(10000), ")".repeat(10000));
            assert_eq!(tree.show_as(Position::Type).unwrap(), expected);
            assert_eq!(tree.show_as(Position::Pattern).unwrap(), expected);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn show_as_only_breaks_lines_at_ambiguous_statement_boundaries() {
    let simple = root("(-> () _ { x y })");
    assert_eq!(simple.show_as(Position::Expression).unwrap(), "() { x y }");
    let negative = root("(-> () _ { x (- 1) })");
    assert_eq!(
        negative.show_as(Position::Expression).unwrap(),
        "() { x\n-1 }"
    );
}

#[test]
fn show_as_attribute_item_uses_only_required_line_breaks() {
    let item = root("(@ custom (= f (-> () _ { Nil })))");
    assert_eq!(
        item.show_as(Position::Item).unwrap(),
        "@custom f = () { Nil }"
    );
    let (module, errors) =
        ilang_syntax::parse_module(&format!("{}\n", item.show_as(Position::Item).unwrap()));
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(module.items.len(), 1);
    assert_eq!(module.items[0].attributes.len(), 1);

    let tuple = root("(@ custom (= (, x y) (, 1 2)))");
    let text = tuple.show_as(Position::Item).unwrap();
    assert_eq!(text, "@custom\n(x, y) = (1, 2)");
    let (module, errors) = ilang_syntax::parse_module(&format!("{text}\n"));
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(module.items[0].attributes.len(), 1);
}

#[test]
fn show_as_accepts_expressions_already_written_in_normal_surface() {
    let scope = Scope {
        module: 0,
        marks: BTreeSet::new(),
    };
    let leaves = [
        "x",
        "1",
        "(1, 2)",
        "[x, y]",
        "f(x)",
        "x + y",
        "-{ x + y }",
        "(x) { x }",
        "x.{ 1 -> 2 _ -> 3 }",
        "{ x y }",
        "<<1, 2>>",
    ];
    let mut checked = 0;
    for leaf in leaves {
        for source in [
            format!("value = {leaf}\n"),
            format!("value = f({leaf})\n"),
            format!("value = -{{ {leaf} }}\n"),
            format!("value = [{leaf}]\n"),
            format!("value = {{ {leaf} }} + z\n"),
            format!("value = ({leaf}, z)\n"),
            format!("value = () {{ {leaf} }}\n"),
        ] {
            let (module, errors) = ilang_syntax::parse_module(&source);
            if !errors.is_empty() {
                continue;
            }
            let ilang_syntax::ast::ItemKind::Binding(binding) = &module.items[0].kind else {
                panic!("binding")
            };
            let syntax = expression_syntax(&binding.value, &scope, &source);
            syntax
                .show_as(Position::Expression)
                .unwrap_or_else(|error| panic!("{source:?}: {error:?}"));
            checked += 1;
        }
    }
    assert!(checked >= 50, "only {checked} source expressions parsed");
}

#[test]
fn typed_macro_type_view_has_no_source_constructor_or_text_round_trip() {
    use ilang_syntax::lisp::syntax::{Node, Position, Syntax, TypeHandle};
    let like = root("Int");
    let handle = TypeHandle {
        snapshot: 19,
        index: 3,
    };
    let view = Syntax::type_view(handle, &like);
    assert!(Syntax::make(Node::TypeView(handle), &like).is_err());
    assert_eq!(view.show(), "#<type-view>");
    assert!(view.show_as(Position::Type).is_err());
    let copied = view.flip_mark(7).flip_mark(7);
    assert!(view.same_input(&copied));
    let other = Syntax::type_view(
        TypeHandle {
            snapshot: 20,
            index: 3,
        },
        &like,
    );
    assert!(!view.same_code(&other));
    let nested = Syntax::make(Node::Round(vec![root("List"), view]), &like).unwrap();
    assert!(nested.show_as(Position::Type).is_err());
}

#[test]
fn typed_macro_type_answers_share_marked_variable_nodes_across_queries() {
    use ilang_syntax::lisp::syntax::{MarkCache, Node, ScopeIdentity, Syntax};
    let variable = root("A").with_scope_identity(ScopeIdentity::TypeVariable {
        variable: 17,
        name: "A".into(),
    });
    let mut cache = MarkCache::new(9);
    let first = cache.flip(std::slice::from_ref(&variable));
    let signature = Syntax::make(Node::Round(vec![root("List"), variable]), &root("List")).unwrap();
    let next = cache.flip(&[signature]);
    let Node::Round(xs) = next[0].node() else {
        panic!("application")
    };
    assert!(first[0].identical(&xs[1]));
}

#[test]
fn macro_parameter_labels_are_explicit_and_survive_alpha_renaming() {
    use ilang_syntax::lisp::{expansion::definitions, parse_source};
    for local in ["value", "renamed"] {
        let (module, errors) = parse_source(&format!(
            "(@ macro (= id (-> (, (= label {local})) _ {local})))"
        ));
        assert!(errors.is_empty(), "{errors:?}");
        let defs = definitions(&module, true).unwrap();
        assert_eq!(defs[0].parameters, [local]);
        assert_eq!(defs[0].labels, [Some("label".into())]);
    }
    let (module, errors) = parse_source("(@ macro (= id (-> (, value) _ value)))");
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(definitions(&module, true).unwrap()[0].labels, [None]);
}

#[test]
fn typed_macro_shapes_are_checked_even_when_a_missing_shape_is_never_called() {
    use ilang_syntax::lisp::{expansion::definitions, parse_source};
    let source = include_str!("../../../tests/spec/14.8/mp11_missing_shape/src/macros.is");
    let (module, errors) = parse_source(source);
    assert!(errors.is_empty(), "{errors:?}");
    assert!(
        definitions(&module, true)
            .unwrap_err()
            .iter()
            .any(|e| e.message.contains("every parameter-data shape"))
    );
    let source = include_str!("../../../tests/spec/14.8/mp11_shape_branches/src/macros.is");
    let (module, errors) = parse_source(source);
    assert!(errors.is_empty(), "{errors:?}");
    let definitions = definitions(&module, true).unwrap();
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].variants.len(), 2);
}
