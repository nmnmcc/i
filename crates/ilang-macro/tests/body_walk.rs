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
