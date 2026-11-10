//! The shape of the resolved program and of the order of top-level values.

mod common;

use ilang_check::hir::*;
use ilang_check::order;

fn ok(src: &str, unused: &[&str]) -> Program {
    let (_, program, diags) = common::check_files(&[("src/main.i", src)]);
    common::expect_unused(&diags, unused);
    program.unwrap()
}

fn binding_names(program: &Program, b: BindingRef) -> Vec<String> {
    program
        .binding(b)
        .defs
        .iter()
        .map(|d| program.def(*d).name.clone())
        .collect()
}

#[test]
fn groups_and_init_order() {
    let program = ok(
        "
a = b()
b = () { c }
c = 1
(even, odd) = ((n) { n == 0 || odd(n - 1) }, (n) { n != 0 && even(n - 1) })
f = (n) { g(n) }
g = (n) { f(n) }
",
        &["a"],
    );
    let main = common::module(&program, "app/main");
    let groups: Vec<Vec<Vec<String>>> = program
        .order
        .groups
        .iter()
        .filter(|g| g[0].module == main)
        .map(|g| g.iter().map(|b| binding_names(&program, *b)).collect())
        .collect();
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    assert_eq!(groups.len(), 5, "{groups:?}");
    assert!(groups.contains(&vec![s(&["f"]), s(&["g"])]));
    assert!(groups.contains(&vec![s(&["even", "odd"])]));
    // `c` before `b` before `a`.
    let pos = |n: &str| {
        groups
            .iter()
            .position(|g| g.iter().any(|b| b[0] == n))
            .unwrap()
    };
    assert!(pos("c") < pos("b") && pos("b") < pos("a"));

    let init: Vec<String> = program
        .order
        .init
        .iter()
        .filter(|b| b.module == main)
        .map(|b| binding_names(&program, *b)[0].clone())
        .collect();
    // `a` calls `b`, which reads `c`: both first. The rest in source order.
    assert_eq!(init, ["b", "c", "a", "even", "f", "g"]);

    // Needs and full needs (§2.5).
    let needs = order::needs(&program, main);
    assert_eq!(needs[0].0.iter().copied().collect::<Vec<_>>(), [1, 2]); // a
    assert!(needs[1].0.is_empty()); // b: direct, `c` is in a function body
    assert_eq!(needs[1].1.iter().copied().collect::<Vec<_>>(), [2]); // b full
    // Library modules come before the modules that import them.
    let first_main = program
        .order
        .init
        .iter()
        .position(|b| b.module == main)
        .unwrap();
    assert!(
        program.order.init[..first_main]
            .iter()
            .all(|b| b.module != main)
    );
}

#[test]
fn names_resolve() {
    let program = ok(
        "\n (i/core, c = (not) = i/core)\n\nT(A) { C(A) }\n\nf(A) = (x: A) -> A {\n  y = C(x)\n   y |> {\n    C(z) | C(z = _) ? True -> core.not(not(True))\n    _ -> panic(\"unreachable\")\n  }\n  x\n}\n",
        &["f"],
    );
    let main = program.module(common::module(&program, "app/main"));
    let b = &main.bindings[0].binding;
    assert!(b.direct);
    assert_eq!(b.ty_vars.len(), 1);
    let ExprKind::Fn(f) = &b.value.kind else {
        panic!()
    };
    let Param::Pattern { pattern, label } = &f.params[0] else {
        panic!()
    };
    assert_eq!(label.as_deref(), Some("x"));
    let PatternKind::Typed {
        binder: Some(x),
        ty,
    } = &pattern.kind
    else {
        panic!()
    };
    assert!(matches!(ty.kind, TypeKind::Var(v) if v == b.ty_vars[0]));
    let Stmt::Bind(y) = &f.body.stmts[0] else {
        panic!()
    };
    let ExprKind::Call { callee, args } = &y.value.kind else {
        panic!()
    };
    assert!(matches!(
        callee.kind,
        ExprKind::Name(ValueRef::Ctor(CtorRef { index: 0, .. }))
    ));
    assert!(matches!(args[0].value.kind, ExprKind::Name(ValueRef::Local(v)) if v == x.var));
    let Stmt::Expr(Expr {
        kind: ExprKind::Case { clauses, .. },
        ..
    }) = &f.body.stmts[1]
    else {
        panic!()
    };
    // The choices bind `z` with one variable.
    let vars: Vec<VarId> = clauses[0]
        .patterns
        .iter()
        .map(|p| match &p.kind {
            PatternKind::Constructor { args, .. } => match &args[0].kind {
                PatternKind::Var(b) | PatternKind::Named { binder: b, .. } => b.var,
                _ => panic!(),
            },
            _ => panic!(),
        })
        .collect();
    assert_eq!(vars[0], vars[1]);
    // `core.not` and `not` are the same definition.
    let ExprKind::Call { callee, args } = &clauses[0].body.kind else {
        panic!()
    };
    let ExprKind::Name(ValueRef::Def(d1)) = callee.kind else {
        panic!()
    };
    let ExprKind::Call { callee: inner, .. } = &args[0].value.kind else {
        panic!()
    };
    assert!(matches!(inner.kind, ExprKind::Name(ValueRef::Def(d2)) if d2 == d1));
    assert_eq!(program.module(d1.module).path, "i/core");
}
