//! Stack safety of the ordinary source parser, not just the Lisp reader.
use ilang_syntax::ast::{ExprKind, ItemKind, PatternKind, TypeKind};
use ilang_syntax::parse_module;

fn small_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn deep_calls_keep_every_argument_and_span() {
    small_stack(|| {
        let n = 10_000;
        let source = format!("value = {}x{}", "f(".repeat(n), ")".repeat(n));
        let (module, diagnostics) = parse_module(&source);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let ItemKind::Binding(binding) = &module.items[0].kind else {
            panic!("binding")
        };
        let mut node = &binding.value;
        for i in 0..n {
            assert_eq!(node.span.start as usize, 8 + 2 * i);
            let ExprKind::Call { args, .. } = &node.kind else {
                panic!("call at {i}")
            };
            assert_eq!(args.len(), 1);
            node = &args[0].value;
        }
        assert!(matches!(node.kind, ExprKind::Name { .. }));
    });
}

#[test]
fn deep_named_types_and_constructor_patterns() {
    small_stack(|| {
        let n = 10_000;
        let source = format!(
            "f = ({}x{}) -> {}Int{} {{ Nil }}",
            "C(".repeat(n),
            ")".repeat(n),
            "T(".repeat(n),
            ")".repeat(n)
        );
        let (module, diagnostics) = parse_module(&source);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let ItemKind::Binding(binding) = &module.items[0].kind else {
            panic!("binding")
        };
        let ExprKind::Fn(function) = &binding.value.kind else {
            panic!("function")
        };
        let ilang_syntax::ast::Param::Pattern(pattern) = &function.params[0] else {
            panic!("parameter")
        };
        let mut pattern = pattern;
        for i in 0..n {
            let PatternKind::Constructor { args, .. } = &pattern.kind else {
                panic!("constructor at {i}")
            };
            assert_eq!(args.len(), 1);
            pattern = &args[0];
        }
        assert!(matches!(pattern.kind, PatternKind::Var(_)));
        let mut ty = &function.ret.as_ref().unwrap().value;
        for i in 0..n {
            let TypeKind::Named { args, .. } = &ty.kind else {
                panic!("type at {i}")
            };
            assert_eq!(args.len(), 1);
            ty = &args[0];
        }
        assert!(matches!(ty.kind, TypeKind::Named { .. }));
    });
}

#[test]
fn deep_mixed_calls_and_lists() {
    small_stack(|| {
        let n = 10_000;
        let mut source = String::from("value = ");
        for i in 0..n {
            source.push_str(if i % 2 == 0 { "f(" } else { "[" });
        }
        source.push('x');
        for i in (0..n).rev() {
            source.push_str(if i % 2 == 0 { ")" } else { "]" });
        }
        let (module, diagnostics) = parse_module(&source);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let ItemKind::Binding(binding) = &module.items[0].kind else {
            panic!("binding")
        };
        let mut node = &binding.value;
        for i in 0..n {
            node = match &node.kind {
                ExprKind::Call { args, .. } if i % 2 == 0 => &args[0].value,
                ExprKind::List { items, .. } if i % 2 == 1 => &items[0],
                _ => panic!("mixed expression at {i}"),
            };
        }
        assert!(matches!(node.kind, ExprKind::Name { .. }));
    });
}

#[test]
fn deep_invalid_argument_reports_the_original_token() {
    small_stack(|| {
        let n = 10_000;
        let source = format!("value = {}_{}\nok = 1\n", "f(".repeat(n), ")".repeat(n));
        let (module, diagnostics) = parse_module(&source);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].span.start as usize, 8 + 2 * n);
        assert_eq!(module.items.len(), 1);
        let ItemKind::Binding(binding) = &module.items[0].kind else {
            panic!("recovered binding")
        };
        assert!(matches!(&binding.pattern.kind, PatternKind::Var(name) if name == "ok"));
    });
}

#[test]
fn nested_bitarray_block_parses_without_repeating_the_subtree() {
    small_stack(|| {
        let n = 10_000;
        let mut source = String::from("value = ");
        for _ in 0..n {
            source.push_str("{ <<0:u(");
        }
        source.push('8');
        for _ in 0..n {
            source.push_str(")>>\n8 }");
        }
        let (module, diagnostics) = parse_module(&source);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(module.items.len(), 1);
    });
}

#[test]
fn many_mismatched_closers_keep_diagnostics_and_do_not_rescan_openers() {
    let n = 10_000;
    let source = format!("{}{}", "(".repeat(n), "]".repeat(n));
    let lexed = ilang_syntax::lex(&source);
    assert_eq!(lexed.diagnostics.len(), 2 * n);
    assert_eq!(lexed.diagnostics[0].span.start as usize, n);
    assert_eq!(lexed.diagnostics[n - 1].span.start as usize, 2 * n - 1);
    assert_eq!(lexed.diagnostics[n].span.start, 0);
    assert_eq!(lexed.diagnostics[2 * n - 1].span.start as usize, n - 1);
}

#[test]
fn mixed_expression_pattern_type_and_probe() {
    small_stack(|| {
        let n = 10_000;
        let mut source = String::from("value = ");
        for _ in 0..n {
            source.push_str("{ <<0:u(apply((C([p: Box(Int)])) -> Int { ");
        }
        source.push('8');
        for _ in 0..n {
            source.push_str(" }))>>\n8 }");
        }
        let (module, diagnostics) = parse_module(&source);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(module.items.len(), 1);
    });
}

#[test]
fn nested_guards_restore_outer_arrow_context() {
    small_stack(|| {
        let n = 10_000;
        let mut source = String::from("value = ");
        for _ in 0..n {
            source.push_str("{ _ ? check((p: Box(Int)) { ");
        }
        source.push_str("True");
        for _ in 0..n {
            source.push_str(" }) -> True\n_ -> False }(seed)");
        }
        let (module, diagnostics) = parse_module(&source);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(module.items.len(), 1);
    });
}

#[test]
fn deeply_grouped_negative_type() {
    small_stack(|| {
        let n = 10_000;
        let source = format!("value: -{}1{} = 1", "{".repeat(n), "}".repeat(n));
        let (module, diagnostics) = parse_module(&source);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(module.items.len(), 1);
    });
}

#[test]
fn nested_bitarray_pattern_bindings_reuse_probe_results() {
    small_stack(|| {
        let n = 10_000;
        let mut source = String::from("value = ");
        for _ in 0..n {
            source.push_str("{ <<0:u(");
        }
        source.push('8');
        for _ in 0..n {
            source.push_str(")>> = 8\n8 }");
        }
        let (module, diagnostics) = parse_module(&source);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(module.items.len(), 1);
    });
}

#[test]
fn nested_bitarray_clause_patterns_reuse_probe_results() {
    small_stack(|| {
        let n = 10_000;
        let mut source = String::from("value = ");
        for _ in 0..n {
            source.push_str("{ <<0:u(");
        }
        source.push('8');
        for _ in 0..n {
            source.push_str(")>> -> 8\n_ -> 8 }(seed)");
        }
        let (module, diagnostics) = parse_module(&source);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(module.items.len(), 1);
    });
}

#[test]
fn nested_bitarray_pipelines_do_not_repeat_the_probe() {
    small_stack(|| {
        let n = 10_000;
        let mut source = String::from("value = ");
        for _ in 0..n {
            source.push_str("{ <<0:u(");
        }
        source.push('8');
        for _ in 0..n {
            source.push_str(")>> | f\n8 }");
        }
        let (module, diagnostics) = parse_module(&source);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(module.items.len(), 1);
    });
}

#[test]
fn deep_bad_pattern_recovers_the_next_clause_and_item() {
    small_stack(|| {
        let n = 10_000;
        let source = format!(
            "value = {{ {}-x{} -> 1\n_ -> 2 }}\nok = 3\n",
            "C(".repeat(n),
            ")".repeat(n)
        );
        let (module, diagnostics) = parse_module(&source);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].code, ilang_syntax::Code::BadPattern);
        assert_eq!(diagnostics[0].span.start as usize, 10 + 2 * n + 1);
        assert_eq!(module.items.len(), 2);
        let ItemKind::Binding(binding) = &module.items[0].kind else {
            panic!("binding")
        };
        let ExprKind::Clauses(clauses) = &binding.value.kind else {
            panic!("clauses")
        };
        assert_eq!(clauses.len(), 1);
        assert!(matches!(
            clauses[0].patterns[0].kind,
            PatternKind::Discard(_)
        ));
    });
}

#[test]
fn deep_bad_type_recovers_the_next_item() {
    small_stack(|| {
        let n = 10_000;
        let source = format!(
            "value: {}1.0{} = 1\nok = 2\n",
            "T(".repeat(n),
            ")".repeat(n)
        );
        let (module, diagnostics) = parse_module(&source);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].span.start as usize, 7 + 2 * n);
        assert_eq!(module.items.len(), 1);
        let ItemKind::Binding(binding) = &module.items[0].kind else {
            panic!("binding")
        };
        assert!(matches!(&binding.pattern.kind, PatternKind::Var(name) if name == "ok"));
    });
}

#[test]
fn deep_bad_guard_restores_context_for_the_following_clause() {
    small_stack(|| {
        let n = 10_000;
        let source = format!(
            "value = {{ _ ? {}_{} -> 1\n_ -> 2 }}\nok = 3\n",
            "f(".repeat(n),
            ")".repeat(n)
        );
        let (module, diagnostics) = parse_module(&source);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].span.start as usize, 14 + 2 * n);
        assert_eq!(module.items.len(), 2);
        let ItemKind::Binding(binding) = &module.items[0].kind else {
            panic!("binding")
        };
        let ExprKind::Clauses(clauses) = &binding.value.kind else {
            panic!("clauses")
        };
        assert_eq!(clauses.len(), 1);
        assert!(matches!(
            clauses[0].patterns[0].kind,
            PatternKind::Discard(_)
        ));
    });
}

#[test]
fn nested_probe_diagnostics_are_shared_and_not_duplicated() {
    small_stack(|| {
        let n = 10_000;
        let mut source = String::from("value = ");
        for _ in 0..n {
            source.push_str("{ <<0:u(f(label: 1, 2, ");
        }
        source.push('8');
        for _ in 0..n {
            source.push_str("))>> = 8\n8 }");
        }
        let (module, diagnostics) = parse_module(&source);
        assert_eq!(module.items.len(), 1);
        assert_eq!(diagnostics.len(), 2 * n);
        assert!(
            diagnostics
                .iter()
                .all(|d| d.code == ilang_syntax::Code::ArgOrder)
        );
        assert!(
            diagnostics
                .windows(2)
                .all(|pair| pair[0].span.start <= pair[1].span.start)
        );
    });
}

#[test]
fn deep_alternative_patterns_reuse_probes() {
    small_stack(|| {
        let n = 10_000;
        let source = format!(
            "value = {}8{}",
            "{ <<>> | <<0:u(".repeat(n),
            ")>> -> 8\n_ -> 8 }(<<0>>)".repeat(n)
        );
        let (_, diagnostics) = parse_module(&source);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    });
}

#[test]
fn deep_pipe_followed_by_function_binding() {
    small_stack(|| {
        let n = 10_000;
        let source = format!(
            "value = {}8{}",
            "{ <<0:u(".repeat(n),
            ")>> | f\ng = (x) -> Int { x }\n8 }".repeat(n)
        );
        let (_, diagnostics) = parse_module(&source);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    });
}

#[test]
fn deep_record_fields_and_spreads_parse_and_drop() {
    small_stack(|| {
        let n = 10_000;
        for (prefix, suffix) in [("(field: ", ")"), ("(..", ")"), ("C(..", ")")] {
            let source = format!("value = {}x{}", prefix.repeat(n), suffix.repeat(n));
            let (module, diagnostics) = parse_module(&source);
            assert!(diagnostics.is_empty(), "{diagnostics:?}");
            let ItemKind::Binding(binding) = &module.items[0].kind else {
                panic!("binding")
            };
            let mut node = &binding.value;
            for _ in 0..n {
                node = match &node.kind {
                    ExprKind::Record {
                        base: Some(base), ..
                    } => base,
                    ExprKind::Record { base: None, fields } => &fields[0].1,
                    ExprKind::RecordUpdate { base, .. } => base,
                    _ => panic!("expected nested record"),
                };
            }
            assert!(matches!(node.kind, ExprKind::Name { .. }));
        }
    });
}

#[test]
fn deep_record_type_fields_and_spreads_parse_and_drop() {
    small_stack(|| {
        let n = 10_000;
        for prefix in ["(field: ", "(.."] {
            let source = format!(
                "f = (x: {}Record{}) {{ x }}",
                prefix.repeat(n),
                ")".repeat(n)
            );
            let (module, diagnostics) = parse_module(&source);
            assert!(diagnostics.is_empty(), "{diagnostics:?}");
            let ItemKind::Binding(binding) = &module.items[0].kind else {
                panic!("binding")
            };
            let ExprKind::Fn(function) = &binding.value.kind else {
                panic!("function")
            };
            let ilang_syntax::ast::Param::Pattern(pattern) = &function.params[0] else {
                panic!("parameter")
            };
            let PatternKind::Typed { ty, .. } = &pattern.kind else {
                panic!("typed parameter")
            };
            let mut node = ty;
            for _ in 0..n {
                node = match &node.kind {
                    TypeKind::Record {
                        base: Some(base), ..
                    } => base,
                    TypeKind::Record {
                        base: None, fields, ..
                    } => &fields[0].1,
                    _ => panic!("expected nested record type"),
                };
            }
            assert!(matches!(node.kind, TypeKind::Named { .. }));
        }
    });
}
