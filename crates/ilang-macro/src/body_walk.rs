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
// A task contains an address into the module currently borrowed by the entry
// point. The walker never changes a containing vector or replaces a node while
// tasks for its children remain. Expansion replaces a node before any children
// are scheduled. Only the current task is dereferenced; hook methods receive
// shared references and cannot mutate the tree. Thus these addresses stay
// valid, and mutable references made from them never overlap.
enum Task {
    Expr(*mut Expr, u16),
    Ty(*mut ast::TypeExpr, u16),
    Pattern(*mut Pattern, u16),
    Block(*mut ast::Block, u16),
    Stmt(*mut Stmt, u16),
    BindValue(*mut ast::Binding, u16),
    BindDone(*mut ast::Binding, usize),
    FunctionEnter(*mut ast::Function, u16),
    Clause(*mut ast::Clause, u16, bool),
    ClauseEnter(*mut ast::Clause, u16, bool, usize),
    BitSegment(*mut ast::Segment<Pattern>, u16),
    BindPattern(*mut Pattern),
    Restore(usize, u32),
}

impl<H: Hook> Walker<'_, H> {
    fn expr(&mut self, expr: &mut Expr, depth: u16) -> Result<(), H::Error> {
        self.run(Task::Expr(expr, depth))
    }
    fn ty(&mut self, ty: &mut ast::TypeExpr, depth: u16) -> Result<(), H::Error> {
        self.run(Task::Ty(ty, depth))
    }
    fn pattern(&mut self, pattern: &mut Pattern, depth: u16) -> Result<(), H::Error> {
        self.run(Task::Pattern(pattern, depth))
    }

    fn run(&mut self, first: Task) -> Result<(), H::Error> {
        let mut tasks = vec![first];
        while let Some(task) = tasks.pop() {
            // SAFETY: all task pointers obey the tree-borrow and scheduling
            // invariants documented on Task above. A task is consumed before
            // a reference is created and no reference escapes this iteration.
            unsafe {
                match task {
                    Task::Expr(ptr, depth) => {
                        let expr = &mut *ptr;
                        if self.hook.is_macro(expr, &self.locals, self.fn_depth)? {
                            if depth == 256 {
                                return Err(self.hook.depth_error(expr.span));
                            }
                            *expr = self.hook.expand(expr, &self.locals, self.fn_depth)?;
                            tasks.push(Task::Expr(ptr, depth + 1));
                            continue;
                        }
                        match &mut expr.kind {
                            ExprKind::Quote(_)
                            | ExprKind::Int(_)
                            | ExprKind::Float(_)
                            | ExprKind::String(_)
                            | ExprKind::Name { .. } => {}
                            ExprKind::Tuple(items) => {
                                tasks.extend(items.iter_mut().rev().map(|e| Task::Expr(e, depth)));
                            }
                            ExprKind::List { items, rest } => {
                                if let Some(rest) = rest {
                                    tasks.push(Task::Expr(&mut **rest, depth));
                                }
                                tasks.extend(items.iter_mut().rev().map(|e| Task::Expr(e, depth)));
                            }
                            ExprKind::BitArray(segments) => {
                                for segment in segments.iter_mut().rev() {
                                    if let Some(format) = &mut segment.format
                                        && let Some(arg) = &mut format.arg
                                    {
                                        tasks.push(Task::Expr(&mut **arg, depth));
                                    }
                                    tasks.push(Task::Expr(&mut segment.value, depth));
                                }
                            }
                            ExprKind::Block(block) => tasks.push(Task::Block(block, depth)),
                            ExprKind::Fn(function) => {
                                tasks.push(Task::FunctionEnter(&mut **function, depth));
                                if let Some(ret) = &mut function.ret {
                                    if let Some(env) = &mut ret.env {
                                        tasks.push(Task::Ty(env, depth));
                                    }
                                    if let Some(err) = &mut ret.err {
                                        tasks.push(Task::Ty(err, depth));
                                    }
                                    tasks.push(Task::Ty(&mut ret.value, depth));
                                }
                                for param in function.params.iter_mut().rev() {
                                    tasks.push(match param {
                                        ast::Param::Pattern(pattern) => {
                                            Task::Pattern(pattern, depth)
                                        }
                                        ast::Param::Env { ty, .. } => Task::Ty(ty, depth),
                                    });
                                }
                            }
                            ExprKind::Case { subject, clauses } => {
                                tasks.extend(
                                    clauses
                                        .iter_mut()
                                        .rev()
                                        .map(|c| Task::Clause(c, depth, false)),
                                );
                                tasks.push(Task::Expr(&mut **subject, depth));
                            }
                            ExprKind::Clauses(clauses) => {
                                tasks.extend(
                                    clauses
                                        .iter_mut()
                                        .rev()
                                        .map(|c| Task::Clause(c, depth, true)),
                                );
                            }
                            ExprKind::Call { callee, args } => {
                                tasks.extend(
                                    args.iter_mut()
                                        .rev()
                                        .map(|a| Task::Expr(&mut a.value, depth)),
                                );
                                tasks.push(Task::Expr(&mut **callee, depth));
                            }
                            ExprKind::Field { target, .. }
                            | ExprKind::TupleIndex { target, .. } => {
                                tasks.push(Task::Expr(&mut **target, depth));
                            }
                            ExprKind::RecordUpdate {
                                constructor,
                                base,
                                fields,
                            } => {
                                tasks.extend(
                                    fields
                                        .iter_mut()
                                        .rev()
                                        .map(|(_, value)| Task::Expr(value, depth)),
                                );
                                tasks.push(Task::Expr(&mut **base, depth));
                                tasks.push(Task::Expr(&mut **constructor, depth));
                            }
                            ExprKind::Unary { expr, .. } => {
                                tasks.push(Task::Expr(&mut **expr, depth))
                            }
                            ExprKind::Binary { lhs, rhs, .. } => {
                                tasks.push(Task::Expr(&mut **rhs, depth));
                                tasks.push(Task::Expr(&mut **lhs, depth));
                            }
                        }
                    }
                    Task::Ty(ptr, depth) => {
                        let ty = &mut *ptr;
                        if self.hook.is_type_macro(ty, &self.locals, self.fn_depth)? {
                            if depth == 256 {
                                return Err(self.hook.depth_error(ty.span));
                            }
                            *ty = self.hook.expand_type(ty, &self.locals, self.fn_depth)?;
                            tasks.push(Task::Ty(ptr, depth + 1));
                            continue;
                        }
                        match &mut ty.kind {
                            ast::TypeKind::Named { args, .. }
                            | ast::TypeKind::Call { args, .. }
                            | ast::TypeKind::Tuple(args)
                            | ast::TypeKind::Union(args)
                            | ast::TypeKind::Inter(args)
                            | ast::TypeKind::Diff(args) => {
                                tasks.extend(args.iter_mut().rev().map(|a| Task::Ty(a, depth)));
                            }
                            ast::TypeKind::Fn {
                                params,
                                ret,
                                err,
                                env,
                            } => {
                                if let Some(env) = env {
                                    tasks.push(Task::Ty(&mut **env, depth));
                                }
                                if let Some(err) = err {
                                    tasks.push(Task::Ty(&mut **err, depth));
                                }
                                tasks.push(Task::Ty(&mut **ret, depth));
                                tasks.extend(
                                    params.iter_mut().rev().map(|p| Task::Ty(&mut p.ty, depth)),
                                );
                            }
                            ast::TypeKind::Lit(_)
                            | ast::TypeKind::ValueOf { .. }
                            | ast::TypeKind::Hole => {}
                        }
                    }
                    Task::Pattern(ptr, depth) => {
                        let pattern = &mut *ptr;
                        match &mut pattern.kind {
                            PatternKind::Named { pattern, .. } => {
                                tasks.push(Task::Pattern(&mut **pattern, depth))
                            }
                            PatternKind::Constructor { args, .. } | PatternKind::Tuple(args) => {
                                tasks
                                    .extend(args.iter_mut().rev().map(|p| Task::Pattern(p, depth)));
                            }
                            PatternKind::List { items, .. } => {
                                tasks.extend(
                                    items.iter_mut().rev().map(|p| Task::Pattern(p, depth)),
                                );
                            }
                            PatternKind::BitArray(segments) => {
                                tasks.push(Task::Restore(self.locals.len(), self.fn_depth));
                                tasks.extend(
                                    segments
                                        .iter_mut()
                                        .rev()
                                        .map(|s| Task::BitSegment(s, depth)),
                                );
                            }
                            PatternKind::Typed { ty, .. } => tasks.push(Task::Ty(ty, depth)),
                            PatternKind::Quote { captures, .. } => {
                                tasks.extend(
                                    captures
                                        .iter_mut()
                                        .rev()
                                        .map(|(_, p)| Task::Pattern(p, depth)),
                                );
                            }
                            PatternKind::Var(_)
                            | PatternKind::Discard(_)
                            | PatternKind::Int { .. }
                            | PatternKind::Float { .. }
                            | PatternKind::String(_)
                            | PatternKind::Prefix { .. } => {}
                        }
                    }
                    Task::BitSegment(ptr, depth) => {
                        let segment = &mut *ptr;
                        tasks.push(Task::BindPattern(&mut segment.value));
                        tasks.push(Task::Pattern(&mut segment.value, depth));
                        if let Some(format) = &mut segment.format
                            && let Some(arg) = &mut format.arg
                        {
                            tasks.push(Task::Expr(&mut **arg, depth));
                        }
                    }
                    Task::BindPattern(ptr) => binders(&*ptr, &mut self.locals, None),
                    Task::Block(ptr, depth) => {
                        let block = &mut *ptr;
                        tasks.push(Task::Restore(self.locals.len(), self.fn_depth));
                        tasks.extend(block.stmts.iter_mut().rev().map(|s| Task::Stmt(s, depth)));
                    }
                    Task::Stmt(ptr, depth) => match &mut *ptr {
                        Stmt::Bind(binding) => {
                            tasks.push(Task::BindValue(binding, depth));
                            tasks.push(Task::Pattern(&mut binding.pattern, depth));
                        }
                        Stmt::Env { ty, value, .. } => {
                            tasks.push(Task::Expr(value, depth));
                            tasks.push(Task::Ty(ty, depth));
                        }
                        Stmt::Expr(value) => tasks.push(Task::Expr(value, depth)),
                    },
                    Task::BindValue(ptr, depth) => {
                        let binding = &mut *ptr;
                        let before = self.locals.len();
                        if self
                            .hook
                            .is_direct(&binding.value, &self.locals, self.fn_depth)
                        {
                            binders(&binding.pattern, &mut self.locals, Some(self.fn_depth));
                        }
                        tasks.push(Task::BindDone(ptr, before));
                        tasks.push(Task::Expr(&mut binding.value, depth));
                    }
                    Task::BindDone(ptr, before) => {
                        self.locals.truncate(before);
                        binders(&(*ptr).pattern, &mut self.locals, None);
                    }
                    Task::FunctionEnter(ptr, depth) => {
                        let function = &mut *ptr;
                        tasks.push(Task::Restore(self.locals.len(), self.fn_depth));
                        self.fn_depth += 1;
                        for param in &function.params {
                            match param {
                                ast::Param::Pattern(pattern) => {
                                    binders(pattern, &mut self.locals, None)
                                }
                                ast::Param::Env { name, .. } => self.locals.push(Local {
                                    name: name.name.clone(),
                                    span: name.span,
                                    fn_only: None,
                                }),
                            }
                        }
                        tasks.push(Task::Block(&mut function.body, depth));
                    }
                    Task::Clause(ptr, depth, function) => {
                        let clause = &mut *ptr;
                        tasks.push(Task::ClauseEnter(ptr, depth, function, self.locals.len()));
                        tasks.extend(
                            clause
                                .patterns
                                .iter_mut()
                                .rev()
                                .map(|p| Task::Pattern(p, depth)),
                        );
                    }
                    Task::ClauseEnter(ptr, depth, function, start) => {
                        let clause = &mut *ptr;
                        tasks.push(Task::Restore(start, self.fn_depth));
                        if function {
                            self.fn_depth += 1;
                        }
                        if let Some(first) = clause.patterns.first() {
                            binders(first, &mut self.locals, None);
                        }
                        tasks.push(Task::Expr(&mut clause.body, depth));
                        if let Some(guard) = &mut clause.guard {
                            tasks.push(Task::Expr(guard, depth));
                        }
                    }
                    Task::Restore(len, fn_depth) => {
                        self.locals.truncate(len);
                        self.fn_depth = fn_depth;
                    }
                }
            }
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
