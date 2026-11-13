//! The places of a module that the tests change by hand, for the two trees.
//!
//! `old` is the tree before syntax three (`ilang_syntax::ast`), which
//! `Parsed.ast` holds today. `new` is the tree of syntax three
//! (`ilang_syntax::syn3::ast`), with the same functions, so that a test moves
//! to the new tree by changing `tree::old` to `tree::new`
//! (docs/design/syn3-final-ast.md, step 2d). `tests/tree_helpers.rs` checks
//! that the two give the same place: a change through `new`, lowered, is the
//! change through `old`.

/// The tree before syntax three.
pub mod old {
    use migration_syntax as ilang_syntax;
    use ilang_syntax::ast::{Expr, ExprKind, Ident, ItemKind, Module, PatternKind, Stmt, TypeExpr};
    use ilang_syntax::span::Span;

    /// The statements of the function that the binding `item` gives.
    pub fn statements(module: &mut Module, item: usize) -> &mut Vec<Stmt> {
        let ItemKind::Binding(binding) = &mut module.items[item].kind else {
            panic!("item {item} is not a binding")
        };
        let ExprKind::Fn(function) = &mut binding.value.kind else {
            panic!("item {item} is not a function")
        };
        &mut function.body.stmts
    }

    /// Puts `expr` in place of statement `index` of the function `item`.
    pub fn set_statement(module: &mut Module, item: usize, index: usize, expr: Expr) {
        statements(module, item)[index] = Stmt::Expr(expr);
    }

    /// The last statement of the function `item`, which is an expression.
    pub fn last_expression(module: &mut Module, item: usize) -> &mut Expr {
        let Some(Stmt::Expr(last)) = statements(module, item).last_mut() else {
            panic!("the function {item} does not end with an expression")
        };
        last
    }

    /// Statement `index` of the function `item`, which is an expression.
    pub fn expression(module: &mut Module, item: usize, index: usize) -> &mut Expr {
        let Stmt::Expr(expr) = &mut statements(module, item)[index] else {
            panic!("statement {index} of the function {item} is not an expression")
        };
        expr
    }

    /// The value of the binding `item`.
    pub fn value(module: &mut Module, item: usize) -> &mut Expr {
        let ItemKind::Binding(binding) = &mut module.items[item].kind else {
            panic!("item {item} is not a binding")
        };
        &mut binding.value
    }

    /// The written type of the binding `item` (`name: T = value`).
    pub fn written_type(module: &mut Module, item: usize) -> &mut TypeExpr {
        let ItemKind::Binding(binding) = &mut module.items[item].kind else {
            panic!("item {item} is not a binding")
        };
        let PatternKind::Typed { ty, .. } = &mut binding.pattern.kind else {
            panic!("item {item} has no written type")
        };
        ty
    }

    /// Gives import `import` the alias `name`, at the span of the import.
    pub fn set_import_alias(module: &mut Module, import: usize, name: &str) {
        let import = &mut module.imports[import];
        import.alias = Some(Ident {
            name: name.into(),
            span: import.span,
        });
    }

    /// The name before the dot of `name.member`.
    pub fn field_target_name(expr: &mut Expr) -> &mut String {
        let ExprKind::Field { target, .. } = &mut expr.kind else {
            panic!("not `name.member`")
        };
        let ExprKind::Name { name, .. } = &mut target.kind else {
            panic!("the target of the member is not a name")
        };
        &mut name.name
    }

    /// A string literal with no interpolation.
    pub fn string(text: &str) -> ExprKind {
        ExprKind::String(text.into())
    }

    pub fn span(expr: &Expr) -> Span {
        expr.span
    }
}

/// The tree of syntax three.
pub mod new {
    use ilang_syntax::span::Span;
    use ilang_syntax::syn3::ast::{
        Expr, ExprKind, Ident, ItemKind, Module, PatternKind, Sp, Stmt, Type,
    };

    /// The statements of the function that the binding `item` gives: a chain
    /// of one branch.
    pub fn statements(module: &mut Module, item: usize) -> &mut Vec<Stmt> {
        let ItemKind::Bind(binding) = &mut module.items[item].kind else {
            panic!("item {item} is not a binding")
        };
        let ExprKind::Chain(branches) = &mut binding.value.kind else {
            panic!("item {item} is not a function")
        };
        let [function] = branches.as_mut_slice() else {
            panic!("item {item} is a chain of several branches")
        };
        &mut function.body.stmts
    }

    /// Puts `expr` in place of statement `index` of the function `item`.
    pub fn set_statement(module: &mut Module, item: usize, index: usize, expr: Expr) {
        statements(module, item)[index] = Stmt::Expr(expr);
    }

    /// The last statement of the function `item`, which is an expression.
    pub fn last_expression(module: &mut Module, item: usize) -> &mut Expr {
        let Some(Stmt::Expr(last)) = statements(module, item).last_mut() else {
            panic!("the function {item} does not end with an expression")
        };
        last
    }

    /// Statement `index` of the function `item`, which is an expression.
    pub fn expression(module: &mut Module, item: usize, index: usize) -> &mut Expr {
        let Stmt::Expr(expr) = &mut statements(module, item)[index] else {
            panic!("statement {index} of the function {item} is not an expression")
        };
        expr
    }

    /// The value of the binding `item`.
    pub fn value(module: &mut Module, item: usize) -> &mut Expr {
        let ItemKind::Bind(binding) = &mut module.items[item].kind else {
            panic!("item {item} is not a binding")
        };
        &mut binding.value
    }

    /// The written type of the binding `item` (`name: T = value`).
    pub fn written_type(module: &mut Module, item: usize) -> &mut Type {
        let ItemKind::Bind(binding) = &mut module.items[item].kind else {
            panic!("item {item} is not a binding")
        };
        let PatternKind::Typed { ty, .. } = &mut binding.pattern.kind else {
            panic!("item {item} has no written type")
        };
        ty
    }

    /// Gives import `import` the alias `name`, at the span of the import.
    pub fn set_import_alias(module: &mut Module, import: usize, name: &str) {
        let import = &mut module.imports[import];
        import.alias = Some(Ident {
            name: name.into(),
            span: Sp(import.span.0),
        });
    }

    /// The name before the dot of `name.member`.
    pub fn field_target_name(expr: &mut Expr) -> &mut String {
        let ExprKind::Field { target, .. } = &mut expr.kind else {
            panic!("not `name.member`")
        };
        let ExprKind::Name { name, .. } = &mut target.kind else {
            panic!("the target of the member is not a name")
        };
        &mut name.name
    }

    /// A string literal with no interpolation.
    pub fn string(text: &str) -> ExprKind {
        ExprKind::String(text.into())
    }

    pub fn span(expr: &Expr) -> Span {
        expr.span.0
    }
}
