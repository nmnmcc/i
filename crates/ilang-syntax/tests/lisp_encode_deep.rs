use std::collections::BTreeSet;

use ilang_syntax::ast::{
    Arg, Block, Expr, ExprKind, Function, Ident, Param, Pattern, PatternKind, RetType, Stmt,
    TypeExpr, TypeKind,
};
use ilang_syntax::lisp::syntax::{Node, Scope};
use ilang_syntax::lisp::{expression_syntax, pattern_syntax, type_syntax};
use ilang_syntax::span::Span;

fn ident(name: &str, start: usize) -> Ident {
    Ident {
        name: name.into(),
        span: Span::new(start as u32, (start + name.len()) as u32),
    }
}
fn scope() -> Scope {
    Scope {
        module: 17,
        marks: BTreeSet::from([3, 9]),
    }
}

#[test]
fn deep_calls_types_and_patterns_preserve_spans_and_scope_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let depth = 10_000;
            let source = format!("{}x{}", "f(".repeat(depth), ")".repeat(depth));
            let end = source.len();
            let mut expr = Expr {
                kind: ExprKind::Name {
                    module: None,
                    name: ident("x", depth * 2),
                },
                span: Span::new((depth * 2) as u32, (depth * 2 + 1) as u32),
            };
            for i in (0..depth).rev() {
                let f = ident("f", i * 2);
                expr = Expr {
                    span: Span::new((i * 2) as u32, (end - i) as u32),
                    kind: ExprKind::Call {
                        callee: Box::new(Expr {
                            span: f.span,
                            kind: ExprKind::Name {
                                module: None,
                                name: f,
                            },
                        }),
                        args: vec![Arg {
                            label: None,
                            value: expr,
                        }],
                    },
                };
            }
            let scope = scope();
            let encoded = expression_syntax(&expr, &scope, &source);
            let mut current = &encoded;
            for i in 0..depth {
                assert_eq!(current.scope(), &scope);
                assert_eq!(current.span(), Span::new((i * 2) as u32, (end - i) as u32));
                let Node::Round(parts) = current.node() else {
                    panic!("expected call")
                };
                assert_eq!(parts.len(), 2);
                assert!(matches!(parts[0].node(), Node::Name(name) if name == "f"));
                assert_eq!(
                    parts[0].span(),
                    Span::new((i * 2) as u32, (i * 2 + 1) as u32)
                );
                current = &parts[1];
            }
            assert!(matches!(current.node(), Node::Name(name) if name == "x"));
            drop(encoded);
            drop(expr);

            let source = format!("{}Int{}", "T(".repeat(depth), ")".repeat(depth));
            let end = source.len();
            let pos = depth * 2;
            let mut ty = TypeExpr {
                span: Span::new(pos as u32, (pos + 3) as u32),
                kind: TypeKind::Named {
                    module: None,
                    name: ident("Int", pos),
                    args: vec![],
                },
            };
            for i in (0..depth).rev() {
                ty = TypeExpr {
                    span: Span::new((i * 2) as u32, (end - i) as u32),
                    kind: TypeKind::Named {
                        module: None,
                        name: ident("T", i * 2),
                        args: vec![ty],
                    },
                };
            }
            let encoded = type_syntax(&ty, &scope, &source);
            let mut current = &encoded;
            for i in 0..depth {
                assert_eq!(current.span(), Span::new((i * 2) as u32, (end - i) as u32));
                let Node::Round(parts) = current.node() else {
                    panic!("expected type call")
                };
                assert!(matches!(parts[0].node(), Node::Up(name) if name == "T"));
                assert_eq!(parts[0].scope(), &scope);
                current = &parts[1];
            }
            assert!(matches!(current.node(), Node::Up(name) if name == "Int"));
            drop(encoded);
            drop(ty);

            let source = format!("{}x{}", "C(".repeat(depth), ")".repeat(depth));
            let end = source.len();
            let pos = depth * 2;
            let mut pattern = Pattern {
                span: Span::new(pos as u32, (pos + 1) as u32),
                kind: PatternKind::Var("x".into()),
            };
            for i in (0..depth).rev() {
                pattern = Pattern {
                    span: Span::new((i * 2) as u32, (end - i) as u32),
                    kind: PatternKind::Constructor {
                        module: None,
                        name: ident("C", i * 2),
                        args: vec![pattern],
                        spread: false,
                    },
                };
            }
            let encoded = pattern_syntax(&pattern, &scope, &source);
            let mut current = &encoded;
            for i in 0..depth {
                assert_eq!(current.span(), Span::new((i * 2) as u32, (end - i) as u32));
                let Node::Round(parts) = current.node() else {
                    panic!("expected constructor")
                };
                assert!(matches!(parts[0].node(), Node::Up(name) if name == "C"));
                current = &parts[1];
            }
            assert!(matches!(current.node(), Node::Name(name) if name == "x"));
            drop(encoded);
            drop(pattern);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn deeply_nested_functions_visit_patterns_types_and_blocks_without_recursion() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let depth = 10_000;
            let prefix = "(x: T) -> T {";
            let source = format!("{}x{}", prefix.repeat(depth), "}".repeat(depth));
            let end = source.len();
            let pos = prefix.len() * depth;
            let mut expr = Expr {
                span: Span::new(pos as u32, (pos + 1) as u32),
                kind: ExprKind::Name {
                    module: None,
                    name: ident("x", pos),
                },
            };
            for i in (0..depth).rev() {
                let start = prefix.len() * i;
                let binder_pos = start + 1;
                let ty_pos = start + 4;
                let ret_pos = start + 10;
                let body_pos = start + 12;
                let ty = TypeExpr {
                    span: Span::new(ty_pos as u32, (ty_pos + 1) as u32),
                    kind: TypeKind::Named {
                        module: None,
                        name: ident("T", ty_pos),
                        args: vec![],
                    },
                };
                let pattern = Pattern {
                    span: Span::new(binder_pos as u32, (ty_pos + 1) as u32),
                    kind: PatternKind::Typed {
                        binder: ilang_syntax::ast::Binder {
                            name: "x".into(),
                            discard: false,
                            span: Span::new(binder_pos as u32, (binder_pos + 1) as u32),
                        },
                        ty,
                    },
                };
                let ret = TypeExpr {
                    span: Span::new(ret_pos as u32, (ret_pos + 1) as u32),
                    kind: TypeKind::Named {
                        module: None,
                        name: ident("T", ret_pos),
                        args: vec![],
                    },
                };
                expr = Expr {
                    span: Span::new(start as u32, (end - i) as u32),
                    kind: ExprKind::Fn(Box::new(Function {
                        params: vec![Param::Pattern(pattern)],
                        ret: Some(RetType {
                            value: ret,
                            err: None,
                            env: None,
                        }),
                        body: Block {
                            stmts: vec![Stmt::Expr(expr)],
                            span: Span::new(body_pos as u32, (end - i) as u32),
                        },
                    })),
                };
            }
            let scope = scope();
            let encoded = expression_syntax(&expr, &scope, &source);
            let mut current = &encoded;
            for i in 0..depth {
                let Node::Round(parts) = current.node() else {
                    panic!("expected function")
                };
                assert!(matches!(parts[0].node(), Node::Symbol(s) if s == "->"));
                let Node::Round(params) = parts[1].node() else {
                    panic!("expected params")
                };
                let Node::Round(typed) = params[0].node() else {
                    panic!("expected annotation")
                };
                assert!(matches!(typed[0].node(), Node::Symbol(s) if s == ":"));
                assert_eq!(typed[1].scope(), &scope);
                assert_eq!(typed[1].span().start, (i * prefix.len() + 1) as u32);
                let Node::Curly(body) = parts[3].node() else {
                    panic!("expected body")
                };
                current = &body[0];
            }
            assert!(matches!(current.node(), Node::Name(s) if s == "x"));
            drop(encoded);
            drop(expr);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn reusable_encoder_preserves_source_spans_and_scope_across_items() {
    use ilang_syntax::lisp::{SourceEncoder, item_syntax};
    let source = "first = (x: Int) -> Int { x + 1 }\nsecond = (y: String) { y }\n";
    let (module, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let scope = scope();
    let encoder = SourceEncoder::new(&scope, source);
    for item in &module.items {
        let reused = encoder.item(item);
        let separate = item_syntax(item, &scope, source);
        assert!(reused.same_code(&separate));
        assert_eq!(reused.span(), separate.span());
        assert_eq!(reused.scope(), &scope);
    }
    // Re-encoding an earlier node must not return a stale cached result.
    let first = encoder.item(&module.items[0]);
    assert!(first.same_code(&item_syntax(&module.items[0], &scope, source)));
    assert_eq!(first.span(), module.items[0].span);
}

// Source AST stores record label text but does not retain whether the source
// used quotes. The independent printer may choose the other spelling.
fn same_code_with_record_label_spelling(
    a: &ilang_syntax::lisp::syntax::Syntax,
    b: &ilang_syntax::lisp::syntax::Syntax,
) -> bool {
    let mut work = vec![(a, b, false)];
    while let Some((a, b, record_field)) = work.pop() {
        if a.same_code(b) {
            continue;
        }
        match (a.node(), b.node()) {
            (Node::Curly(a), Node::Curly(b)) if a.len() == b.len() => {
                work.extend(a.iter().zip(b).map(|(a, b)| (a, b, true)));
            }
            (Node::Round(a), Node::Round(b)) if a.len() == b.len() => {
                let is_label = record_field
                    && matches!(a.first().map(|x| x.node()), Some(Node::Symbol(x)) if x == ":" || x == "-");
                for (i, (a, b)) in a.iter().zip(b).enumerate() {
                    if is_label
                        && i == 1
                        && let (Node::Name(x), Node::Text(y)) | (Node::Text(x), Node::Name(y)) =
                            (a.node(), b.node())
                        && x == y
                    {
                        continue;
                    }
                    work.push((a, b, false));
                }
            }
            (Node::Square(a), Node::Square(b)) | (Node::Bits(a), Node::Bits(b))
                if a.len() == b.len() =>
            {
                work.extend(a.iter().zip(b).map(|(a, b)| (a, b, false)));
            }
            (Node::Quote(a), Node::Quote(b))
            | (Node::Unquote(a), Node::Unquote(b))
            | (Node::Splice(a), Node::Splice(b)) => work.push((a, b, false)),
            _ => return false,
        }
    }
    true
}

#[test]
fn reusable_encoder_agrees_with_independent_lisp_printer_on_valid_corpus() {
    use ilang_syntax::lisp::SourceEncoder;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut dirs = vec![root.join("lib"), root.join("std"), root.join("tests/spec")];
    let scope = scope();
    let mut checked = 0;
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
            let (module, diagnostics) = ilang_syntax::parse_module(&source);
            if !diagnostics.is_empty() {
                // The spec corpus includes deliberately invalid source files.
                continue;
            }
            let encoder = SourceEncoder::new(&scope, &source);
            for item in &module.items {
                let mut clean = item.clone();
                clean.doc.clear();
                let expected = ilang_syntax::ast::Module {
                    items: vec![clean],
                    ..Default::default()
                };
                let plan = planned(&encoder, item);
                let encoded = encoder.item(item);
                assert_eq!(plan, direct_charge_events(&encoded), "{}", path.display());
                let printed = ilang_syntax::lisp::print_module(&expected);
                let parsed = ilang_syntax::lisp::parse(&printed).unwrap();
                assert_eq!(parsed.len(), 1, "{}", path.display());
                let expected_syntax =
                    ilang_syntax::lisp::syntax::Syntax::from_datum(&parsed[0], &scope);
                assert!(
                    same_code_with_record_label_spelling(&encoded, &expected_syntax),
                    "{}: {} != {}",
                    path.display(),
                    encoded.show(),
                    printed
                );
                checked += 1;
            }
        }
    }
    assert!(
        checked > 100,
        "expected substantive valid corpus, saw {checked} items"
    );
}

#[test]
fn written_macro_argument_preserves_identity_scope_and_locations_through_quote() {
    use ilang_syntax::lisp::parse;
    use ilang_syntax::lisp::syntax::{Substitution, Syntax};
    let source = "main = () { foo(1) }\n";
    let (module, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let ilang_syntax::ast::ItemKind::Binding(binding) = &module.items[0].kind else {
        panic!("expected binding")
    };
    let ExprKind::Fn(function) = &binding.value.kind else {
        panic!("expected function")
    };
    let Stmt::Expr(call) = &function.body.stmts[0] else {
        panic!("expected call")
    };
    let caller_scope = scope();
    let arg = expression_syntax(call, &caller_scope, source);
    assert_eq!(arg.show(), "(foo 1)");
    let template_scope = Scope {
        module: 24,
        marks: BTreeSet::from([11]),
    };
    let template = Syntax::from_datum(&parse("`(wrap ~x)").unwrap()[0], &template_scope);
    let result = template
        .instantiate(&mut |_| Ok(Substitution::One(arg.clone())))
        .unwrap();
    let Node::Round(parts) = result.node() else {
        panic!("expected expanded call")
    };
    assert!(parts[1].identical(&arg));
    assert_eq!(parts[1].scope(), &caller_scope);
    assert_eq!(
        &source[parts[1].span().start as usize..parts[1].span().end as usize],
        "foo(1)"
    );
    let Node::Round(call_parts) = parts[1].node() else {
        panic!("expected original call")
    };
    assert_eq!(
        &source[call_parts[0].span().start as usize..call_parts[0].span().end as usize],
        "foo"
    );
    assert_eq!(
        &source[call_parts[1].span().start as usize..call_parts[1].span().end as usize],
        "1"
    );
}

fn planned(
    encoder: &ilang_syntax::lisp::SourceEncoder,
    item: &ilang_syntax::ast::Item,
) -> Vec<ilang_syntax::lisp::SyntaxCharge> {
    let mut events = Vec::new();
    encoder
        .visit_item_charge(item, |event| {
            events.push(event);
            Ok::<_, ()>(())
        })
        .unwrap();
    events
}

// Independent oracle for the host's output=true Syntax traversal: each
// event is checked individually, including zero-cost integer operations.
fn direct_charge_events(
    root: &ilang_syntax::lisp::syntax::Syntax,
) -> Vec<ilang_syntax::lisp::SyntaxCharge> {
    use ilang_syntax::lisp::SyntaxCharge as C;
    enum Task<'a> {
        Node(&'a ilang_syntax::lisp::syntax::Syntax),
        Leave,
    }
    let mut pending = vec![Task::Node(root)];
    let mut events = Vec::new();
    while let Some(task) = pending.pop() {
        let s = match task {
            Task::Leave => {
                events.push(C::LeaveFrame);
                continue;
            }
            Task::Node(s) => s,
        };
        events.push(C::EnterFrame(1));
        events.push(C::Charge { work: 1, words: 80 });
        pending.push(Task::Leave);
        let bytes = match s.node() {
            Node::Name(text)
            | Node::Discard(text)
            | Node::Up(text)
            | Node::Symbol(text)
            | Node::Text(text) => text.len() as u64,
            Node::Int(value) => {
                let bytes = value.bits().div_ceil(8);
                let limbs = bytes.div_ceil(4);
                events.push(C::Charge {
                    work: limbs.saturating_mul(limbs),
                    words: 0,
                });
                bytes
            }
            Node::Float(_) => 8,
            Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                let n = xs.len() as u64;
                events.push(C::Charge {
                    work: n,
                    words: 4 + n.saturating_mul(8),
                });
                pending.extend(xs.iter().rev().map(Task::Node));
                0
            }
            Node::Quote(child) | Node::Unquote(child) | Node::Splice(child) => {
                events.push(C::Charge { work: 1, words: 8 });
                pending.push(Task::Node(child));
                0
            }
        };
        let work = bytes.saturating_mul(12);
        events.push(C::Charge {
            work,
            words: 4 + work.div_ceil(8),
        });
    }
    events
}

#[test]
fn charge_plan_matches_each_event_without_building_the_answer() {
    use ilang_syntax::lisp::SourceEncoder;
    let source = "Item(A) = (label = A, number = 123456789012345678901234567890)\nThing(A) = Item(A)\n";
    let (module, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let scope = scope();
    let encoder = SourceEncoder::new(&scope, source);
    let plan = planned(&encoder, &module.items[0]);
    let encoded = encoder.item(&module.items[0]);
    assert_eq!(plan, direct_charge_events(&encoded));
    let mut prefix = Vec::new();
    let stopped = encoder.visit_item_charge(&module.items[0], |event| {
        prefix.push(event);
        if prefix.len() == 7 {
            Err("limit")
        } else {
            Ok(())
        }
    });
    assert_eq!(stopped, Err("limit"));
    assert_eq!(prefix, plan[..7]);
    fn send_sync<T: Send + Sync>() {}
    send_sync::<SourceEncoder>();
    // Neither lexical context nor an unrelated item may affect the selected
    // definition's plan, including its event order.
    let extended = format!("{source}unrelated = \"{}\"\n", "x".repeat(50_000));
    let extended_encoder = SourceEncoder::new(&scope, &extended);
    assert_eq!(plan, planned(&extended_encoder, &module.items[0]));
}

#[test]
fn deep_item_charge_plan_is_stack_safe_and_has_exact_order() {
    use ilang_syntax::ast::{Binding, Item, ItemKind};
    use ilang_syntax::lisp::SourceEncoder;
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let depth = 10_000;
            let prefix = "main = ";
            let source = format!("{prefix}{}x{}", "f(".repeat(depth), ")".repeat(depth));
            let end = source.len();
            let pos = prefix.len() + depth * 2;
            let mut expr = Expr {
                span: Span::new(pos as u32, (pos + 1) as u32),
                kind: ExprKind::Name {
                    module: None,
                    name: ident("x", pos),
                },
            };
            for i in (0..depth).rev() {
                let start = prefix.len() + i * 2;
                let f = ident("f", start);
                expr = Expr {
                    span: Span::new(start as u32, (end - i) as u32),
                    kind: ExprKind::Call {
                        callee: Box::new(Expr {
                            span: f.span,
                            kind: ExprKind::Name {
                                module: None,
                                name: f,
                            },
                        }),
                        args: vec![Arg {
                            label: None,
                            value: expr,
                        }],
                    },
                };
            }
            let pattern = Pattern {
                span: Span::new(0, 4),
                kind: PatternKind::Var("main".into()),
            };
            let item = Item {
                doc: vec![],
                attributes: vec![],
                span: Span::new(0, end as u32),
                kind: ItemKind::Binding(Binding {
                    pattern,
                    value: expr,
                    span: Span::new(0, end as u32),
                    type_params: vec![],
                }),
            };
            let scope = scope();
            let encoder = SourceEncoder::new(&scope, &source);
            let plan = planned(&encoder, &item);
            let result = encoder.item(&item);
            assert_eq!(plan, direct_charge_events(&result));
            drop(result);
            drop(item);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn charge_events_count_unicode_zero_and_large_integer_exactly() {
    use ilang_syntax::lisp::SourceEncoder;
    let source = "value = () { [\"é\\n\", 0, 1234567890123456789012345678901234567890] }\n";
    let (module, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let scope = scope();
    let encoder = SourceEncoder::new(&scope, source);
    let item = &module.items[0];
    let plan = planned(&encoder, item);
    assert_eq!(plan, direct_charge_events(&encoder.item(item)));
    assert!(plan.iter().any(|event| matches!(
        event,
        ilang_syntax::lisp::SyntaxCharge::Charge { work: 0, words: 0 }
    )));
}

#[test]
fn records_preserve_labels_spreads_and_streamed_cost() {
    use ilang_syntax::lisp::SourceEncoder;
    let source = "f = <R, K>(r: (k = Int, full_name = String)) { (..r, full_name = \"Ada\", x = 1) }\nempty = ()\n";
    let (module, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let scope = scope();
    let encoder = SourceEncoder::new(&scope, source);
    for item in &module.items {
        let syntax = encoder.item(item);
        assert_eq!(planned(&encoder, item), direct_charge_events(&syntax));
        let mut nodes = vec![&syntax];
        while let Some(node) = nodes.pop() {
            assert_eq!(node.scope(), &scope);
            match node.node() {
                Node::Round(children) | Node::Curly(children) | Node::Square(children) => {
                    nodes.extend(children)
                }
                Node::Symbol(symbol) if matches!(symbol.as_str(), ":" | ".." | "-") => {
                    let span = node.span();
                    assert_eq!(&source[span.start as usize..span.end as usize], symbol);
                }
                Node::Text(label) if label == "full name" => {
                    let span = node.span();
                    assert_eq!(
                        &source[span.start as usize..span.end as usize],
                        "\"full name\""
                    );
                }
                _ => {}
            }
        }
        let shown = syntax.show();
        if shown.starts_with("(= (f ") {
            assert!(shown.contains("(- old)"), "{shown}");
            assert!(shown.contains("(: K Int)"), "{shown}");
            assert!(shown.contains("(- old) (: K Int) (- other)"), "{shown}");
            assert!(shown.contains("(: \"full name\" String)"), "{shown}");
            assert!(shown.contains("(: \"full name\" \"Ada\")"), "{shown}");
        } else {
            assert_eq!(shown, "(= empty ())");
        }
    }
}

#[test]
fn charge_events_for_interpolation_and_two_sided_string_patterns() {
    use ilang_syntax::lisp::SourceEncoder;
    let source = r##"value = (s: String) {
	s |> ("a" <> r <> "b") { "x\(r)" } | (r <> "b") { r } | (_) { "" }
}"##;
    let (module, errors) = ilang_syntax::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let scope = scope();
    let encoder = SourceEncoder::new(&scope, source);
    let item = &module.items[0];
    assert_eq!(
        planned(&encoder, item),
        direct_charge_events(&encoder.item(item))
    );
}
