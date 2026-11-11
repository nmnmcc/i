use ilang_macro::body_walk::{self, Hook, Local};
use ilang_syntax::{
    ast::{Expr, ExprKind, ItemKind, Stmt},
    span::Span,
};

fn expression(source: &str) -> Expr {
    let (module, diags) = ilang_syntax::parse_module(&format!("f = () {{ {source} }}"));
    assert!(diags.is_empty(), "{diags:?}");
    let ItemKind::Binding(binding) = &module.items[0].kind else {
        panic!()
    };
    let ExprKind::Fn(function) = &binding.value.kind else {
        panic!()
    };
    let Stmt::Expr(value) = &function.body.stmts[0] else {
        panic!()
    };
    value.clone()
}

#[derive(Default)]
struct TestHook {
    calls: Vec<String>,
    loop_forever: bool,
}
impl Hook for TestHook {
    type Error = &'static str;
    fn is_macro(&mut self, e: &Expr, locals: &[Local], depth: u32) -> Result<bool, Self::Error> {
        let ExprKind::Call { callee, .. } = &e.kind else {
            return Ok(false);
        };
        let ExprKind::Name { module: None, name } = &callee.kind else {
            return Ok(false);
        };
        if locals
            .iter()
            .rev()
            .any(|b| b.name == name.name && b.fn_only.is_none_or(|d| depth > d))
        {
            return Ok(false);
        }
        Ok(matches!(name.name.as_str(), "outer" | "inner" | "m"))
    }
    fn expand(&mut self, e: &Expr, _: &[Local], _: u32) -> Result<Expr, Self::Error> {
        let ExprKind::Call { callee, .. } = &e.kind else {
            unreachable!()
        };
        let ExprKind::Name { name, .. } = &callee.kind else {
            unreachable!()
        };
        self.calls.push(name.name.clone());
        if self.loop_forever {
            Ok(e.clone())
        } else if name.name == "outer" {
            Ok(expression("inner(1)"))
        } else {
            Ok(expression("0"))
        }
    }
    fn is_type_macro(
        &mut self,
        _: &ilang_syntax::ast::TypeExpr,
        _: &[Local],
        _: u32,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn expand_type(
        &mut self,
        _: &ilang_syntax::ast::TypeExpr,
        _: &[Local],
        _: u32,
    ) -> Result<ilang_syntax::ast::TypeExpr, Self::Error> {
        unreachable!()
    }
    fn depth_error(&mut self, _: Span) -> Self::Error {
        "depth"
    }
    fn is_direct(&mut self, e: &Expr, _: &[Local], _: u32) -> bool {
        matches!(e.kind, ExprKind::Fn(_) | ExprKind::Clauses(_))
    }
}

#[test]
fn outer_call_sees_unexpanded_arguments_and_result_is_expanded() {
    let (mut module, errors) = ilang_syntax::parse_module("f = () { outer(inner()) }");
    assert!(errors.is_empty());
    let mut hook = TestHook::default();
    body_walk::expressions(&mut module, &mut hook).unwrap();
    assert_eq!(hook.calls, ["outer", "inner"]);
}

#[test]
fn local_parameter_and_earlier_binding_shadow_imported_macro() {
    let (mut module, errors) = ilang_syntax::parse_module(
        "a = (m) { m(1) }\nb = () { m = () { Nil } m(2) }\nc = () { m(3) }",
    );
    assert!(errors.is_empty(), "{errors:?}");
    let mut hook = TestHook::default();
    body_walk::expressions(&mut module, &mut hook).unwrap();
    assert_eq!(hook.calls, ["m"]);
}

#[test]
fn nested_expansions_stop_before_running_the_257th_call() {
    let (mut module, errors) = ilang_syntax::parse_module("f = () { m() }");
    assert!(errors.is_empty());
    let mut hook = TestHook {
        loop_forever: true,
        ..Default::default()
    };
    assert_eq!(body_walk::expressions(&mut module, &mut hook), Err("depth"));
    assert_eq!(hook.calls.len(), 256);
}

#[test]
fn direct_recursive_bindings_shadow_macros_only_in_function_bodies() {
    let (mut module, errors) =
        ilang_syntax::parse_module("a = () { m = () { m() } m() }\nb = () { m = m() m() }");
    assert!(errors.is_empty(), "{errors:?}");
    let mut hook = TestHook::default();
    body_walk::expressions(&mut module, &mut hook).unwrap();
    // The only macro is the non-direct binding's right-hand side; its own
    // binder does not cover an earlier definition until after that value.
    assert_eq!(hook.calls, ["m"]);
}

#[test]
fn type_macros_are_expanded_outside_in_in_body_annotations() {
    use ilang_syntax::ast::{TypeExpr, TypeKind};
    #[derive(Default)]
    struct Types(Vec<String>);
    impl Hook for Types {
        type Error = &'static str;
        fn is_macro(&mut self, _: &Expr, _: &[Local], _: u32) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn expand(&mut self, _: &Expr, _: &[Local], _: u32) -> Result<Expr, Self::Error> {
            unreachable!()
        }
        fn is_type_macro(
            &mut self,
            t: &TypeExpr,
            _: &[Local],
            _: u32,
        ) -> Result<bool, Self::Error> {
            Ok(
                matches!(&t.kind, TypeKind::Call { name, .. } if name.name == "outer" || name.name == "inner"),
            )
        }
        fn expand_type(
            &mut self,
            t: &TypeExpr,
            _: &[Local],
            _: u32,
        ) -> Result<TypeExpr, Self::Error> {
            let TypeKind::Call { name, .. } = &t.kind else {
                unreachable!()
            };
            self.0.push(name.name.clone());
            let source = if name.name == "outer" {
                "inner(Int)"
            } else {
                "Int"
            };
            let (module, errors) = ilang_syntax::parse_module(&format!("x: {source}"));
            assert!(errors.is_empty(), "{errors:?}");
            let ItemKind::Declaration { ty, .. } = &module.items[0].kind else {
                panic!()
            };
            Ok(ty.clone())
        }
        fn depth_error(&mut self, _: Span) -> Self::Error {
            "depth"
        }
        fn is_direct(&mut self, _: &Expr, _: &[Local], _: u32) -> bool {
            false
        }
    }
    let (mut module, errors) = ilang_syntax::parse_module("x: outer(inner(Int))");
    assert!(errors.is_empty(), "{errors:?}");
    let mut hook = Types::default();
    body_walk::expressions(&mut module, &mut hook).unwrap();
    assert_eq!(hook.0, ["outer", "inner"]);
}

#[test]
fn deep_mixed_bodies_patterns_and_types_do_not_use_the_host_stack() {
    use ilang_syntax::ast::{Block, Function, Pattern, PatternKind, TypeExpr, TypeKind};
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let (mut module, errors) = ilang_syntax::parse_module("f = () { Nil }\nx: Int");
            assert!(errors.is_empty(), "{errors:?}");
            let span = Span::default();
            let mut value = expression("m()");
            for index in 0..10_000 {
                let kind = match index % 4 {
                    0 => ExprKind::Tuple(vec![value]),
                    1 => ExprKind::Block(Block {
                        stmts: vec![Stmt::Expr(value)],
                        span,
                    }),
                    2 => ExprKind::Fn(Box::new(Function {
                        params: vec![],
                        ret: None,
                        body: Block {
                            stmts: vec![Stmt::Expr(value)],
                            span,
                        },
                    })),
                    _ => ExprKind::Call {
                        callee: Box::new(value),
                        args: vec![],
                    },
                };
                value = Expr { kind, span };
            }
            let ItemKind::Binding(binding) = &mut module.items[0].kind else {
                panic!()
            };
            let ExprKind::Fn(function) = &mut binding.value.kind else {
                panic!()
            };
            function.body.stmts[0] = Stmt::Expr(value);
            let mut pattern = Pattern {
                kind: PatternKind::Discard("_".into()),
                span,
            };
            for _ in 0..10_000 {
                pattern = Pattern {
                    kind: PatternKind::Tuple(vec![pattern]),
                    span,
                };
            }
            binding.pattern = pattern;
            let ItemKind::Declaration { ty, .. } = &mut module.items[1].kind else {
                panic!()
            };
            let mut nested = TypeExpr {
                kind: TypeKind::Hole,
                span,
            };
            for _ in 0..10_000 {
                nested = TypeExpr {
                    kind: TypeKind::Tuple(vec![nested]),
                    span,
                };
            }
            *ty = nested;
            let mut hook = TestHook::default();
            body_walk::expressions(&mut module, &mut hook).unwrap();
            assert_eq!(hook.calls, ["m"]);
            // Dropping the fully traversed tree on the same small stack is
            // part of the regression: no recursive cleanup is permitted.
            drop(module);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn bit_segment_binders_cover_later_sizes_and_only_their_clause() {
    let (mut module, errors) =
        ilang_syntax::parse_module("f = { <<m:u(8), _:u(m())>> -> m()  _ -> m() }");
    assert!(errors.is_empty(), "{errors:?}");
    let mut hook = TestHook::default();
    body_walk::expressions(&mut module, &mut hook).unwrap();
    // First segment binds m for the next size and that clause's body. It
    // must not leak to the next clause.
    assert_eq!(hook.calls, ["m"]);
}

#[test]
fn macro_result_is_traversed_without_the_host_stack() {
    use ilang_syntax::ast::Block;
    struct Generate;
    impl Hook for Generate {
        type Error = &'static str;
        fn is_macro(&mut self, e: &Expr, _: &[Local], _: u32) -> Result<bool, Self::Error> {
            Ok(matches!(&e.kind, ExprKind::Call { callee, .. }
                if matches!(&callee.kind, ExprKind::Name { name, .. } if name.name == "m")))
        }
        fn expand(&mut self, _: &Expr, _: &[Local], _: u32) -> Result<Expr, Self::Error> {
            let span = Span::default();
            let mut result = expression("1");
            for index in 0..10_000 {
                result = Expr {
                    span,
                    kind: if index % 2 == 0 {
                        ExprKind::Tuple(vec![result])
                    } else {
                        ExprKind::Block(Block {
                            stmts: vec![Stmt::Expr(result)],
                            span,
                        })
                    },
                };
            }
            Ok(result)
        }
        fn is_type_macro(
            &mut self,
            _: &ilang_syntax::ast::TypeExpr,
            _: &[Local],
            _: u32,
        ) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn expand_type(
            &mut self,
            _: &ilang_syntax::ast::TypeExpr,
            _: &[Local],
            _: u32,
        ) -> Result<ilang_syntax::ast::TypeExpr, Self::Error> {
            unreachable!()
        }
        fn depth_error(&mut self, _: Span) -> Self::Error {
            "depth"
        }
        fn is_direct(&mut self, _: &Expr, _: &[Local], _: u32) -> bool {
            false
        }
    }
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let (mut module, errors) = ilang_syntax::parse_module("f = () { m() }");
            assert!(errors.is_empty(), "{errors:?}");
            body_walk::expressions(&mut module, &mut Generate).unwrap();
            drop(module);
        })
        .unwrap()
        .join()
        .unwrap();
}
