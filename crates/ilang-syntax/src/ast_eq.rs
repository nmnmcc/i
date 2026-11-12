//! Stack-safe structural equality for syntax nodes produced by macros.
use crate::ast::*;

enum Pair<'a> {
    Expr(&'a Expr, &'a Expr),
    Pattern(&'a Pattern, &'a Pattern),
    Type(&'a TypeExpr, &'a TypeExpr),
}
fn exprs<'a>(a: &'a [Expr], b: &'a [Expr], q: &mut Vec<Pair<'a>>) -> bool {
    if a.len() != b.len() {
        return false;
    }
    q.extend(a.iter().zip(b).map(|(a, b)| Pair::Expr(a, b)));
    true
}
fn pats<'a>(a: &'a [Pattern], b: &'a [Pattern], q: &mut Vec<Pair<'a>>) -> bool {
    if a.len() != b.len() {
        return false;
    }
    q.extend(a.iter().zip(b).map(|(a, b)| Pair::Pattern(a, b)));
    true
}
fn types<'a>(a: &'a [TypeExpr], b: &'a [TypeExpr], q: &mut Vec<Pair<'a>>) -> bool {
    if a.len() != b.len() {
        return false;
    }
    q.extend(a.iter().zip(b).map(|(a, b)| Pair::Type(a, b)));
    true
}
fn optional<'a, T>(
    a: &'a Option<T>,
    b: &'a Option<T>,
    mut f: impl FnMut(&'a T, &'a T) -> bool,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => f(a, b),
        _ => false,
    }
}
fn fmt<'a>(a: &'a Option<Format>, b: &'a Option<Format>, q: &mut Vec<Pair<'a>>) -> bool {
    optional(a, b, |a, b| {
        a.name == b.name
            && optional(&a.arg, &b.arg, |x, y| {
                q.push(Pair::Expr(x, y));
                true
            })
    })
}
fn block<'a>(a: &'a Block, b: &'a Block, q: &mut Vec<Pair<'a>>) -> bool {
    if a.span != b.span || a.stmts.len() != b.stmts.len() {
        return false;
    }
    a.stmts.iter().zip(&b.stmts).all(|(a, b)| match (a, b) {
        (Stmt::Expr(a), Stmt::Expr(b)) => {
            q.push(Pair::Expr(a, b));
            true
        }
        (Stmt::Bind(a), Stmt::Bind(b)) => {
            if a.span != b.span || a.type_params != b.type_params {
                return false;
            }
            q.push(Pair::Pattern(&a.pattern, &b.pattern));
            q.push(Pair::Expr(&a.value, &b.value));
            true
        }
        (
            Stmt::Env {
                ty: a,
                value: av,
                span: aspan,
            },
            Stmt::Env {
                ty: b,
                value: bv,
                span: bspan,
            },
        ) => {
            q.push(Pair::Type(a, b));
            q.push(Pair::Expr(av, bv));
            aspan == bspan
        }
        _ => false,
    })
}
fn clauses<'a>(a: &'a [Clause], b: &'a [Clause], q: &mut Vec<Pair<'a>>) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.span == b.span
                && pats(&a.patterns, &b.patterns, q)
                && optional(&a.guard, &b.guard, |a, b| {
                    q.push(Pair::Expr(a, b));
                    true
                })
                && {
                    q.push(Pair::Expr(&a.body, &b.body));
                    true
                }
        })
}
fn function<'a>(a: &'a Function, b: &'a Function, q: &mut Vec<Pair<'a>>) -> bool {
    if a.params.len() != b.params.len() {
        return false;
    }
    let ps = a.params.iter().zip(&b.params).all(|(a, b)| match (a, b) {
        (Param::Pattern(a), Param::Pattern(b)) => {
            q.push(Pair::Pattern(a, b));
            true
        }
        (Param::Env { name: a, ty: at }, Param::Env { name: b, ty: bt }) => {
            q.push(Pair::Type(at, bt));
            a == b
        }
        _ => false,
    });
    ps && optional(&a.ret, &b.ret, |a, b| {
        q.push(Pair::Type(&a.value, &b.value));
        optional(&a.err, &b.err, |a, b| {
            q.push(Pair::Type(a, b));
            true
        }) && optional(&a.env, &b.env, |a, b| {
            q.push(Pair::Type(a, b));
            true
        })
    }) && block(&a.body, &b.body, q)
}
fn eq(mut q: Vec<Pair<'_>>) -> bool {
    while let Some(p) = q.pop() {
        match p {
            Pair::Expr(a, b) => {
                if a.span != b.span {
                    return false;
                }
                use ExprKind as K;
                let same = match (&a.kind, &b.kind) {
                    (K::Quote(a), K::Quote(b)) => a == b,
                    (K::Int(a), K::Int(b)) => a == b,
                    (K::Float(a), K::Float(b)) => a == b,
                    (K::String(a), K::String(b)) => a == b,
                    (
                        K::Name {
                            module: a,
                            name: an,
                        },
                        K::Name {
                            module: b,
                            name: bn,
                        },
                    ) => a == b && an == bn,
                    (K::Tuple(a), K::Tuple(b)) => exprs(a, b, &mut q),
                    (
                        K::Record {
                            base: a,
                            fields: af,
                        },
                        K::Record {
                            base: b,
                            fields: bf,
                        },
                    ) => {
                        optional(a, b, |a, b| {
                            q.push(Pair::Expr(a, b));
                            true
                        }) && af.len() == bf.len()
                            && af.iter().zip(bf).all(|((al, av), (bl, bv))| {
                                q.push(Pair::Expr(av, bv));
                                al == bl
                            })
                    }
                    (K::List { items: a, rest: ar }, K::List { items: b, rest: br }) => {
                        exprs(a, b, &mut q)
                            && optional(ar, br, |a, b| {
                                q.push(Pair::Expr(a, b));
                                true
                            })
                    }
                    (K::BitArray(a), K::BitArray(b)) => {
                        a.len() == b.len()
                            && a.iter().zip(b).all(|(a, b)| {
                                q.push(Pair::Expr(&a.value, &b.value));
                                a.span == b.span && fmt(&a.format, &b.format, &mut q)
                            })
                    }
                    (K::Block(a), K::Block(b)) => block(a, b, &mut q),
                    (K::Fn(a), K::Fn(b)) => function(a, b, &mut q),
                    (
                        K::Case {
                            subject: a,
                            clauses: ac,
                        },
                        K::Case {
                            subject: b,
                            clauses: bc,
                        },
                    ) => {
                        q.push(Pair::Expr(a, b));
                        clauses(ac, bc, &mut q)
                    }
                    (K::Clauses(a), K::Clauses(b)) => clauses(a, b, &mut q),
                    (
                        K::Call {
                            callee: a,
                            args: aa,
                        },
                        K::Call {
                            callee: b,
                            args: ba,
                        },
                    ) => {
                        q.push(Pair::Expr(a, b));
                        aa.len() == ba.len()
                            && aa.iter().zip(ba).all(|(a, b)| {
                                q.push(Pair::Expr(&a.value, &b.value));
                                a.label == b.label
                            })
                    }
                    (
                        K::Field {
                            target: a,
                            name: an,
                        },
                        K::Field {
                            target: b,
                            name: bn,
                        },
                    ) => {
                        q.push(Pair::Expr(a, b));
                        an == bn
                    }
                    (
                        K::TupleIndex {
                            target: a,
                            index: ai,
                            index_span: aspan,
                        },
                        K::TupleIndex {
                            target: b,
                            index: bi,
                            index_span: bspan,
                        },
                    ) => {
                        q.push(Pair::Expr(a, b));
                        ai == bi && aspan == bspan
                    }
                    (
                        K::RecordUpdate {
                            constructor: a,
                            base: ab,
                            fields: af,
                        },
                        K::RecordUpdate {
                            constructor: b,
                            base: bb,
                            fields: bf,
                        },
                    ) => {
                        q.push(Pair::Expr(a, b));
                        q.push(Pair::Expr(ab, bb));
                        af.len() == bf.len()
                            && af.iter().zip(bf).all(|((an, a), (bn, b))| {
                                q.push(Pair::Expr(a, b));
                                an == bn
                            })
                    }
                    (K::Unary { op: a, expr: ae }, K::Unary { op: b, expr: be }) => {
                        q.push(Pair::Expr(ae, be));
                        a == b
                    }
                    (
                        K::Binary {
                            op: a,
                            lhs: al,
                            rhs: ar,
                        },
                        K::Binary {
                            op: b,
                            lhs: bl,
                            rhs: br,
                        },
                    ) => {
                        q.push(Pair::Expr(al, bl));
                        q.push(Pair::Expr(ar, br));
                        a == b
                    }
                    _ => false,
                };
                if !same {
                    return false;
                }
            }
            Pair::Pattern(a, b) => {
                if a.span != b.span {
                    return false;
                }
                use PatternKind as K;
                let same = match (&a.kind, &b.kind) {
                    (
                        K::Quote {
                            datum: a,
                            template: at,
                            captures: ac,
                        },
                        K::Quote {
                            datum: b,
                            template: bt,
                            captures: bc,
                        },
                    ) => {
                        if ac.len() != bc.len()
                            || ac.iter().zip(bc).any(|((as_, _), (bs, _))| as_ != bs)
                        {
                            return false;
                        }
                        q.extend(
                            ac.iter()
                                .zip(bc)
                                .map(|((_, a), (_, b))| Pair::Pattern(a, b)),
                        );
                        a == b && at == bt
                    }
                    (K::Var(a), K::Var(b))
                    | (K::Discard(a), K::Discard(b))
                    | (K::String(a), K::String(b)) => a == b,
                    (K::Typed { binder: a, ty: at }, K::Typed { binder: b, ty: bt }) => {
                        q.push(Pair::Type(at, bt));
                        a == b
                    }
                    (
                        K::Named {
                            name: a,
                            pattern: ap,
                        },
                        K::Named {
                            name: b,
                            pattern: bp,
                        },
                    ) => {
                        q.push(Pair::Pattern(ap, bp));
                        a == b
                    }
                    (
                        K::Int {
                            negative: a,
                            value: av,
                        },
                        K::Int {
                            negative: b,
                            value: bv,
                        },
                    ) => a == b && av == bv,
                    (
                        K::Float {
                            negative: a,
                            value: av,
                        },
                        K::Float {
                            negative: b,
                            value: bv,
                        },
                    ) => a == b && av == bv,
                    (
                        K::Prefix {
                            prefix: a,
                            suffix: az,
                            rest: ar,
                        },
                        K::Prefix {
                            prefix: b,
                            suffix: bz,
                            rest: br,
                        },
                    ) => a == b && az == bz && ar == br,
                    (
                        K::Constructor {
                            module: a,
                            name: an,
                            args: aa,
                            spread: as_,
                        },
                        K::Constructor {
                            module: b,
                            name: bn,
                            args: ba,
                            spread: bs,
                        },
                    ) => a == b && an == bn && as_ == bs && pats(aa, ba, &mut q),
                    (K::Tuple(a), K::Tuple(b)) => pats(a, b, &mut q),
                    (K::List { items: a, rest: ar }, K::List { items: b, rest: br }) => {
                        ar == br && pats(a, b, &mut q)
                    }
                    (K::BitArray(a), K::BitArray(b)) => {
                        a.len() == b.len()
                            && a.iter().zip(b).all(|(a, b)| {
                                q.push(Pair::Pattern(&a.value, &b.value));
                                a.span == b.span && fmt(&a.format, &b.format, &mut q)
                            })
                    }
                    _ => false,
                };
                if !same {
                    return false;
                }
            }
            Pair::Type(a, b) => {
                if a.span != b.span {
                    return false;
                }
                use TypeKind as K;
                let same = match (&a.kind, &b.kind) {
                    (K::Lit(a), K::Lit(b)) => a == b,
                    (
                        K::Named {
                            module: a,
                            name: an,
                            args: aa,
                        },
                        K::Named {
                            module: b,
                            name: bn,
                            args: ba,
                        },
                    )
                    | (
                        K::Call {
                            module: a,
                            name: an,
                            args: aa,
                        },
                        K::Call {
                            module: b,
                            name: bn,
                            args: ba,
                        },
                    ) => a == b && an == bn && types(aa, ba, &mut q),
                    (
                        K::ValueOf {
                            module: a,
                            name: an,
                        },
                        K::ValueOf {
                            module: b,
                            name: bn,
                        },
                    ) => a == b && an == bn,
                    (K::Hole, K::Hole) => true,
                    (
                        K::Record {
                            base: a,
                            fields: af,
                            removed: ar,
                        },
                        K::Record {
                            base: b,
                            fields: bf,
                            removed: br,
                        },
                    ) => {
                        ar == br
                            && optional(a, b, |a, b| {
                                q.push(Pair::Type(a, b));
                                true
                            })
                            && af.len() == bf.len()
                            && af.iter().zip(bf).all(|((al, av), (bl, bv))| {
                                q.push(Pair::Type(av, bv));
                                al == bl
                            })
                    }
                    (K::Tuple(a), K::Tuple(b))
                    | (K::Union(a), K::Union(b))
                    | (K::Inter(a), K::Inter(b))
                    | (K::Diff(a), K::Diff(b)) => types(a, b, &mut q),
                    (
                        K::Fn {
                            params: a,
                            ret: ar,
                            err: ae,
                            env: an,
                        },
                        K::Fn {
                            params: b,
                            ret: br,
                            err: be,
                            env: bn,
                        },
                    ) => {
                        if a.len() != b.len() {
                            false
                        } else {
                            let params = a.iter().zip(b).all(|(a, b)| {
                                q.push(Pair::Type(&a.ty, &b.ty));
                                a.label == b.label
                            });
                            q.push(Pair::Type(ar, br));
                            params
                                && optional(ae, be, |a, b| {
                                    q.push(Pair::Type(a, b));
                                    true
                                })
                                && optional(an, bn, |a, b| {
                                    q.push(Pair::Type(a, b));
                                    true
                                })
                        }
                    }
                    _ => false,
                };
                if !same {
                    return false;
                }
            }
        }
    }
    true
}
impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        eq(vec![Pair::Expr(self, other)])
    }
}
impl PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        eq(vec![Pair::Pattern(self, other)])
    }
}
impl PartialEq for TypeExpr {
    fn eq(&self, other: &Self) -> bool {
        eq(vec![Pair::Type(self, other)])
    }
}
