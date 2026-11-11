//! Stack-safe ownership operations for recursively nested resolved trees.
use crate::hir::*;
use crate::hir_trampoline::{self as trampoline, Scheduler};

async fn ty<'a>(s: Scheduler<'a>, t: &'a Type) -> Type {
    let kind = match &t.kind {
        TypeKind::Lit(x) => TypeKind::Lit(x.clone()),
        TypeKind::Of {
            value,
            site,
            projection,
        } => TypeKind::Of {
            value: *value,
            site: *site,
            projection: *projection,
        },
        TypeKind::Named { ty: name, args } => TypeKind::Named {
            ty: *name,
            args: tys(&s, args).await,
        },
        TypeKind::Var(x) => TypeKind::Var(*x),
        TypeKind::Hole => TypeKind::Hole,
        TypeKind::Tuple(xs) => TypeKind::Tuple(tys(&s, xs).await),
        TypeKind::Union(xs) => TypeKind::Union(tys(&s, xs).await),
        TypeKind::Inter(xs) => TypeKind::Inter(tys(&s, xs).await),
        TypeKind::Diff(xs) => TypeKind::Diff(tys(&s, xs).await),
        TypeKind::Fn {
            params,
            ret,
            err,
            env,
        } => {
            let mut ps = Vec::with_capacity(params.len());
            for p in params {
                ps.push(FnParamType {
                    label: p.label.clone(),
                    ty: s.call(|s| ty(s, &p.ty)).await,
                });
            }
            TypeKind::Fn {
                params: ps,
                ret: Box::new(s.call(|s| ty(s, ret)).await),
                err: opt_ty(&s, err).await,
                env: opt_ty(&s, env).await,
            }
        }
        TypeKind::Error => TypeKind::Error,
    };
    Type { kind, span: t.span }
}
async fn tys<'a>(s: &Scheduler<'a>, xs: &'a [Type]) -> Vec<Type> {
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        out.push(s.call(|s| ty(s, x)).await);
    }
    out
}
async fn opt_ty<'a>(s: &Scheduler<'a>, t: &'a Option<Box<Type>>) -> Option<Box<Type>> {
    match t {
        Some(t) => Some(Box::new(s.call(|s| ty(s, t)).await)),
        None => None,
    }
}
async fn fmt<'a>(s: &Scheduler<'a>, f: &'a Option<Format>) -> Option<Format> {
    match f {
        Some(f) => Some(Format {
            name: f.name.clone(),
            arg: match &f.arg {
                Some(e) => Some(Box::new(s.call(|s| expr(s, e)).await)),
                None => None,
            },
        }),
        None => None,
    }
}
async fn pattern<'a>(s: Scheduler<'a>, p: &'a Pattern) -> Pattern {
    let kind = match &p.kind {
        PatternKind::Var(b) => PatternKind::Var(b.clone()),
        PatternKind::Discard => PatternKind::Discard,
        PatternKind::Typed { binder, ty: t } => PatternKind::Typed {
            binder: binder.clone(),
            ty: s.call(|s| ty(s, t)).await,
        },
        PatternKind::Named { binder, pattern: p } => PatternKind::Named {
            binder: binder.clone(),
            pattern: Box::new(s.call(|s| pattern(s, p)).await),
        },
        PatternKind::Int { negative, value } => PatternKind::Int {
            negative: *negative,
            value: value.clone(),
        },
        PatternKind::Float { negative, value } => PatternKind::Float {
            negative: *negative,
            value: *value,
        },
        PatternKind::String(x) => PatternKind::String(x.clone()),
        PatternKind::Prefix { prefix, rest } => PatternKind::Prefix {
            prefix: prefix.clone(),
            rest: rest.clone(),
        },
        PatternKind::Constructor { ctor, args, spread } => PatternKind::Constructor {
            ctor: *ctor,
            args: pats(&s, args).await,
            spread: *spread,
        },
        PatternKind::Tuple(xs) => PatternKind::Tuple(pats(&s, xs).await),
        PatternKind::List { items, rest } => PatternKind::List {
            items: pats(&s, items).await,
            rest: rest.clone(),
        },
        PatternKind::BitArray(xs) => {
            let mut out = Vec::with_capacity(xs.len());
            for x in xs {
                out.push(Segment {
                    value: s.call(|s| pattern(s, &x.value)).await,
                    format: fmt(&s, &x.format).await,
                    span: x.span,
                });
            }
            PatternKind::BitArray(out)
        }
        PatternKind::Error(xs) => PatternKind::Error(pats(&s, xs).await),
    };
    Pattern { kind, span: p.span }
}
async fn pats<'a>(s: &Scheduler<'a>, xs: &'a [Pattern]) -> Vec<Pattern> {
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        out.push(s.call(|s| pattern(s, x)).await);
    }
    out
}
async fn exprs<'a>(s: &Scheduler<'a>, xs: &'a [Expr]) -> Vec<Expr> {
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        out.push(s.call(|s| expr(s, x)).await);
    }
    out
}
async fn block<'a>(s: &Scheduler<'a>, b: &'a Block) -> Block {
    let mut stmts = Vec::with_capacity(b.stmts.len());
    for st in &b.stmts {
        stmts.push(match st {
            Stmt::Bind(b) => Stmt::Bind(Binding {
                pattern: s.call(|s| pattern(s, &b.pattern)).await,
                value: s.call(|s| expr(s, &b.value)).await,
                direct: b.direct,
                ty_vars: b.ty_vars.clone(),
                span: b.span,
            }),
            Stmt::Env { ty: t, value, span } => Stmt::Env {
                ty: s.call(|s| ty(s, t)).await,
                value: s.call(|s| expr(s, value)).await,
                span: *span,
            },
            Stmt::Expr(e) => Stmt::Expr(s.call(|s| expr(s, e)).await),
        });
    }
    Block {
        stmts,
        span: b.span,
    }
}
async fn clauses<'a>(s: &Scheduler<'a>, xs: &'a [Clause]) -> Vec<Clause> {
    let mut out = Vec::with_capacity(xs.len());
    for c in xs {
        out.push(Clause {
            patterns: pats(s, &c.patterns).await,
            guard: match &c.guard {
                Some(e) => Some(s.call(|s| expr(s, e)).await),
                None => None,
            },
            body: s.call(|s| expr(s, &c.body)).await,
            span: c.span,
        });
    }
    out
}
async fn function<'a>(s: &Scheduler<'a>, f: &'a Function) -> Function {
    let mut params = Vec::with_capacity(f.params.len());
    for p in &f.params {
        params.push(match p {
            Param::Pattern { pattern: p, label } => Param::Pattern {
                pattern: s.call(|s| pattern(s, p)).await,
                label: label.clone(),
            },
            Param::Env { var, name, ty: t } => Param::Env {
                var: *var,
                name: name.clone(),
                ty: s.call(|s| ty(s, t)).await,
            },
        });
    }
    let ret = match &f.ret {
        Some(r) => Some(RetType {
            value: s.call(|s| ty(s, &r.value)).await,
            err: match &r.err {
                Some(x) => Some(s.call(|s| ty(s, x)).await),
                None => None,
            },
            env: match &r.env {
                Some(x) => Some(s.call(|s| ty(s, x)).await),
                None => None,
            },
        }),
        None => None,
    };
    Function {
        params,
        ret,
        body: block(s, &f.body).await,
        span: f.span,
    }
}
async fn expr<'a>(s: Scheduler<'a>, e: &'a Expr) -> Expr {
    let kind = match &e.kind {
        ExprKind::Int(x) => ExprKind::Int(x.clone()),
        ExprKind::Float(x) => ExprKind::Float(*x),
        ExprKind::String(x) => ExprKind::String(x.clone()),
        ExprKind::Name(x) => ExprKind::Name(*x),
        ExprKind::Tuple(xs) => ExprKind::Tuple(exprs(&s, xs).await),
        ExprKind::List { items, rest } => ExprKind::List {
            items: exprs(&s, items).await,
            rest: match rest {
                Some(x) => Some(Box::new(s.call(|s| expr(s, x)).await)),
                None => None,
            },
        },
        ExprKind::BitArray(xs) => {
            let mut out = Vec::with_capacity(xs.len());
            for x in xs {
                out.push(Segment {
                    value: s.call(|s| expr(s, &x.value)).await,
                    format: fmt(&s, &x.format).await,
                    span: x.span,
                });
            }
            ExprKind::BitArray(out)
        }
        ExprKind::Block(b) => ExprKind::Block(block(&s, b).await),
        ExprKind::Fn(f) => ExprKind::Fn(Box::new(function(&s, f).await)),
        ExprKind::Case {
            subject,
            clauses: cs,
        } => ExprKind::Case {
            subject: Box::new(s.call(|s| expr(s, subject)).await),
            clauses: clauses(&s, cs).await,
        },
        ExprKind::Call { callee, args } => {
            let callee = Box::new(s.call(|s| expr(s, callee)).await);
            let mut out = Vec::with_capacity(args.len());
            for a in args {
                out.push(Arg {
                    label: a.label.clone(),
                    value: s.call(|s| expr(s, &a.value)).await,
                });
            }
            ExprKind::Call { callee, args: out }
        }
        ExprKind::Field { target, name } => ExprKind::Field {
            target: Box::new(s.call(|s| expr(s, target)).await),
            name: name.clone(),
        },
        ExprKind::TupleIndex {
            target,
            index,
            index_span,
        } => ExprKind::TupleIndex {
            target: Box::new(s.call(|s| expr(s, target)).await),
            index: index.clone(),
            index_span: *index_span,
        },
        ExprKind::RecordUpdate {
            ctor,
            ctor_span,
            base,
            fields,
        } => {
            let base = Box::new(s.call(|s| expr(s, base)).await);
            let mut out = Vec::with_capacity(fields.len());
            for (n, e) in fields {
                out.push((n.clone(), s.call(|s| expr(s, e)).await));
            }
            ExprKind::RecordUpdate {
                ctor: *ctor,
                ctor_span: *ctor_span,
                base,
                fields: out,
            }
        }
        ExprKind::Unary { op, expr: e } => ExprKind::Unary {
            op: *op,
            expr: Box::new(s.call(|s| expr(s, e)).await),
        },
        ExprKind::Binary { op, lhs, rhs } => ExprKind::Binary {
            op: *op,
            lhs: Box::new(s.call(|s| expr(s, lhs)).await),
            rhs: Box::new(s.call(|s| expr(s, rhs)).await),
        },
        ExprKind::Error => ExprKind::Error,
    };
    Expr { kind, span: e.span }
}
impl Clone for Type {
    fn clone(&self) -> Self {
        trampoline::run(|s| ty(s, self))
    }
}
impl Clone for Pattern {
    fn clone(&self) -> Self {
        trampoline::run(|s| pattern(s, self))
    }
}
impl Clone for Expr {
    fn clone(&self) -> Self {
        trampoline::run(|s| expr(s, self))
    }
}

// Each owning node replaces its children by terminal sentinels before the
// children are dropped. This makes cleanup stack-safe even on failed checks.
enum Part {
    Expr(ExprKind),
    Pattern(PatternKind),
    Type(TypeKind),
}
fn take_expr(e: &mut Expr, w: &mut Vec<Part>) {
    w.push(Part::Expr(std::mem::replace(&mut e.kind, ExprKind::Error)));
}
fn take_pat(p: &mut Pattern, w: &mut Vec<Part>) {
    w.push(Part::Pattern(std::mem::replace(
        &mut p.kind,
        PatternKind::Discard,
    )));
}
fn take_ty(t: &mut Type, w: &mut Vec<Part>) {
    w.push(Part::Type(std::mem::replace(&mut t.kind, TypeKind::Error)));
}
fn take_format(f: &mut Option<Format>, w: &mut Vec<Part>) {
    if let Some(e) = f.as_mut().and_then(|f| f.arg.as_mut()) {
        take_expr(e, w);
    }
}
fn take_block(b: &mut Block, w: &mut Vec<Part>) {
    for st in &mut b.stmts {
        match st {
            Stmt::Bind(b) => {
                take_pat(&mut b.pattern, w);
                take_expr(&mut b.value, w)
            }
            Stmt::Env { ty, value, .. } => {
                take_ty(ty, w);
                take_expr(value, w)
            }
            Stmt::Expr(e) => take_expr(e, w),
        }
    }
}
fn drain(mut work: Vec<Part>) {
    while let Some(part) = work.pop() {
        match part {
            Part::Expr(mut k) => match &mut k {
                ExprKind::Tuple(xs) => {
                    for e in xs {
                        take_expr(e, &mut work)
                    }
                }
                ExprKind::List { items, rest } => {
                    for e in items {
                        take_expr(e, &mut work)
                    }
                    if let Some(e) = rest {
                        take_expr(e, &mut work)
                    }
                }
                ExprKind::BitArray(xs) => {
                    for x in xs {
                        take_expr(&mut x.value, &mut work);
                        take_format(&mut x.format, &mut work)
                    }
                }
                ExprKind::Block(b) => take_block(b, &mut work),
                ExprKind::Fn(f) => {
                    for p in &mut f.params {
                        match p {
                            Param::Pattern { pattern, .. } => take_pat(pattern, &mut work),
                            Param::Env { ty, .. } => take_ty(ty, &mut work),
                        }
                    }
                    if let Some(r) = &mut f.ret {
                        take_ty(&mut r.value, &mut work);
                        if let Some(t) = &mut r.err {
                            take_ty(t, &mut work)
                        }
                        if let Some(t) = &mut r.env {
                            take_ty(t, &mut work)
                        }
                    }
                    take_block(&mut f.body, &mut work)
                }
                ExprKind::Case { subject, clauses } => {
                    take_expr(subject, &mut work);
                    for c in clauses {
                        for p in &mut c.patterns {
                            take_pat(p, &mut work)
                        }
                        if let Some(e) = &mut c.guard {
                            take_expr(e, &mut work)
                        }
                        take_expr(&mut c.body, &mut work)
                    }
                }
                ExprKind::Call { callee, args } => {
                    take_expr(callee, &mut work);
                    for a in args {
                        take_expr(&mut a.value, &mut work)
                    }
                }
                ExprKind::Field { target, .. }
                | ExprKind::TupleIndex { target, .. }
                | ExprKind::Unary { expr: target, .. } => take_expr(target, &mut work),
                ExprKind::RecordUpdate { base, fields, .. } => {
                    take_expr(base, &mut work);
                    for (_, e) in fields {
                        take_expr(e, &mut work)
                    }
                }
                ExprKind::Binary { lhs, rhs, .. } => {
                    take_expr(lhs, &mut work);
                    take_expr(rhs, &mut work)
                }
                ExprKind::Int(_)
                | ExprKind::Float(_)
                | ExprKind::String(_)
                | ExprKind::Name(_)
                | ExprKind::Error => {}
            },
            Part::Pattern(mut k) => match &mut k {
                PatternKind::Typed { ty, .. } => take_ty(ty, &mut work),
                PatternKind::Named { pattern, .. } => take_pat(pattern, &mut work),
                PatternKind::Constructor { args, .. }
                | PatternKind::Tuple(args)
                | PatternKind::Error(args) => {
                    for p in args {
                        take_pat(p, &mut work)
                    }
                }
                PatternKind::List { items, .. } => {
                    for p in items {
                        take_pat(p, &mut work)
                    }
                }
                PatternKind::BitArray(xs) => {
                    for x in xs {
                        take_pat(&mut x.value, &mut work);
                        take_format(&mut x.format, &mut work)
                    }
                }
                PatternKind::Var(_)
                | PatternKind::Discard
                | PatternKind::Int { .. }
                | PatternKind::Float { .. }
                | PatternKind::String(_)
                | PatternKind::Prefix { .. } => {}
            },
            Part::Type(mut k) => match &mut k {
                TypeKind::Named { args, .. }
                | TypeKind::Tuple(args)
                | TypeKind::Union(args)
                | TypeKind::Inter(args)
                | TypeKind::Diff(args) => {
                    for t in args {
                        take_ty(t, &mut work)
                    }
                }
                TypeKind::Fn {
                    params,
                    ret,
                    err,
                    env,
                } => {
                    for p in params {
                        take_ty(&mut p.ty, &mut work)
                    }
                    take_ty(ret, &mut work);
                    if let Some(t) = err {
                        take_ty(t, &mut work)
                    }
                    if let Some(t) = env {
                        take_ty(t, &mut work)
                    }
                }
                TypeKind::Lit(_)
                | TypeKind::Of { .. }
                | TypeKind::Var(_)
                | TypeKind::Hole
                | TypeKind::Error => {}
            },
        }
    }
}
impl Drop for Expr {
    fn drop(&mut self) {
        if !matches!(
            self.kind,
            ExprKind::Error
                | ExprKind::Int(_)
                | ExprKind::Float(_)
                | ExprKind::String(_)
                | ExprKind::Name(_)
        ) {
            let mut w = Vec::new();
            take_expr(self, &mut w);
            drain(w);
        }
    }
}
impl Drop for Pattern {
    fn drop(&mut self) {
        if !matches!(
            self.kind,
            PatternKind::Discard
                | PatternKind::Var(_)
                | PatternKind::Int { .. }
                | PatternKind::Float { .. }
                | PatternKind::String(_)
                | PatternKind::Prefix { .. }
        ) {
            let mut w = Vec::new();
            take_pat(self, &mut w);
            drain(w);
        }
    }
}
impl Drop for Type {
    fn drop(&mut self) {
        if !matches!(
            self.kind,
            TypeKind::Error
                | TypeKind::Hole
                | TypeKind::Var(_)
                | TypeKind::Lit(_)
                | TypeKind::Of { .. }
        ) {
            let mut w = Vec::new();
            take_ty(self, &mut w);
            drain(w);
        }
    }
}

async fn type_pattern<'a>(s: Scheduler<'a>, p: &'a TypePattern) -> TypePattern {
    match p {
        TypePattern::Bind(id) => TypePattern::Bind(*id),
        TypePattern::Skip => TypePattern::Skip,
        TypePattern::Named(ty, args) => {
            let mut out = Vec::with_capacity(args.len());
            for p in args {
                out.push(s.call(|s| type_pattern(s, p)).await);
            }
            TypePattern::Named(*ty, out)
        }
        TypePattern::Tuple(args) => {
            let mut out = Vec::with_capacity(args.len());
            for p in args {
                out.push(s.call(|s| type_pattern(s, p)).await);
            }
            TypePattern::Tuple(out)
        }
        TypePattern::Exact(t) => TypePattern::Exact(s.call(|s| ty(s, t)).await),
        TypePattern::Error => TypePattern::Error,
    }
}
impl Clone for TypePattern {
    fn clone(&self) -> Self {
        trampoline::run(|s| type_pattern(s, self))
    }
}
impl Drop for TypePattern {
    fn drop(&mut self) {
        let children = match self {
            TypePattern::Named(_, children) | TypePattern::Tuple(children) => {
                std::mem::take(children)
            }
            _ => return,
        };
        let mut work = children;
        while let Some(mut p) = work.pop() {
            match &mut p {
                TypePattern::Named(_, children) | TypePattern::Tuple(children) => {
                    work.append(children)
                }
                _ => {}
            }
        }
    }
}

impl Type {
    /// Move out the resolved kind while leaving a terminal node for iterative cleanup.
    pub fn into_kind(mut self) -> TypeKind {
        std::mem::replace(&mut self.kind, TypeKind::Error)
    }
}
impl Pattern {
    /// Move out the resolved kind while leaving a terminal node for iterative cleanup.
    pub fn into_kind(mut self) -> PatternKind {
        std::mem::replace(&mut self.kind, PatternKind::Discard)
    }
}
impl Expr {
    /// Move out the resolved kind while leaving a terminal node for iterative cleanup.
    pub fn into_kind(mut self) -> ExprKind {
        std::mem::replace(&mut self.kind, ExprKind::Error)
    }
}
