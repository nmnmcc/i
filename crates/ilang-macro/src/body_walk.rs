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
    /// Whether a binding has a direct value (one made without running any
    /// code); module-qualified names require the same scope-aware lookup used
    /// by name resolution.
    fn is_direct(&mut self, expression: &Expr, locals: &[Local], fn_depth: u32) -> bool;
}

/// Expands one module's expression positions. A returned macro expression is
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

/// Expands type-position macros in one written type definition during the item
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
// Each task owns a disjoint mutable borrow of its tree node. Binders can be
// gathered before visiting a pattern: the only expansions inside a pattern are
// type/format expressions, which cannot change that pattern's binder names.
// They are installed only by the later scope event, in the same order as the
// recursive traversal. This keeps both the tree and the scope stack safe while
// allowing arbitrarily deep syntax without the host call stack.
enum Task<'a> {
    Expr(&'a mut Expr, u16),
    Ty(&'a mut ast::TypeExpr, u16),
    Pattern(&'a mut Pattern, u16),
    Block(&'a mut ast::Block, u16),
    Stmt(&'a mut Stmt, u16),
    BindValue {
        value: &'a mut Expr,
        bindings: Vec<Local>,
        depth: u16,
    },
    BindDone {
        bindings: Vec<Local>,
        before: usize,
    },
    FunctionEnter {
        body: &'a mut ast::Block,
        bindings: Vec<Local>,
        depth: u16,
    },
    Clause(&'a mut ast::Clause, u16, bool),
    ClauseEnter {
        body: &'a mut Expr,
        guard: Option<&'a mut Expr>,
        bindings: Vec<Local>,
        depth: u16,
        function: bool,
        start: usize,
    },
    BitSegment(&'a mut ast::Segment<Pattern>, u16),
    AddBindings(Vec<Local>),
    Restore(usize, u32),
}

fn collected(pattern: &Pattern) -> Vec<Local> {
    let mut out = Vec::new();
    binders(pattern, &mut out, None);
    out
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

    fn run<'a>(&mut self, first: Task<'a>) -> Result<(), H::Error> {
        let mut tasks = vec![first];
        while let Some(task) = tasks.pop() {
            match task {
                Task::Expr(expr, depth) => {
                    if self.hook.is_macro(expr, &self.locals, self.fn_depth)? {
                        if depth == 256 {
                            return Err(self.hook.depth_error(expr.span));
                        }
                        *expr = self.hook.expand(expr, &self.locals, self.fn_depth)?;
                        tasks.push(Task::Expr(expr, depth + 1));
                        continue;
                    }
                    match &mut expr.kind {
                        ExprKind::Quote(_)
                        | ExprKind::Int(_)
                        | ExprKind::Float(_)
                        | ExprKind::String(_)
                        | ExprKind::Name { .. } => {}
                        ExprKind::Tuple(items) => {
                            tasks.extend(items.iter_mut().rev().map(|e| Task::Expr(e, depth)))
                        }
                        ExprKind::Record { base, fields } => {
                            tasks.extend(
                                fields
                                    .iter_mut()
                                    .rev()
                                    .map(|(_, value)| Task::Expr(value, depth)),
                            );
                            if let Some(base) = base {
                                tasks.push(Task::Expr(base, depth));
                            }
                        }
                        ExprKind::List { items, rest } => {
                            if let Some(rest) = rest {
                                tasks.push(Task::Expr(rest, depth));
                            }
                            tasks.extend(items.iter_mut().rev().map(|e| Task::Expr(e, depth)));
                        }
                        ExprKind::BitArray(segments) => {
                            for segment in segments.iter_mut().rev() {
                                if let Some(format) = &mut segment.format
                                    && let Some(arg) = &mut format.arg
                                {
                                    tasks.push(Task::Expr(arg, depth));
                                }
                                tasks.push(Task::Expr(&mut segment.value, depth));
                            }
                        }
                        ExprKind::Block(block) => tasks.push(Task::Block(block, depth)),
                        ExprKind::Fn(function) => {
                            let mut bindings = Vec::new();
                            for param in &function.params {
                                match param {
                                    ast::Param::Pattern(pattern) => {
                                        binders(pattern, &mut bindings, None)
                                    }
                                    ast::Param::Env { name, .. } => bindings.push(Local {
                                        name: name.name.clone(),
                                        span: name.span,
                                        fn_only: None,
                                    }),
                                }
                            }
                            tasks.push(Task::FunctionEnter {
                                body: &mut function.body,
                                bindings,
                                depth,
                            });
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
                                    ast::Param::Pattern(pattern) => Task::Pattern(pattern, depth),
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
                            tasks.push(Task::Expr(subject, depth));
                        }
                        ExprKind::Clauses(clauses) => tasks.extend(
                            clauses
                                .iter_mut()
                                .rev()
                                .map(|c| Task::Clause(c, depth, true)),
                        ),
                        ExprKind::Call { callee, args } => {
                            tasks.extend(
                                args.iter_mut()
                                    .rev()
                                    .map(|a| Task::Expr(&mut a.value, depth)),
                            );
                            tasks.push(Task::Expr(callee, depth));
                        }
                        ExprKind::Field { target, .. } | ExprKind::TupleIndex { target, .. } => {
                            tasks.push(Task::Expr(target, depth))
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
                            tasks.push(Task::Expr(base, depth));
                            tasks.push(Task::Expr(constructor, depth));
                        }
                        ExprKind::Unary { expr, .. } => tasks.push(Task::Expr(expr, depth)),
                        ExprKind::Binary { lhs, rhs, .. } => {
                            tasks.push(Task::Expr(rhs, depth));
                            tasks.push(Task::Expr(lhs, depth));
                        }
                    }
                }
                Task::Ty(ty, depth) => {
                    if self.hook.is_type_macro(ty, &self.locals, self.fn_depth)? {
                        if depth == 256 {
                            return Err(self.hook.depth_error(ty.span));
                        }
                        *ty = self.hook.expand_type(ty, &self.locals, self.fn_depth)?;
                        tasks.push(Task::Ty(ty, depth + 1));
                        continue;
                    }
                    match &mut ty.kind {
                        ast::TypeKind::Named { args, .. }
                        | ast::TypeKind::Call { args, .. }
                        | ast::TypeKind::Tuple(args)
                        | ast::TypeKind::Union(args)
                        | ast::TypeKind::Inter(args)
                        | ast::TypeKind::Diff(args) => {
                            tasks.extend(args.iter_mut().rev().map(|a| Task::Ty(a, depth)))
                        }
                        ast::TypeKind::Record { base, fields, .. } => {
                            tasks
                                .extend(fields.iter_mut().rev().map(|(_, ty)| Task::Ty(ty, depth)));
                            if let Some(base) = base {
                                tasks.push(Task::Ty(base, depth));
                            }
                        }
                        ast::TypeKind::Fn {
                            params,
                            ret,
                            err,
                            env,
                        } => {
                            if let Some(env) = env {
                                tasks.push(Task::Ty(env, depth));
                            }
                            if let Some(err) = err {
                                tasks.push(Task::Ty(err, depth));
                            }
                            tasks.push(Task::Ty(ret, depth));
                            tasks.extend(
                                params.iter_mut().rev().map(|p| Task::Ty(&mut p.ty, depth)),
                            );
                        }
                        ast::TypeKind::Lit(_)
                        | ast::TypeKind::ValueOf { .. }
                        | ast::TypeKind::Hole => {}
                    }
                }
                Task::Pattern(pattern, depth) => match &mut pattern.kind {
                    PatternKind::Named { pattern, .. } => tasks.push(Task::Pattern(pattern, depth)),
                    PatternKind::Constructor { args, .. } | PatternKind::Tuple(args) => {
                        tasks.extend(args.iter_mut().rev().map(|p| Task::Pattern(p, depth)))
                    }
                    PatternKind::List { items, .. } => {
                        tasks.extend(items.iter_mut().rev().map(|p| Task::Pattern(p, depth)))
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
                    PatternKind::Quote { captures, .. } => tasks.extend(
                        captures
                            .iter_mut()
                            .rev()
                            .map(|(_, p)| Task::Pattern(p, depth)),
                    ),
                    PatternKind::Var(_)
                    | PatternKind::Discard(_)
                    | PatternKind::Int { .. }
                    | PatternKind::Float { .. }
                    | PatternKind::String(_)
                    | PatternKind::Prefix { .. } => {}
                },
                Task::BitSegment(segment, depth) => {
                    let bindings = collected(&segment.value);
                    tasks.push(Task::AddBindings(bindings));
                    tasks.push(Task::Pattern(&mut segment.value, depth));
                    if let Some(format) = &mut segment.format
                        && let Some(arg) = &mut format.arg
                    {
                        tasks.push(Task::Expr(arg, depth));
                    }
                }
                Task::AddBindings(bindings) => self.locals.extend(bindings),
                Task::Block(block, depth) => {
                    tasks.push(Task::Restore(self.locals.len(), self.fn_depth));
                    tasks.extend(block.stmts.iter_mut().rev().map(|s| Task::Stmt(s, depth)));
                }
                Task::Stmt(stmt, depth) => match stmt {
                    Stmt::Bind(binding) => {
                        let bindings = collected(&binding.pattern);
                        tasks.push(Task::BindValue {
                            value: &mut binding.value,
                            bindings,
                            depth,
                        });
                        tasks.push(Task::Pattern(&mut binding.pattern, depth));
                    }
                    Stmt::Env { ty, value, .. } => {
                        tasks.push(Task::Expr(value, depth));
                        tasks.push(Task::Ty(ty, depth));
                    }
                    Stmt::Expr(value) => tasks.push(Task::Expr(value, depth)),
                },
                Task::BindValue {
                    value,
                    mut bindings,
                    depth,
                } => {
                    let before = self.locals.len();
                    if self.hook.is_direct(value, &self.locals, self.fn_depth) {
                        for binding in &mut bindings {
                            binding.fn_only = Some(self.fn_depth);
                        }
                        self.locals.extend(bindings.iter().cloned());
                        for binding in &mut bindings {
                            binding.fn_only = None;
                        }
                    }
                    tasks.push(Task::BindDone { bindings, before });
                    tasks.push(Task::Expr(value, depth));
                }
                Task::BindDone { bindings, before } => {
                    self.locals.truncate(before);
                    self.locals.extend(bindings);
                }
                Task::FunctionEnter {
                    body,
                    bindings,
                    depth,
                } => {
                    tasks.push(Task::Restore(self.locals.len(), self.fn_depth));
                    self.fn_depth += 1;
                    self.locals.extend(bindings);
                    tasks.push(Task::Block(body, depth));
                }
                Task::Clause(clause, depth, function) => {
                    let bindings = clause.patterns.first().map(collected).unwrap_or_default();
                    tasks.push(Task::ClauseEnter {
                        body: &mut clause.body,
                        guard: clause.guard.as_mut(),
                        bindings,
                        depth,
                        function,
                        start: self.locals.len(),
                    });
                    tasks.extend(
                        clause
                            .patterns
                            .iter_mut()
                            .rev()
                            .map(|p| Task::Pattern(p, depth)),
                    );
                }
                Task::ClauseEnter {
                    body,
                    guard,
                    bindings,
                    depth,
                    function,
                    start,
                } => {
                    tasks.push(Task::Restore(start, self.fn_depth));
                    if function {
                        self.fn_depth += 1;
                    }
                    self.locals.extend(bindings);
                    tasks.push(Task::Expr(body, depth));
                    if let Some(guard) = guard {
                        tasks.push(Task::Expr(guard, depth));
                    }
                }
                Task::Restore(len, fn_depth) => {
                    self.locals.truncate(len);
                    self.fn_depth = fn_depth;
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
