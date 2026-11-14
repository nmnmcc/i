//! The body walker on the tree of syntax three: the tests of
//! `tests/body_walk.rs` on new trees, the forms that only the new tree has,
//! and a comparison with the old walker on the lowered tree over every
//! source file of the repository.
use ilang_macro::body_walk3::{self, Hook, Local};
use ilang_macro::{body_engine3, call3};
use ilang_syntax::{
    span::Span,
    syn3::ast::{
        Block, DItem, DPItem, Expr, ExprKind, Function, ItemKind, Module, Pattern, PatternKind,
        Stmt, TItem, Type, TypeKind,
    },
};
use migration_syntax::{macro_body_walk as body_walk, macro_call as call};

fn module(source: &str) -> Module {
    let (module, diags) = ilang_syntax::syn3::parse_module(source);
    assert!(diags.is_empty(), "{source}: {diags:?}");
    module
}

fn expression(source: &str) -> Expr {
    let module = module(&format!("f = () {{ {source} }}"));
    let ItemKind::Bind(binding) = &module.items[0].kind else {
        panic!()
    };
    let ExprKind::Chain(functions) = &binding.value.kind else {
        panic!()
    };
    let Stmt::Expr(value) = &functions[0].body.stmts[0] else {
        panic!()
    };
    value.clone()
}

fn declared_type(source: &str) -> Type {
    let module = module(&format!("x: {source}"));
    let ItemKind::Decl { ty, .. } = &module.items[0].kind else {
        panic!()
    };
    ty.clone()
}

#[derive(Default)]
struct TestHook {
    calls: Vec<String>,
    loop_forever: bool,
}
impl Hook for TestHook {
    type Error = &'static str;
    fn is_macro(&mut self, e: &Expr, locals: &[Local], depth: u32) -> Result<bool, Self::Error> {
        let ExprKind::Call { callee, args } = &e.kind else {
            return Ok(false);
        };
        let ExprKind::Name { module: None, name } = &callee.kind else {
            return Ok(false);
        };
        if call3::is_update(args) {
            return Ok(false);
        }
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
    fn is_type_macro(&mut self, _: &Type, _: &[Local], _: u32) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn expand_type(&mut self, _: &Type, _: &[Local], _: u32) -> Result<Type, Self::Error> {
        unreachable!()
    }
    fn depth_error(&mut self, _: Span) -> Self::Error {
        "depth"
    }
    fn is_direct(&mut self, e: &Expr, _: &[Local], _: u32) -> bool {
        matches!(e.kind, ExprKind::Chain(_))
    }
}

fn calls(source: &str) -> Vec<String> {
    let mut module = module(source);
    let mut hook = TestHook::default();
    body_walk3::expressions(&mut module, &mut hook).unwrap();
    hook.calls
}

// ---- The tests of `tests/body_walk.rs` ----

#[test]
fn outer_call_sees_unexpanded_arguments_and_result_is_expanded() {
    assert_eq!(calls("f = () { outer(inner()) }"), ["outer", "inner"]);
}

#[test]
fn local_parameter_and_earlier_binding_shadow_imported_macro() {
    assert_eq!(
        calls("a = (m) { m(1) }\nb = () { m = () { Nil } m(2) }\nc = () { m(3) }"),
        ["m"]
    );
}

#[test]
fn nested_expansions_stop_before_running_the_257th_call() {
    let mut module = module("f = () { m() }");
    let mut hook = TestHook {
        loop_forever: true,
        ..Default::default()
    };
    assert_eq!(
        body_walk3::expressions(&mut module, &mut hook),
        Err("depth")
    );
    assert_eq!(hook.calls.len(), 256);
}

#[test]
fn direct_recursive_bindings_shadow_macros_only_in_function_bodies() {
    // The only macro is the non-direct binding's right-hand side; its own
    // binder does not cover an earlier definition until after that value.
    assert_eq!(
        calls("a = () { m = () { m() } m() }\nb = () { m = m() m() }"),
        ["m"]
    );
}

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
    fn is_type_macro(&mut self, t: &Type, _: &[Local], _: u32) -> Result<bool, Self::Error> {
        Ok(matches!(&t.kind, TypeKind::Call { name, .. }
            if matches!(name.name.as_str(), "outer" | "inner" | "a" | "b" | "c" | "d")))
    }
    fn expand_type(&mut self, t: &Type, _: &[Local], _: u32) -> Result<Type, Self::Error> {
        let TypeKind::Call { name, .. } = &t.kind else {
            unreachable!()
        };
        self.0.push(name.name.clone());
        Ok(declared_type(if name.name == "outer" {
            "inner(Int)"
        } else {
            "Int"
        }))
    }
    fn depth_error(&mut self, _: Span) -> Self::Error {
        "depth"
    }
    fn is_direct(&mut self, _: &Expr, _: &[Local], _: u32) -> bool {
        false
    }
}

fn type_calls(source: &str) -> Vec<String> {
    let mut module = module(source);
    let mut hook = Types::default();
    body_walk3::expressions(&mut module, &mut hook).unwrap();
    hook.0
}

#[test]
fn type_macros_are_expanded_outside_in_in_body_annotations() {
    assert_eq!(type_calls("x: outer(inner(Int))"), ["outer", "inner"]);
}

fn deep_expression(depth: usize, inner: Expr) -> Expr {
    let span = Span::default();
    let mut value = inner;
    for index in 0..depth {
        let block = |value| Block {
            stmts: vec![Stmt::Expr(value)],
            span: span.into(),
        };
        let kind = match index % 4 {
            0 => ExprKind::Data(vec![DItem::Pos(value)]),
            1 => ExprKind::Block(block(value)),
            2 => ExprKind::Chain(vec![Function {
                generics: vec![],
                params: vec![],
                guard: None,
                services: None,
                ret: None,
                body: block(value),
                span: span.into(),
            }]),
            _ => ExprKind::Call {
                callee: Box::new(value),
                args: vec![],
            },
        };
        value = Expr::new(kind, span);
    }
    value
}

/// A small host stack, as the tests of the old walker have.
fn on_small_stack(test: impl FnOnce() -> Module + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let module = test();
            // No recursive cleanup is permitted either.
            drop(module);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn deep_mixed_bodies_patterns_and_types_do_not_use_the_host_stack() {
    on_small_stack(|| {
        let mut module = module("f = () { Nil }\nx: Int");
        let span = Span::default();
        let value = deep_expression(10_000, expression("m()"));
        let ItemKind::Bind(binding) = &mut module.items[0].kind else {
            panic!()
        };
        let ExprKind::Chain(functions) = &mut binding.value.kind else {
            panic!()
        };
        functions[0].body.stmts[0] = Stmt::Expr(value);
        let mut pattern = Pattern::new(PatternKind::Discard("_".into()), span);
        for _ in 0..10_000 {
            pattern = Pattern::new(PatternKind::Data(vec![DPItem::Pos(pattern)]), span);
        }
        binding.pattern = pattern;
        let ItemKind::Decl { ty, .. } = &mut module.items[1].kind else {
            panic!()
        };
        let mut nested = Type::new(TypeKind::Hole, span);
        for _ in 0..10_000 {
            nested = Type::new(TypeKind::Data(vec![TItem::Pos(nested)]), span);
        }
        *ty = nested;
        let mut hook = TestHook::default();
        body_walk3::expressions(&mut module, &mut hook).unwrap();
        assert_eq!(hook.calls, ["m"]);
        module
    });
}

#[test]
fn bit_segment_binders_cover_later_sizes_and_only_their_clause() {
    // First segment binds m for the next size and that clause's body. It
    // must not leak to the next clause.
    assert_eq!(
        calls("f = (<<m:u(8), _:u(m())>>) { m() } | (_) { m() }"),
        ["m"]
    );
}

#[test]
fn macro_result_is_traversed_without_the_host_stack() {
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
                result = Expr::new(
                    if index % 2 == 0 {
                        ExprKind::Data(vec![DItem::Pos(result)])
                    } else {
                        ExprKind::Block(Block {
                            stmts: vec![Stmt::Expr(result)],
                            span: span.into(),
                        })
                    },
                    span,
                );
            }
            Ok(result)
        }
        fn is_type_macro(&mut self, _: &Type, _: &[Local], _: u32) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn expand_type(&mut self, _: &Type, _: &[Local], _: u32) -> Result<Type, Self::Error> {
            unreachable!()
        }
        fn depth_error(&mut self, _: Span) -> Self::Error {
            "depth"
        }
        fn is_direct(&mut self, _: &Expr, _: &[Local], _: u32) -> bool {
            false
        }
    }
    on_small_stack(|| {
        let mut module = module("f = () { m() }");
        body_walk3::expressions(&mut module, &mut Generate).unwrap();
        module
    });
}

#[test]
fn type_macro_result_is_traversed_without_the_host_stack() {
    struct GenerateType(usize);
    impl Hook for GenerateType {
        type Error = &'static str;
        fn is_macro(&mut self, _: &Expr, _: &[Local], _: u32) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn expand(&mut self, _: &Expr, _: &[Local], _: u32) -> Result<Expr, Self::Error> {
            unreachable!()
        }
        fn is_type_macro(&mut self, ty: &Type, _: &[Local], _: u32) -> Result<bool, Self::Error> {
            Ok(matches!(&ty.kind, TypeKind::Call { name, .. } if name.name == "deep"))
        }
        fn expand_type(&mut self, _: &Type, _: &[Local], _: u32) -> Result<Type, Self::Error> {
            self.0 += 1;
            let span = Span::default();
            let mut result = Type::new(TypeKind::Hole, span);
            for _ in 0..10_000 {
                result = Type::new(TypeKind::Data(vec![TItem::Pos(result)]), span);
            }
            Ok(result)
        }
        fn depth_error(&mut self, _: Span) -> Self::Error {
            "depth"
        }
        fn is_direct(&mut self, _: &Expr, _: &[Local], _: u32) -> bool {
            false
        }
    }
    on_small_stack(|| {
        let mut module = module("x: deep(Int)");
        let mut hook = GenerateType(0);
        body_walk3::expressions(&mut module, &mut hook).unwrap();
        assert_eq!(hook.0, 1);
        module
    });
}

// ---- The forms of the tree of syntax three ----

#[test]
fn a_service_name_is_a_binder_of_its_function() {
    assert_eq!(calls("f = (x) <- (m: S) { m(1) }\ng = () { m(2) }"), ["m"]);
    // In a chain that is one function over a match, the service names of
    // every branch cover every branch.
    assert_eq!(
        calls("f = (0) <- (m: S) { m(1) } | (x) { m(2) }\ng = () { m(3) }"),
        ["m"]
    );
}

#[test]
fn a_match_binds_per_branch_at_the_same_function_depth() {
    assert_eq!(calls("f = (x) { x |> (m) { m(1) } | (_) { m(2) } }"), ["m"]);
    // The value of `m` is not direct (it is a match), so `m` is not in
    // scope inside its own value.
    assert_eq!(calls("f = (x) { m = x |> (_) { m(1) }  m(2) }"), ["m"]);
}

#[test]
fn a_clause_block_binds_the_first_choice_of_each_branch() {
    assert_eq!(calls("f = (m) { m(1) } | (_) { m(2) }"), ["m"]);
    assert_eq!(calls("f = (m, 0) { m(1) } | (_, _) { m(2) }"), ["m"]);
    assert_eq!(calls("f = (m ? m(0)) { 1 } | (_ ? m(1)) { 2 }"), ["m"]);
}

#[test]
fn provide_visits_the_services_and_then_the_target() {
    assert_eq!(
        calls("f = () { inner(1) <- (outer(2)) }"),
        ["outer", "inner", "inner"]
    );
    assert_eq!(calls("f = () { { m = 1  m(1) } <- (m(2), s) }"), ["m"]);
}

#[test]
fn conjunction_and_key_patterns_bind_as_the_old_named_patterns() {
    assert_eq!(calls("f = ((a, b) = m) { m(1) }"), Vec::<String>::new());
    assert_eq!(calls("f = (C(k = m)) { m(1) }"), Vec::<String>::new());
    // The label of a key is a binder too (the old `name = label`).
    assert_eq!(calls("f = (C(m = x)) { m(1) }"), Vec::<String>::new());
    assert_eq!(calls("f = (C(m = _)) { m(1) }"), Vec::<String>::new());
    assert_eq!(calls("f = (C(k = x)) { m(1) }"), ["m"]);
    let binders = |source: &str| {
        let pattern =
            ilang_syntax::syn3::parse_pattern(source).unwrap_or_else(|e| panic!("{source}: {e:?}"));
        let mut out = Vec::new();
        body_walk3::for_each_binder(&pattern, |name, _| out.push(name.to_owned()));
        out
    };
    assert_eq!(
        binders("C(a, k = b, c = d: Int, e = e)"),
        ["a", "b", "k", "d", "c", "e"]
    );
    assert_eq!(binders("(x, y) = whole"), ["whole", "x", "y"]);
    assert_eq!(binders("[a, b, ..rest]"), ["a", "b", "rest"]);
}

#[test]
fn interpolation_parts_are_visited() {
    assert_eq!(
        calls("f = () { \"a\\(m(1))b\\(inner(2))\" }"),
        ["m", "inner"]
    );
}

#[test]
fn a_record_update_is_not_a_macro_call() {
    assert_eq!(calls("f = (r) { m(..r, a = inner(1)) }"), ["inner"]);
    // A pipe into an ordinary call still visits that call.
    assert_eq!(calls("f = (r) { r |> m(1) }"), ["m"]);
}

#[test]
fn a_function_type_is_visited_as_parameters_result_error_environment() {
    assert_eq!(
        type_calls("x: (a(Int)) <- (S(b(Int)), Fail(c(Int))) -> d(Int)"),
        ["a", "d", "c", "b"]
    );
}

// ---- The same answers as the old walker on the lowered tree ----

type Event = (String, (u32, u32), String, Vec<(String, (u32, u32))>);

fn visible(locals: &[Local], depth: u32) -> Vec<(String, (u32, u32))> {
    locals
        .iter()
        .filter(|l| l.fn_only.is_none_or(|level| depth > level))
        // The lowering's own names (`%arg1`, `%service1`, `%field1`).
        .filter(|l| !l.name.contains('%'))
        .map(|l| (l.name.clone(), (l.span.start, l.span.end)))
        .collect()
}

fn at(span: Span) -> (u32, u32) {
    (span.start, span.end)
}

#[derive(Default)]
struct New {
    events: Vec<Event>,
    direct: Vec<((u32, u32), bool)>,
    aliases: Vec<String>,
}
impl Hook for New {
    type Error = ();
    fn is_macro(&mut self, e: &Expr, locals: &[Local], depth: u32) -> Result<bool, ()> {
        if let Some(call) = call3::expression(e) {
            // `core.never(x)` is a match with no clause in the old tree.
            let never = call.name.name == "never"
                && call.module.is_some_and(|m| m.name == "core")
                && call.prefix.is_none();
            if !never {
                self.events.push((
                    "call".into(),
                    at(e.span.0),
                    format!(
                        "{}.{}/{}{}",
                        call.module.map_or("", |m| &m.name),
                        call.name.name,
                        call.args.len(),
                        if call.prefix.is_some() { "+pipe" } else { "" }
                    ),
                    visible(locals, depth),
                ));
            }
        }
        Ok(false)
    }
    fn expand(&mut self, _: &Expr, _: &[Local], _: u32) -> Result<Expr, ()> {
        unreachable!()
    }
    fn is_type_macro(&mut self, t: &Type, locals: &[Local], depth: u32) -> Result<bool, ()> {
        if let Some(call) = call3::ty(t) {
            self.events.push((
                "type".into(),
                at(t.span.0),
                format!(
                    "{}.{}/{}",
                    call.module.map_or("", |m| &m.name),
                    call.name.name,
                    call.args.len()
                ),
                visible(locals, depth),
            ));
        }
        Ok(false)
    }
    fn expand_type(&mut self, _: &Type, _: &[Local], _: u32) -> Result<Type, ()> {
        unreachable!()
    }
    fn depth_error(&mut self, _: Span) {}
    fn is_direct(&mut self, e: &Expr, _: &[Local], _: u32) -> bool {
        let direct = body_engine3::is_direct(e, |name| self.aliases.contains(&name.name));
        // Multi-shape values have a synthetic one-byte root in the
        // transition AST. Compare their classification at that root.
        let span = if matches!(e.kind, ExprKind::Chain(_)) {
            migration_syntax::syn3::lower::lower_expr(e)
                .map(|e| e.span)
                .unwrap_or(e.span.0)
        } else {
            e.span.0
        };
        self.direct.push((at(span), direct));
        direct
    }
}

#[derive(Default)]
struct Old {
    events: Vec<Event>,
    direct: std::collections::HashMap<(u32, u32), bool>,
    aliases: Vec<String>,
}
/// `BodyEngine::is_direct` of the old tree, with the module aliases given.
fn old_is_direct(expression: &migration_syntax::ast::Expr, aliases: &[String]) -> bool {
    use migration_syntax::ast::{BinOp, Expr, ExprKind, Stmt};
    fn ungroup(mut expression: &Expr) -> &Expr {
        while let ExprKind::Block(block) = &expression.kind {
            let [Stmt::Expr(inner)] = block.stmts.as_slice() else {
                break;
            };
            expression = inner;
        }
        expression
    }
    fn constructor(expression: &Expr) -> bool {
        matches!(&expression.kind, ExprKind::Name { name, .. } if name.name.chars().next().is_some_and(char::is_uppercase))
    }
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match &ungroup(expression).kind {
            ExprKind::Fn(_)
            | ExprKind::Clauses(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Name { .. } => {}
            ExprKind::Unary { expr, .. }
                if matches!(
                    ungroup(expr).kind,
                    ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::String(_)
                ) => {}
            ExprKind::Field { target, .. } => match &target.kind {
                ExprKind::Name { module: None, name } if aliases.contains(&name.name) => {}
                _ => return false,
            },
            ExprKind::Tuple(items) => pending.extend(items),
            ExprKind::List { items, rest } => {
                pending.extend(items);
                pending.extend(rest.as_deref());
            }
            ExprKind::Call { callee, args } if matches!(&callee.kind, ExprKind::Name { module: None, name } if name.name == migration_syntax::syn3::lower::MULTI) =>
            {
                pending.extend(args.iter().map(|arg| &arg.value));
            }
            ExprKind::Call { callee, args } if constructor(callee) => {
                pending.extend(args.iter().map(|arg| &arg.value))
            }
            ExprKind::Binary {
                op: BinOp::Pipe,
                lhs,
                rhs,
            } => {
                pending.push(lhs);
                match &rhs.kind {
                    ExprKind::Name { .. } if constructor(rhs) => {}
                    ExprKind::Call { callee, args } if constructor(callee) => {
                        pending.extend(args.iter().map(|arg| &arg.value))
                    }
                    _ => return false,
                }
            }
            _ => return false,
        }
    }
    true
}
impl body_walk::Hook for Old {
    type Error = ();
    fn is_macro(
        &mut self,
        e: &migration_syntax::ast::Expr,
        locals: &[body_walk::Local],
        depth: u32,
    ) -> Result<bool, ()> {
        if let Some(call) = call::expression(e) {
            // A shape-group closure is introduced by transition lowering;
            // it has no call in the source tree (syntax-3 §6.12).
            if call.name.name.starts_with('%') {
                return Ok(false);
            }
            self.events.push((
                "call".into(),
                at(e.span),
                format!(
                    "{}.{}/{}{}",
                    call.module.map_or("", |m| &m.name),
                    call.name.name,
                    call.args.len(),
                    if call.prefix.is_some() { "+pipe" } else { "" }
                ),
                visible_old(locals, depth),
            ));
        }
        Ok(false)
    }
    fn expand(
        &mut self,
        _: &migration_syntax::ast::Expr,
        _: &[body_walk::Local],
        _: u32,
    ) -> Result<migration_syntax::ast::Expr, ()> {
        unreachable!()
    }
    fn is_type_macro(
        &mut self,
        t: &migration_syntax::ast::TypeExpr,
        locals: &[body_walk::Local],
        depth: u32,
    ) -> Result<bool, ()> {
        if let Some(call) = call::ty(t) {
            self.events.push((
                "type".into(),
                at(t.span),
                format!(
                    "{}.{}/{}",
                    call.module.map_or("", |m| &m.name),
                    call.name.name,
                    call.args.len()
                ),
                visible_old(locals, depth),
            ));
        }
        Ok(false)
    }
    fn expand_type(
        &mut self,
        _: &migration_syntax::ast::TypeExpr,
        _: &[body_walk::Local],
        _: u32,
    ) -> Result<migration_syntax::ast::TypeExpr, ()> {
        unreachable!()
    }
    fn depth_error(&mut self, _: Span) {}
    fn is_direct(
        &mut self,
        e: &migration_syntax::ast::Expr,
        _: &[body_walk::Local],
        _: u32,
    ) -> bool {
        let direct = old_is_direct(e, &self.aliases);
        self.direct.insert(at(e.span), direct);
        direct
    }
}

/// Compares the two walkers on one source. Returns false when the source
/// is not a program (it has syntax errors, or errors of the lowering).
fn same_as_old_walker(name: &str, source: &str, lisp: bool) -> bool {
    let (mut new, errors) = if lisp {
        ilang_syntax::syn3::lisp::from_lisp(source)
    } else {
        ilang_syntax::syn3::parse_module(source)
    };
    if !errors.is_empty() {
        return false;
    }
    let (mut old, errors) = if lisp {
        migration_syntax::syn3::lower::lower_lisp(&new)
    } else {
        migration_syntax::syn3::lower::lower(&new)
    };
    if !errors.is_empty() {
        return false;
    }
    let is_macro_item = |m: &Module| {
        m.items
            .iter()
            .any(|i| matches!(i.kind, ItemKind::Macro { .. }))
    };
    if is_macro_item(&new) {
        return false;
    }
    let aliases: Vec<String> = new
        .imports
        .iter()
        .filter_map(|i| i.alias.as_ref().or(i.path.last()))
        .map(|a| a.name.clone())
        .collect();
    let mut new_hook = New {
        aliases: aliases.clone(),
        ..Default::default()
    };
    body_walk3::expressions(&mut new, &mut new_hook).unwrap();
    let mut old_hook = Old {
        aliases,
        ..Default::default()
    };
    body_walk::expressions(&mut old, &mut old_hook).unwrap();
    // Lowering groups multi-shape branches and may hoist their hidden
    // bindings. Compare source events in source order, retaining every
    // written call and its lexical environment on both sides.
    new_hook.events.sort();
    old_hook.events.sort();
    if new_hook.events != old_hook.events {
        let first = new_hook
            .events
            .iter()
            .zip(&old_hook.events)
            .position(|(a, b)| a != b)
            .unwrap_or(new_hook.events.len().min(old_hook.events.len()));
        panic!(
            "{name}: event {first} differs\n new: {:?}\n old: {:?}\n({} new, {} old)",
            new_hook.events.get(first),
            old_hook.events.get(first),
            new_hook.events.len(),
            old_hook.events.len()
        );
    }
    for (span, direct) in &new_hook.direct {
        assert_eq!(
            old_hook.direct.get(span),
            Some(direct),
            "{name}: is_direct of the value at {span:?}"
        );
    }
    true
}

#[test]
fn snippets_walk_as_their_lowered_trees() {
    let cases = [
        "f = (m) { m(1) }",
        "S = (f = () -> Int)\nf = (body) { provided: S = S(f = () { g() }) body() <- (provided) }",
        "S(N) = (f = () <- N -> Int)\nf = <N>(body) { provided: S(N) = S(f = () { g() }) body() <- (provided) }",
        "S(N) = (f = () <- N -> Int)\nf = (body) { provided: S(_) = S(f = () { g() }) body() <- (provided) }",
        "S(A) = (A)\nf = (body) { provided: S(Int) = S(1) body() <- (provided) }",
        "S = (f = () -> Int)\nf = (body) { provided: S = S(f = () { g() }) touch(provided) body() <- (provided) }",
        "S = (f = () -> Int)\nf = (body) { p: S = S(f = () { g() }) { p: S = S(f = () { h() }) body() <- (p) } <- (p) }",
        "f = () <- (failure: Fail(Int), service: S) -> Int { g() }",
        "f = (x) <- (failure: Fail(Int)) -> Int { g(x) } | (x) <- (failure: Fail(Int)) { h(x) }",
        "Alias(E) = Fail(E)\nf = () <- (failure: Alias(Int)) { g() }",
        "f = (a, b) -> Int { g(a) |> h(b) }",
        "f = (0, x) <- (s: S) -> Int { g(s, x) } | (n, _) <- (t: T) { h(n, t, s) }",
        "f = (x) { x |> (Some(y)) { g(y) } | (None) { h(x) } }",
        "f = (x) { x |> (a) -> Int { g(a) } }",
        "f = (x) { y = (z) { y(z) }  w = (y, [y(1)], C(y(2)), y(3) |> C)  w(y) }",
        "f = (r) { C(..r, a = g(r)) |> h }",
        "f = () { g(1) <- (h(2), k) }",
        "f = () { { a = g(1)  h(a) } <- (s) }",
        "f = () { a = 1  g(a) <- (h(a)) }",
        "f = (C(k = v), D(k2 = _), E(a = b: Int, ..)) { g(v, k, k2, a, b) }",
        "f = ((a, b) = c, [d, ..e]) { g(a, b, c, d, e) }",
        "f = (<<a:u(8), b:bytes(g(a))>>) { h(a, b) }",
        "f = (x ? g(x)) { h(x) } | (y | y) { k(y) }",
        "f = (x) { \"a\\(g(x))b\" }",
        "f = (x) { core.never(x) }",
        "x: (m(Int), a = n(Int)) <- (S, Fail(o(Int)), ..N) -> p(Int)",
        "f: <A, N>(A) <- N -> q(A) = (a) { g(a) }",
        "f = <A>(a: m(A)) -> n(A) { (b: o(A)) = g(a)  b }",
        "f = (x) { (x, g(x)).0 + -h(1) }",
        "f = () { (a = g(1), b = h(2)) }",
    ];
    for case in cases {
        assert!(
            same_as_old_walker(case, case, false),
            "{case}: not a program"
        );
        let text = ilang_syntax::syn3::lisp::to_lisp(&module(case));
        same_as_old_walker(&text, &text, true);
    }
}

fn sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !matches!(
                &*name,
                "target" | "node_modules" | "binaryen" | ".git" | ".rd"
            ) {
                sources(&path, out);
            }
        } else if name.ends_with(".i") || name.ends_with(".is") {
            out.push(path);
        }
    }
}

/// Every `.i` and `.is` file of the repository that is a program.
#[test]
fn repository_sources_walk_as_their_lowered_trees() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    sources(&root, &mut files);
    files.sort();
    let mut compared = 0;
    for path in &files {
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        let lisp = path.extension().is_some_and(|e| e == "is");
        if same_as_old_walker(&path.display().to_string(), &source, lisp) {
            compared += 1;
        }
    }
    assert!(compared > 1000, "only {compared} of {} files", files.len());
}

// The private oracle owns its walker locals. Compare the same source facts.
fn visible_old(locals: &[body_walk::Local], depth: u32) -> Vec<(String, (u32, u32))> {
    let locals: Vec<_> = locals
        .iter()
        .map(|l| Local {
            name: l.name.clone(),
            span: l.span,
            fn_only: l.fn_only,
        })
        .collect();
    visible(&locals, depth)
}
