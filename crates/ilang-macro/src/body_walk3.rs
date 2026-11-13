//! `body_walk` on the tree of syntax three: outside-in traversal of
//! expression and type macro positions.
//!
//! The hook decides whether a call resolves to a macro and returns its result.
//! Arguments are not traversed before that decision. Local frames carry source
//! spans so the hook can compare scopes rather than only written names.
//!
//! The scopes, the function depth and the order of the visits are those of
//! `body_walk` on the lowered tree (`syn3::lower`), form by form:
//!
//! | Form | Old form | Rule |
//! |---|---|---|
//! | `Chain([f])`, `f` plain | `Fn` | patterns, service types, result type; then one function deeper with the binders of the patterns and then the service names; the body block |
//! | `Chain`, one position, no need or result | `Clauses` | each branch: its patterns; then one function deeper with the binders of its first choice; guard, body |
//! | `Chain`, other | `Fn` over `Case` | the service types and result types of all branches; one function deeper with the service names of all branches (first of each name); each branch as a clause of a match |
//! | `Pipe` into a chain of one-position branches with no need, result, type parameter or key | `Case` | the left side; each branch as a clause, at the same function depth |
//! | `Pipe`, other | `Binary(Pipe)` | left, right |
//! | `Provide` | block of `Stmt::Env` | each service, then the target |
//! | `Function.services` | `Param::Env` | the name is a binder after the pattern binders |
//! | `And` | `Named` | the names first, then the binders of the other part |
//! | key in a constructor pattern | `Named` chain ending in the label | the label is a binder, after the names |
//! | `FnType` | `Fn { params, ret, err, env }` | parameter items, result, the error of a `Fail` need, the other needs |
//!
//! Differences from the old walker, none of which a macro can see:
//! - the locals that the lowering invents (`%arg1`, `%service1`, `%field1`)
//!   are not in the scope: no written or generated name is equal to one;
//! - a pipe into a chain is always read as the match when its branches have
//!   the shape of one. The lowering of a Lisp file reads it as a pipe into a
//!   clause block, one function deeper; the depth only hides the binders of
//!   a direct binding inside its own value, and such a value holds no match;
//! - parts that the lowering drops with an error (a second result type, the
//!   type of a service that another branch names, a labelled service) are
//!   visited.
//!
//! The walk is a plain recursion. [`expressions`] and [`type_definition`]
//! run it on a deep stack, because a tree made by a macro can be deeper than
//! the host stack.
use ilang_syntax::lisp::expansion3::{clause_block, plain};
use ilang_syntax::span::Span;
use ilang_syntax::syn3::ast::{
    Binding, Block, DItem, DPItem, Expr, ExprKind, Function, Ident, ItemKind, Module, Pattern,
    PatternKind, Stmt, TItem, Type, TypeDef, TypeKind, TypeRhs,
};
use std::collections::HashSet;

pub use crate::body_walk::Local;

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
        ty: &Type,
        locals: &[Local],
        fn_depth: u32,
    ) -> Result<bool, Self::Error>;
    fn expand_type(
        &mut self,
        ty: &Type,
        locals: &[Local],
        fn_depth: u32,
    ) -> Result<Type, Self::Error>;
    fn depth_error(&mut self, span: Span) -> Self::Error;
    /// Whether a binding has a direct value (one made without running any
    /// code); module-qualified names require the same scope-aware lookup used
    /// by name resolution.
    fn is_direct(&mut self, expression: &Expr, locals: &[Local], fn_depth: u32) -> bool;
}

/// Expands one module's expression positions. A returned macro expression is
/// visited before any of its children, so an outer macro sees written arguments.
pub fn expressions<H: Hook + Send>(module: &mut Module, hook: &mut H) -> Result<(), H::Error>
where
    H::Error: Send,
{
    ilang_syntax::syn3::deep::deep(|| expressions_on_this_stack(module, hook))
}

/// [`expressions`] for a caller that is already on a deep stack.
pub fn expressions_on_this_stack<H: Hook>(
    module: &mut Module,
    hook: &mut H,
) -> Result<(), H::Error> {
    let mut walker = Walker {
        hook,
        locals: Vec::new(),
        fn_depth: 0,
    };
    for item in &mut module.items {
        match &mut item.kind {
            ItemKind::Bind(binding) => {
                walker.pattern(&mut binding.pattern, 0)?;
                walker.expr(&mut binding.value, 0)?;
            }
            ItemKind::Macro { .. } => {
                panic!("item expansion must finish before body expansion")
            }
            ItemKind::Decl { ty, .. } => walker.ty(ty, 0)?,
            ItemKind::TypeDef(_) => {}
        }
    }
    Ok(())
}

/// Expands type-position macros in one written type definition during the item
/// phase. Bindings, declarations and their type positions are left for bodies.
pub fn type_definition<H: Hook + Send>(
    definition: &mut TypeDef,
    hook: &mut H,
) -> Result<(), H::Error>
where
    H::Error: Send,
{
    ilang_syntax::syn3::deep::deep(|| type_definition_on_this_stack(definition, hook))
}

/// [`type_definition`] for a caller that is already on a deep stack.
pub fn type_definition_on_this_stack<H: Hook>(
    definition: &mut TypeDef,
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
    match &mut definition.rhs {
        TypeRhs::Opaque => {}
        TypeRhs::Type(ty) => walker.ty(ty, 0)?,
        TypeRhs::Data(items) => walker.titems(items, 0)?,
    }
    Ok(())
}

/// A pipe into this chain is a match (§2.11): one position per branch, and
/// nothing that only a function has.
pub fn is_match(fs: &[Function]) -> bool {
    fs.iter().all(|f| {
        f.params.len() == 1
            && f.services.is_none()
            && f.ret.is_none()
            && f.generics.is_empty()
            && f.params[0].key.is_none()
    })
}

/// The type `E` of a need `Fail(E)`: the error channel of a function type.
fn fail_need(ty: &mut Type) -> Option<&mut Type> {
    match &mut ty.kind {
        TypeKind::Named {
            module: None,
            name,
            args,
        } if name.name == "Fail" && args.len() == 1 => args.first_mut(),
        _ => None,
    }
}

fn is_fail_need(ty: &Type) -> bool {
    matches!(&ty.kind, TypeKind::Named { module: None, name, args }
        if name.name == "Fail" && args.len() == 1)
}

struct Walker<'a, H> {
    hook: &'a mut H,
    locals: Vec<Local>,
    fn_depth: u32,
}

fn collected(pattern: &Pattern) -> Vec<Local> {
    let mut out = Vec::new();
    binders(pattern, &mut out);
    out
}

fn binders(pattern: &Pattern, out: &mut Vec<Local>) {
    for_each_binder(pattern, |name, span| {
        out.push(Local {
            name: name.into(),
            span,
            fn_only: None,
        })
    });
}

fn local(name: &Ident) -> Local {
    Local {
        name: name.name.clone(),
        span: name.span.0,
        fn_only: None,
    }
}

impl<H: Hook> Walker<'_, H> {
    fn restore(&mut self, (len, fn_depth): (usize, u32)) {
        self.locals.truncate(len);
        self.fn_depth = fn_depth;
    }
    fn mark(&self) -> (usize, u32) {
        (self.locals.len(), self.fn_depth)
    }

    fn expr(&mut self, expr: &mut Expr, mut depth: u16) -> Result<(), H::Error> {
        while self.hook.is_macro(expr, &self.locals, self.fn_depth)? {
            if depth == 256 {
                return Err(self.hook.depth_error(expr.span.0));
            }
            *expr = self.hook.expand(expr, &self.locals, self.fn_depth)?;
            depth += 1;
        }
        match &mut expr.kind {
            ExprKind::Quote(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Name { .. } => {}
            ExprKind::Interp(parts) => {
                for part in parts {
                    if let ilang_syntax::syn3::ast::InterpPart::Expr(e) = part {
                        self.expr(e, depth)?;
                    }
                }
            }
            ExprKind::Data(items) => self.ditems(items, depth)?,
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
            ExprKind::Chain(fs) => self.chain(fs, depth)?,
            ExprKind::Call { callee, args } => {
                self.expr(callee, depth)?;
                self.ditems(args, depth)?;
            }
            ExprKind::Provide { target, services } => {
                match &mut services.kind {
                    ExprKind::Data(items) => self.ditems(items, depth)?,
                    _ => self.expr(services, depth)?,
                }
                self.expr(target, depth)?;
            }
            ExprKind::Field { target, .. }
            | ExprKind::Index { target, .. }
            | ExprKind::Unary { expr: target, .. } => self.expr(target, depth)?,
            ExprKind::Binary { lhs, rhs, .. } => {
                self.expr(lhs, depth)?;
                self.expr(rhs, depth)?;
            }
            ExprKind::Pipe { lhs, rhs } => {
                self.expr(lhs, depth)?;
                match &mut rhs.kind {
                    ExprKind::Chain(fs) if is_match(fs) => {
                        for f in fs {
                            self.clause(f, depth, false)?;
                        }
                    }
                    _ => self.expr(rhs, depth)?,
                }
            }
        }
        Ok(())
    }

    fn ditems(&mut self, items: &mut [DItem], depth: u16) -> Result<(), H::Error> {
        for item in items {
            match item {
                DItem::Pos(e) | DItem::Label(_, e) | DItem::Spread(e) => self.expr(e, depth)?,
            }
        }
        Ok(())
    }

    fn titems(&mut self, items: &mut [TItem], depth: u16) -> Result<(), H::Error> {
        for item in items {
            match item {
                TItem::Pos(t) | TItem::Label(_, t) | TItem::Spread(t) => self.ty(t, depth)?,
            }
        }
        Ok(())
    }

    /// A chain as a value.
    fn chain(&mut self, fs: &mut [Function], depth: u16) -> Result<(), H::Error> {
        if let [f] = fs
            && plain(f)
        {
            return self.function(f, depth);
        }
        if clause_block(fs) {
            for f in fs {
                self.clause(f, depth, true)?;
            }
            return Ok(());
        }
        // Several positions, or a written need or result: one function
        // whose body is a match over its parameters.
        for f in fs.iter_mut() {
            for service in f.services.iter_mut().flatten() {
                self.ty(&mut service.ty, depth)?;
            }
        }
        for f in fs.iter_mut() {
            if let Some(ret) = &mut f.ret {
                self.ty(ret, depth)?;
            }
        }
        let mark = self.mark();
        self.fn_depth += 1;
        let mut seen = HashSet::new();
        for f in fs.iter() {
            for service in f.services.iter().flatten() {
                if seen.insert(service.name.name.as_str()) {
                    self.locals.push(local(&service.name));
                }
            }
        }
        for f in fs {
            self.clause(f, depth, false)?;
        }
        self.restore(mark);
        Ok(())
    }

    /// One plain function.
    fn function(&mut self, f: &mut Function, depth: u16) -> Result<(), H::Error> {
        let mut bindings = Vec::new();
        for param in &f.params {
            if let Some(pattern) = param.choices.first() {
                binders(pattern, &mut bindings);
            }
        }
        bindings.extend(f.services.iter().flatten().map(|s| local(&s.name)));
        for param in &mut f.params {
            for pattern in &mut param.choices {
                self.pattern(pattern, depth)?;
            }
        }
        for service in f.services.iter_mut().flatten() {
            self.ty(&mut service.ty, depth)?;
        }
        if let Some(ret) = &mut f.ret {
            self.ty(ret, depth)?;
        }
        let mark = self.mark();
        self.fn_depth += 1;
        self.locals.extend(bindings);
        self.block(&mut f.body, depth)?;
        self.restore(mark);
        Ok(())
    }

    /// A branch as a clause: the binders of the first choice of each
    /// position cover the guard and the body. `function` is set for a
    /// branch of a clause block, which is a function of its own.
    fn clause(&mut self, f: &mut Function, depth: u16, function: bool) -> Result<(), H::Error> {
        let mark = self.mark();
        let mut bindings = Vec::new();
        for param in &f.params {
            if let Some(pattern) = param.choices.first() {
                binders(pattern, &mut bindings);
            }
        }
        for param in &mut f.params {
            for pattern in &mut param.choices {
                self.pattern(pattern, depth)?;
            }
        }
        if function {
            self.fn_depth += 1;
        }
        self.locals.extend(bindings);
        if let Some(guard) = &mut f.guard {
            self.expr(guard, depth)?;
        }
        self.block(&mut f.body, depth)?;
        self.restore(mark);
        Ok(())
    }

    fn block(&mut self, block: &mut Block, depth: u16) -> Result<(), H::Error> {
        let mark = self.mark();
        for stmt in &mut block.stmts {
            match stmt {
                Stmt::Bind(binding) => self.bind(binding, depth)?,
                Stmt::Expr(value) => self.expr(value, depth)?,
            }
        }
        self.restore(mark);
        Ok(())
    }

    fn bind(&mut self, binding: &mut Binding, depth: u16) -> Result<(), H::Error> {
        // The binders can be gathered before the pattern is visited: the
        // only expansions inside a pattern are types and format arguments,
        // which cannot change its names.
        let bindings = collected(&binding.pattern);
        self.pattern(&mut binding.pattern, depth)?;
        let before = self.locals.len();
        if self
            .hook
            .is_direct(&binding.value, &self.locals, self.fn_depth)
        {
            let level = self.fn_depth;
            self.locals.extend(bindings.iter().cloned().map(|mut b| {
                b.fn_only = Some(level);
                b
            }));
        }
        self.expr(&mut binding.value, depth)?;
        self.locals.truncate(before);
        self.locals.extend(bindings);
        Ok(())
    }

    fn ty(&mut self, ty: &mut Type, mut depth: u16) -> Result<(), H::Error> {
        while self.hook.is_type_macro(ty, &self.locals, self.fn_depth)? {
            if depth == 256 {
                return Err(self.hook.depth_error(ty.span.0));
            }
            *ty = self.hook.expand_type(ty, &self.locals, self.fn_depth)?;
            depth += 1;
        }
        match &mut ty.kind {
            TypeKind::Named { args, .. }
            | TypeKind::Call { args, .. }
            | TypeKind::Union(args)
            | TypeKind::Inter(args)
            | TypeKind::Diff(args) => {
                for arg in args {
                    self.ty(arg, depth)?;
                }
            }
            TypeKind::Data(items) => self.titems(items, depth)?,
            TypeKind::Fn(f) => {
                match &mut f.param.kind {
                    TypeKind::Data(items) => self.titems(items, depth)?,
                    _ => self.ty(&mut f.param, depth)?,
                }
                self.ty(&mut f.ret, depth)?;
                // The needs: the error of a `Fail` entry, then the others.
                if let Some(services) = &mut f.services {
                    if let TypeKind::Data(items) = &mut services.kind {
                        for item in items.iter_mut() {
                            if let TItem::Pos(need) = item
                                && let Some(error) = fail_need(need)
                            {
                                self.ty(error, depth)?;
                            }
                        }
                        for item in items.iter_mut() {
                            match item {
                                TItem::Pos(need) if is_fail_need(need) => {}
                                TItem::Pos(need) | TItem::Label(_, need) | TItem::Spread(need) => {
                                    self.ty(need, depth)?
                                }
                            }
                        }
                    } else {
                        self.ty(services, depth)?;
                    }
                }
            }
            TypeKind::Lit(_) | TypeKind::ValueOf { .. } | TypeKind::Hole => {}
        }
        Ok(())
    }

    fn pattern(&mut self, pattern: &mut Pattern, depth: u16) -> Result<(), H::Error> {
        match &mut pattern.kind {
            PatternKind::Quote(quote) => {
                for (_, capture) in &mut quote.captures {
                    self.pattern(capture, depth)?;
                }
            }
            PatternKind::Typed { ty, .. } => self.ty(ty, depth)?,
            PatternKind::Ctor {
                args: Some(items), ..
            }
            | PatternKind::Data(items) => {
                for item in items {
                    match item {
                        DPItem::Pos(p) | DPItem::Key(_, p) => self.pattern(p, depth)?,
                        DPItem::Rest(_) => {}
                    }
                }
            }
            PatternKind::List { items, .. } | PatternKind::And(items) => {
                for item in items {
                    self.pattern(item, depth)?;
                }
            }
            PatternKind::BitArray(segments) => {
                // The binders of a segment cover the sizes of the later
                // segments, and nothing after the bit array.
                let mark = self.mark();
                for segment in segments {
                    let bindings = collected(&segment.value);
                    if let Some(format) = &mut segment.format
                        && let Some(arg) = &mut format.arg
                    {
                        self.expr(arg, depth)?;
                    }
                    self.pattern(&mut segment.value, depth)?;
                    self.locals.extend(bindings);
                }
                self.restore(mark);
            }
            PatternKind::Ctor { args: None, .. }
            | PatternKind::Var(_)
            | PatternKind::Discard(_)
            | PatternKind::Int { .. }
            | PatternKind::Float { .. }
            | PatternKind::String(_)
            | PatternKind::Prefix { .. } => {}
        }
        Ok(())
    }
}

/// The names that a pattern binds, in the order of the old tree. This is a
/// work list and not a recursion: it is called on patterns of any depth
/// from places that are not on a deep stack.
pub fn for_each_binder<'a>(pattern: &'a Pattern, mut visit: impl FnMut(&'a str, Span)) {
    enum Task<'a> {
        Pattern(&'a Pattern),
        Keyed(&'a Ident, &'a Pattern),
        Binder(&'a str, Span),
    }
    fn items<'a>(items: &'a [DPItem], pending: &mut Vec<Task<'a>>) {
        pending.extend(items.iter().rev().filter_map(|item| match item {
            DPItem::Pos(p) => Some(Task::Pattern(p)),
            DPItem::Key(label, p) => Some(Task::Keyed(label, p)),
            // `..name` is an error of the lowering today; the name is bound.
            DPItem::Rest(Some(rest)) if !rest.discard => {
                Some(Task::Binder(&rest.name, rest.span.0))
            }
            DPItem::Rest(_) => None,
        }));
    }
    let mut pending = vec![Task::Pattern(pattern)];
    while let Some(task) = pending.pop() {
        let pattern = match task {
            Task::Pattern(pattern) => pattern,
            Task::Binder(name, span) => {
                visit(name, span);
                continue;
            }
            // `key = pattern` in a constructor pattern is the old tree's
            // chain of names that ends in the label: the label is bound.
            Task::Keyed(label, pattern) => {
                let single = std::slice::from_ref(pattern);
                let parts = match &pattern.kind {
                    PatternKind::And(parts) => parts.as_slice(),
                    PatternKind::Discard(_) => &[],
                    _ => single,
                };
                let core = parts
                    .iter()
                    .find(|p| !matches!(p.kind, PatternKind::Var(_) | PatternKind::Discard(_)));
                let names = parts.iter().filter_map(|p| match &p.kind {
                    PatternKind::Var(name) => Some((name.as_str(), p.span.0)),
                    _ => None,
                });
                match core.map(|core| (&core.kind, core)) {
                    Some((PatternKind::Typed { binder, .. }, _)) => {
                        if binder.name != label.name && !binder.discard {
                            visit(&binder.name, binder.span.0);
                        }
                        names.for_each(|(name, span)| visit(name, span));
                        visit(&label.name, label.span.0);
                    }
                    // A pattern inside a labelled field is an error of the
                    // lowering today, which then binds no label.
                    Some(_) => {
                        names.for_each(|(name, span)| visit(name, span));
                        pending.extend(
                            parts
                                .iter()
                                .rev()
                                .filter(|p| {
                                    !matches!(
                                        p.kind,
                                        PatternKind::Var(_) | PatternKind::Discard(_)
                                    )
                                })
                                .map(Task::Pattern),
                        );
                    }
                    None => {
                        names
                            .filter(|(name, _)| *name != label.name)
                            .for_each(|(name, span)| visit(name, span));
                        visit(&label.name, label.span.0);
                    }
                }
                continue;
            }
        };
        match &pattern.kind {
            PatternKind::Quote(quote) => pending.extend(
                quote
                    .captures
                    .iter()
                    .rev()
                    .map(|(_, pattern)| Task::Pattern(pattern)),
            ),
            PatternKind::Var(name) => visit(name, pattern.span.0),
            PatternKind::Typed { binder, .. } if !binder.discard => {
                visit(&binder.name, binder.span.0)
            }
            PatternKind::Prefix { rest, .. } if !rest.discard => visit(&rest.name, rest.span.0),
            PatternKind::Ctor {
                args: Some(args), ..
            }
            | PatternKind::Data(args) => items(args, &mut pending),
            PatternKind::List { items, rest } => {
                if let Some(Some(rest)) = rest
                    && !rest.discard
                {
                    pending.push(Task::Binder(&rest.name, rest.span.0));
                }
                pending.extend(items.iter().rev().map(Task::Pattern));
            }
            PatternKind::BitArray(segments) => pending.extend(
                segments
                    .iter()
                    .rev()
                    .map(|segment| Task::Pattern(&segment.value)),
            ),
            // `p1 = p2 = …`: the names are the old tree's `Named` around
            // the one other part, so they come first.
            PatternKind::And(parts) => {
                pending.extend(
                    parts
                        .iter()
                        .rev()
                        .filter(|p| !matches!(p.kind, PatternKind::Var(_)))
                        .map(Task::Pattern),
                );
                for part in parts {
                    if let PatternKind::Var(name) = &part.kind {
                        visit(name, part.span.0);
                    }
                }
            }
            _ => {}
        }
    }
}
