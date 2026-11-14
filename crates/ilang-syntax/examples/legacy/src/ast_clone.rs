//! Stack-safe cloning of syntax nodes. Each recursive child is a separate
//! work-stack task; ordinary wrappers have fixed nesting independent of input.
use crate::ast::*;
use crate::lisp::trampoline::{self, Scheduler};

async fn ce<'a>(s: Scheduler<'a>, e: &'a Expr) -> Expr {
    use ExprKind as K;
    let kind = match &e.kind {
        K::Quote(d) => K::Quote(d.clone()),
        K::Int(x) => K::Int(x.clone()),
        K::Float(x) => K::Float(*x),
        K::String(x) => K::String(x.clone()),
        K::Name { module, name } => K::Name {
            module: module.clone(),
            name: name.clone(),
        },
        K::Tuple(xs) => K::Tuple(exprs(&s, xs).await),
        K::Record { base, fields } => {
            let base = match base {
                Some(x) => Some(Box::new(s.call(|s| ce(s, x)).await)),
                None => None,
            };
            let mut out = Vec::with_capacity(fields.len());
            for (label, value) in fields {
                out.push((label.clone(), s.call(|s| ce(s, value)).await));
            }
            K::Record { base, fields: out }
        }
        K::List { items, rest } => K::List {
            items: exprs(&s, items).await,
            rest: match rest {
                Some(x) => Some(Box::new(s.call(|s| ce(s, x)).await)),
                None => None,
            },
        },
        K::BitArray(xs) => {
            let mut out = Vec::with_capacity(xs.len());
            for x in xs {
                out.push(Segment {
                    value: s.call(|s| ce(s, &x.value)).await,
                    format: format(&s, &x.format).await,
                    span: x.span,
                });
            }
            K::BitArray(out)
        }
        K::Block(b) => K::Block(block(&s, b).await),
        K::Fn(f) => K::Fn(Box::new(function(&s, f).await)),
        K::Case {
            subject,
            clauses: cs,
        } => K::Case {
            subject: Box::new(s.call(|s| ce(s, subject)).await),
            clauses: clauses(&s, cs).await,
        },
        K::Clauses(cs) => K::Clauses(clauses(&s, cs).await),
        K::Call { callee, args } => {
            let callee = Box::new(s.call(|s| ce(s, callee)).await);
            let mut out = Vec::with_capacity(args.len());
            for a in args {
                out.push(Arg {
                    label: a.label.clone(),
                    value: s.call(|s| ce(s, &a.value)).await,
                });
            }
            K::Call { callee, args: out }
        }
        K::Field { target, name } => K::Field {
            target: Box::new(s.call(|s| ce(s, target)).await),
            name: name.clone(),
        },
        K::TupleIndex {
            target,
            index,
            index_span,
        } => K::TupleIndex {
            target: Box::new(s.call(|s| ce(s, target)).await),
            index: index.clone(),
            index_span: *index_span,
        },
        K::RecordUpdate {
            constructor,
            base,
            fields,
        } => {
            let constructor = Box::new(s.call(|s| ce(s, constructor)).await);
            let base = Box::new(s.call(|s| ce(s, base)).await);
            let mut out = Vec::with_capacity(fields.len());
            for (n, v) in fields {
                out.push((n.clone(), s.call(|s| ce(s, v)).await));
            }
            K::RecordUpdate {
                constructor,
                base,
                fields: out,
            }
        }
        K::Unary { op, expr } => K::Unary {
            op: *op,
            expr: Box::new(s.call(|s| ce(s, expr)).await),
        },
        K::Binary { op, lhs, rhs } => K::Binary {
            op: *op,
            lhs: Box::new(s.call(|s| ce(s, lhs)).await),
            rhs: Box::new(s.call(|s| ce(s, rhs)).await),
        },
    };
    Expr { kind, span: e.span }
}
async fn exprs<'a>(s: &Scheduler<'a>, xs: &'a [Expr]) -> Vec<Expr> {
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        out.push(s.call(|s| ce(s, x)).await);
    }
    out
}
async fn pats<'a>(s: &Scheduler<'a>, xs: &'a [Pattern]) -> Vec<Pattern> {
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        out.push(s.call(|s| cp(s, x)).await);
    }
    out
}
async fn types<'a>(s: &Scheduler<'a>, xs: &'a [TypeExpr]) -> Vec<TypeExpr> {
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        out.push(s.call(|s| ct(s, x)).await);
    }
    out
}
async fn format<'a>(s: &Scheduler<'a>, f: &'a Option<Format>) -> Option<Format> {
    match f {
        None => None,
        Some(f) => Some(Format {
            name: f.name.clone(),
            arg: match &f.arg {
                None => None,
                Some(x) => Some(Box::new(s.call(|s| ce(s, x)).await)),
            },
        }),
    }
}
async fn block<'a>(s: &Scheduler<'a>, b: &'a Block) -> Block {
    let mut out = Vec::with_capacity(b.stmts.len());
    for x in &b.stmts {
        out.push(match x {
            Stmt::Expr(e) => Stmt::Expr(s.call(|s| ce(s, e)).await),
            Stmt::Env { ty, value, span } => Stmt::Env {
                ty: s.call(|s| ct(s, ty)).await,
                value: s.call(|s| ce(s, value)).await,
                span: *span,
            },
            Stmt::Bind(b) => Stmt::Bind(Binding {
                pattern: s.call(|s| cp(s, &b.pattern)).await,
                value: s.call(|s| ce(s, &b.value)).await,
                span: b.span,
                type_params: b.type_params.clone(),
            }),
        });
    }
    Block {
        stmts: out,
        span: b.span,
    }
}
async fn clauses<'a>(s: &Scheduler<'a>, xs: &'a [Clause]) -> Vec<Clause> {
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        out.push(Clause {
            patterns: pats(s, &x.patterns).await,
            guard: match &x.guard {
                None => None,
                Some(g) => Some(s.call(|s| ce(s, g)).await),
            },
            body: s.call(|s| ce(s, &x.body)).await,
            span: x.span,
        });
    }
    out
}
async fn function<'a>(s: &Scheduler<'a>, f: &'a Function) -> Function {
    let mut params = Vec::with_capacity(f.params.len());
    for p in &f.params {
        params.push(match p {
            Param::Pattern(p) => Param::Pattern(s.call(|s| cp(s, p)).await),
            Param::Env { name, ty } => Param::Env {
                name: name.clone(),
                ty: s.call(|s| ct(s, ty)).await,
            },
        });
    }
    let ret = match &f.ret {
        None => None,
        Some(r) => Some(RetType {
            value: s.call(|s| ct(s, &r.value)).await,
            err: match &r.err {
                None => None,
                Some(t) => Some(s.call(|s| ct(s, t)).await),
            },
            env: match &r.env {
                None => None,
                Some(t) => Some(s.call(|s| ct(s, t)).await),
            },
        }),
    };
    Function {
        params,
        ret,
        body: block(s, &f.body).await,
    }
}
async fn cp<'a>(s: Scheduler<'a>, p: &'a Pattern) -> Pattern {
    use PatternKind as K;
    let kind = match &p.kind {
        K::Quote {
            datum,
            template,
            captures,
        } => {
            let mut out = Vec::with_capacity(captures.len());
            for (splice, p) in captures {
                out.push((*splice, s.call(|s| cp(s, p)).await));
            }
            K::Quote {
                datum: datum.clone(),
                template: *template,
                captures: out,
            }
        }
        K::Var(x) => K::Var(x.clone()),
        K::Discard(x) => K::Discard(x.clone()),
        K::Typed { binder, ty } => K::Typed {
            binder: binder.clone(),
            ty: s.call(|s| ct(s, ty)).await,
        },
        K::Named { name, pattern } => K::Named {
            name: name.clone(),
            pattern: Box::new(s.call(|s| cp(s, pattern)).await),
        },
        K::Int { negative, value } => K::Int {
            negative: *negative,
            value: value.clone(),
        },
        K::Float { negative, value } => K::Float {
            negative: *negative,
            value: *value,
        },
        K::String(x) => K::String(x.clone()),
        K::Prefix {
            prefix,
            suffix,
            rest,
        } => K::Prefix {
            prefix: prefix.clone(),
            suffix: suffix.clone(),
            rest: rest.clone(),
        },
        K::Constructor {
            module,
            name,
            args,
            spread,
        } => K::Constructor {
            module: module.clone(),
            name: name.clone(),
            args: pats(&s, args).await,
            spread: *spread,
        },
        K::Tuple(xs) => K::Tuple(pats(&s, xs).await),
        K::List { items, rest } => K::List {
            items: pats(&s, items).await,
            rest: rest.clone(),
        },
        K::BitArray(xs) => {
            let mut out = Vec::with_capacity(xs.len());
            for x in xs {
                out.push(Segment {
                    value: s.call(|s| cp(s, &x.value)).await,
                    format: format(&s, &x.format).await,
                    span: x.span,
                });
            }
            K::BitArray(out)
        }
    };
    Pattern { kind, span: p.span }
}
async fn ct<'a>(s: Scheduler<'a>, t: &'a TypeExpr) -> TypeExpr {
    use TypeKind as K;
    let kind = match &t.kind {
        K::Lit(x) => K::Lit(x.clone()),
        K::Named { module, name, args } => K::Named {
            module: module.clone(),
            name: name.clone(),
            args: types(&s, args).await,
        },
        K::ValueOf { module, name } => K::ValueOf {
            module: module.clone(),
            name: name.clone(),
        },
        K::Call { module, name, args } => K::Call {
            module: module.clone(),
            name: name.clone(),
            args: types(&s, args).await,
        },
        K::Hole => K::Hole,
        K::Tuple(xs) => K::Tuple(types(&s, xs).await),
        K::Record {
            base,
            fields,
            removed,
        } => {
            let base = match base {
                Some(x) => Some(Box::new(s.call(|s| ct(s, x)).await)),
                None => None,
            };
            let mut out = Vec::with_capacity(fields.len());
            for (label, value) in fields {
                out.push((label.clone(), s.call(|s| ct(s, value)).await));
            }
            K::Record {
                base,
                fields: out,
                removed: removed.clone(),
            }
        }
        K::Union(xs) => K::Union(types(&s, xs).await),
        K::Inter(xs) => K::Inter(types(&s, xs).await),
        K::Diff(xs) => K::Diff(types(&s, xs).await),
        K::Fn {
            params,
            ret,
            err,
            env,
        } => {
            let mut ps = Vec::with_capacity(params.len());
            for p in params {
                ps.push(FnParamType {
                    label: p.label.clone(),
                    ty: s.call(|s| ct(s, &p.ty)).await,
                });
            }
            K::Fn {
                params: ps,
                ret: Box::new(s.call(|s| ct(s, ret)).await),
                err: match err {
                    None => None,
                    Some(t) => Some(Box::new(s.call(|s| ct(s, t)).await)),
                },
                env: match env {
                    None => None,
                    Some(t) => Some(Box::new(s.call(|s| ct(s, t)).await)),
                },
            }
        }
    };
    TypeExpr { kind, span: t.span }
}
impl Clone for Expr {
    fn clone(&self) -> Self {
        trampoline::run(|s| ce(s, self))
    }
}
impl Clone for Pattern {
    fn clone(&self) -> Self {
        trampoline::run(|s| cp(s, self))
    }
}
impl Clone for TypeExpr {
    fn clone(&self) -> Self {
        trampoline::run(|s| ct(s, self))
    }
}
