//! Outside-in traversal of expression macro positions.
//!
//! The hook decides whether a call resolves to a macro and returns its result.
//! Arguments are not traversed before that decision. Local frames carry source
//! spans so the hook can compare scopes rather than only written names.
use ilang_syntax::ast::{self, Expr, ExprKind, Pattern, PatternKind, Stmt};
use ilang_syntax::span::Span;

#[derive(Clone, Debug)]
pub struct Local {
    pub name: String,
    pub span: Span,
    /// A recursive direct binding is visible only inside a deeper function.
    pub fn_only: Option<u32>,
}

pub trait Hook {
    type Error;
    fn is_macro(
        &mut self,
        expression: &Expr,
        locals: &[Local],
        fn_depth: u32,
    ) -> Result<bool, Self::Error>;
    fn expand(
        &mut self,
        expression: &Expr,
        locals: &[Local],
        fn_depth: u32,
    ) -> Result<Expr, Self::Error>;
    fn is_type_macro(
        &mut self,
        ty: &ast::TypeExpr,
        locals: &[Local],
        fn_depth: u32,
    ) -> Result<bool, Self::Error>;
    fn expand_type(
        &mut self,
        ty: &ast::TypeExpr,
        locals: &[Local],
        fn_depth: u32,
    ) -> Result<ast::TypeExpr, Self::Error>;
    fn depth_error(&mut self, span: Span) -> Self::Error;
    /// Whether a binding has a direct value (§6.10); module-qualified names
    /// require the same scope-aware lookup used by name resolution.
    fn is_direct(&mut self, expression: &Expr, locals: &[Local], fn_depth: u32) -> bool;
}

/// Expand one module's expression positions. A returned macro expression is
/// visited before any of its children, so an outer macro sees written arguments.
pub fn expressions<H: Hook>(module: &mut ast::Module, hook: &mut H) -> Result<(), H::Error> {
    let mut walker = Walker {
        hook,
        locals: Vec::new(),
        fn_depth: 0,
    };
    for item in &mut module.items {
        match &mut item.kind {
            ast::ItemKind::Binding(binding) => {
                walker.pattern(&mut binding.pattern, 0)?;
                walker.expr(&mut binding.value, 0)?;
            }
            ast::ItemKind::MacroItem { .. } => {
                panic!("item expansion must finish before body expansion")
            }
            ast::ItemKind::Declaration { ty, .. } => walker.ty(ty, 0)?,
            ast::ItemKind::TypeDef(_) => {}
        }
    }
    Ok(())
}

/// Expand type-position macros in one written type definition during the item
/// phase. Bindings, declarations and their type positions are left for bodies.
pub fn type_definition<H: Hook>(
    definition: &mut ast::TypeDef,
    hook: &mut H,
) -> Result<(), H::Error> {
    let mut walker = Walker {
        hook,
        locals: Vec::new(),
        fn_depth: 0,
    };
    for head in &mut definition.head {
        walker.ty(head, 0)?;
    }
    match &mut definition.body {
        ast::TypeBody::Alias(ty) => walker.ty(ty, 0)?,
        ast::TypeBody::Custom(constructors) => {
            for constructor in constructors {
                for field in &mut constructor.fields {
                    walker.ty(&mut field.ty, 0)?;
                }
            }
        }
    }
    Ok(())
}

struct Walker<'a, H> {
    hook: &'a mut H,
    locals: Vec<Local>,
    fn_depth: u32,
}
impl<H: Hook> Walker<'_, H> {
    fn expr(&mut self, expr: &mut Expr, depth: u16) -> Result<(), H::Error> {
        if self.hook.is_macro(expr, &self.locals, self.fn_depth)? {
            if depth == 256 {
                return Err(self.hook.depth_error(expr.span));
            }
            *expr = self.hook.expand(expr, &self.locals, self.fn_depth)?;
            return self.expr(expr, depth + 1);
        }
        match &mut expr.kind {
            ExprKind::Quote(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Name { .. } => {}
            ExprKind::Tuple(items) => {
                for item in items {
                    self.expr(item, depth)?;
                }
            }
            ExprKind::List { items, rest } => {
                for item in items {
                    self.expr(item, depth)?;
                }
                if let Some(rest) = rest {
                    self.expr(rest, depth)?;
                }
            }
            ExprKind::BitArray(segments) => {
                for segment in segments {
                    self.expr(&mut segment.value, depth)?;
                    if let Some(format) = &mut segment.format
                        && let Some(arg) = &mut format.arg
                    {
                        self.expr(arg, depth)?;
                    }
                }
            }
            ExprKind::Block(block) => self.block(block, depth)?,
            ExprKind::Fn(function) => {
                let start = self.locals.len();
                // Parameter patterns are checked before any parameter enters
                // scope; earlier segments of one bit pattern are handled by
                // `pattern` itself.
                for param in &mut function.params {
                    match param {
                        ast::Param::Pattern(pattern) => self.pattern(pattern, depth)?,
                        ast::Param::Env { ty, .. } => self.ty(ty, depth)?,
                    }
                }
                if let Some(ret) = &mut function.ret {
                    self.ty(&mut ret.value, depth)?;
                    if let Some(err) = &mut ret.err {
                        self.ty(err, depth)?;
                    }
                    if let Some(env) = &mut ret.env {
                        self.ty(env, depth)?;
                    }
                }
                self.fn_depth += 1;
                for param in &function.params {
                    match param {
                        ast::Param::Pattern(pattern) => binders(pattern, &mut self.locals, None),
                        ast::Param::Env { name, .. } => self.locals.push(Local {
                            name: name.name.clone(),
                            span: name.span,
                            fn_only: None,
                        }),
                    }
                }
                self.block(&mut function.body, depth)?;
                self.locals.truncate(start);
                self.fn_depth -= 1;
            }
            ExprKind::Case { subject, clauses } => {
                self.expr(subject, depth)?;
                for clause in clauses {
                    self.clause(clause, depth, false)?;
                }
            }
            ExprKind::Clauses(clauses) => {
                for clause in clauses {
                    self.clause(clause, depth, true)?;
                }
            }
            ExprKind::Call { callee, args } => {
                self.expr(callee, depth)?;
                for arg in args {
                    self.expr(&mut arg.value, depth)?;
                }
            }
            ExprKind::Field { target, .. } | ExprKind::TupleIndex { target, .. } => {
                self.expr(target, depth)?
            }
            ExprKind::RecordUpdate {
                constructor,
                base,
                fields,
            } => {
                self.expr(constructor, depth)?;
                self.expr(base, depth)?;
                for (_, value) in fields {
                    self.expr(value, depth)?;
                }
            }
            ExprKind::Unary { expr, .. } => self.expr(expr, depth)?,
            ExprKind::Binary { lhs, rhs, .. } => {
                self.expr(lhs, depth)?;
                self.expr(rhs, depth)?;
            }
        }
        Ok(())
    }
    fn block(&mut self, block: &mut ast::Block, depth: u16) -> Result<(), H::Error> {
        let start = self.locals.len();
        for stmt in &mut block.stmts {
            match stmt {
                Stmt::Bind(binding) => {
                    self.pattern(&mut binding.pattern, depth)?;
                    let before = self.locals.len();
                    if self
                        .hook
                        .is_direct(&binding.value, &self.locals, self.fn_depth)
                    {
                        binders(&binding.pattern, &mut self.locals, Some(self.fn_depth));
                    }
                    self.expr(&mut binding.value, depth)?;
                    self.locals.truncate(before);
                    binders(&binding.pattern, &mut self.locals, None);
                }
                Stmt::Env { ty, value, .. } => {
                    self.ty(ty, depth)?;
                    self.expr(value, depth)?;
                }
                Stmt::Expr(value) => self.expr(value, depth)?,
            }
        }
        self.locals.truncate(start);
        Ok(())
    }
    fn clause(
        &mut self,
        clause: &mut ast::Clause,
        depth: u16,
        function: bool,
    ) -> Result<(), H::Error> {
        let start = self.locals.len();
        for pattern in &mut clause.patterns {
            self.pattern(pattern, depth)?;
        }
        if function {
            self.fn_depth += 1;
        }
        if let Some(first) = clause.patterns.first() {
            binders(first, &mut self.locals, None);
        }
        if let Some(guard) = &mut clause.guard {
            self.expr(guard, depth)?;
        }
        self.expr(&mut clause.body, depth)?;
        self.locals.truncate(start);
        if function {
            self.fn_depth -= 1;
        }
        Ok(())
    }
    fn ty(&mut self, ty: &mut ast::TypeExpr, depth: u16) -> Result<(), H::Error> {
        if self.hook.is_type_macro(ty, &self.locals, self.fn_depth)? {
            if depth == 256 {
                return Err(self.hook.depth_error(ty.span));
            }
            *ty = self.hook.expand_type(ty, &self.locals, self.fn_depth)?;
            return self.ty(ty, depth + 1);
        }
        match &mut ty.kind {
            ast::TypeKind::Named { args, .. }
            | ast::TypeKind::Call { args, .. }
            | ast::TypeKind::Tuple(args)
            | ast::TypeKind::Union(args)
            | ast::TypeKind::Inter(args)
            | ast::TypeKind::Diff(args) => {
                for arg in args {
                    self.ty(arg, depth)?;
                }
            }
            ast::TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => {
                for param in params {
                    self.ty(&mut param.ty, depth)?;
                }
                self.ty(ret, depth)?;
                if let Some(err) = err {
                    self.ty(err, depth)?;
                }
                if let Some(env) = env {
                    self.ty(env, depth)?;
                }
            }
            ast::TypeKind::Lit(_) | ast::TypeKind::ValueOf { .. } | ast::TypeKind::Hole => {}
        }
        Ok(())
    }
    fn pattern(&mut self, pattern: &mut Pattern, depth: u16) -> Result<(), H::Error> {
        match &mut pattern.kind {
            PatternKind::Named { pattern, .. } => self.pattern(pattern, depth)?,
            PatternKind::Constructor { args, .. } | PatternKind::Tuple(args) => {
                for arg in args {
                    self.pattern(arg, depth)?;
                }
            }
            PatternKind::List { items, .. } => {
                for item in items {
                    self.pattern(item, depth)?;
                }
            }
            PatternKind::BitArray(segments) => {
                let start = self.locals.len();
                for segment in segments {
                    if let Some(format) = &mut segment.format
                        && let Some(arg) = &mut format.arg
                    {
                        self.expr(arg, depth)?;
                    }
                    self.pattern(&mut segment.value, depth)?;
                    binders(&segment.value, &mut self.locals, None);
                }
                self.locals.truncate(start);
            }
            PatternKind::Typed { ty, .. } => self.ty(ty, depth)?,
            PatternKind::Quote { captures, .. } => {
                for (_, pattern) in captures {
                    self.pattern(pattern, depth)?;
                }
            }
            PatternKind::Var(_)
            | PatternKind::Discard(_)
            | PatternKind::Int { .. }
            | PatternKind::Float { .. }
            | PatternKind::String(_)
            | PatternKind::Prefix { .. } => {}
        }
        Ok(())
    }
}

pub(crate) fn for_each_binder<'a>(
    pattern: &'a Pattern,
    mut visit: impl FnMut(&'a str, ilang_syntax::span::Span),
) {
    enum Task<'a> {
        Pattern(&'a Pattern),
        Binder(&'a str, ilang_syntax::span::Span),
    }
    let mut pending = vec![Task::Pattern(pattern)];
    while let Some(task) = pending.pop() {
        let pattern = match task {
            Task::Pattern(pattern) => pattern,
            Task::Binder(name, span) => {
                visit(name, span);
                continue;
            }
        };
        match &pattern.kind {
            PatternKind::Quote { captures, .. } => pending.extend(
                captures
                    .iter()
                    .rev()
                    .map(|(_, pattern)| Task::Pattern(pattern)),
            ),
            PatternKind::Var(name) => visit(name, pattern.span),
            PatternKind::Typed { binder, .. } if !binder.discard => {
                visit(&binder.name, binder.span)
            }
            PatternKind::Named { name, pattern } => {
                visit(&name.name, name.span);
                pending.push(Task::Pattern(pattern));
            }
            PatternKind::Prefix { rest, .. } if !rest.discard => visit(&rest.name, rest.span),
            PatternKind::Constructor { args, .. } | PatternKind::Tuple(args) => {
                pending.extend(args.iter().rev().map(Task::Pattern))
            }
            PatternKind::List { items, rest } => {
                if let Some(Some(rest)) = rest
                    && !rest.discard
                {
                    pending.push(Task::Binder(&rest.name, rest.span));
                }
                pending.extend(items.iter().rev().map(Task::Pattern));
            }
            PatternKind::BitArray(segments) => pending.extend(
                segments
                    .iter()
                    .rev()
                    .map(|segment| Task::Pattern(&segment.value)),
            ),
            _ => {}
        }
    }
}

fn binders(pattern: &Pattern, out: &mut Vec<Local>, fn_only: Option<u32>) {
    for_each_binder(pattern, |name, span| {
        out.push(Local {
            name: name.into(),
            span,
            fn_only,
        })
    });
}
