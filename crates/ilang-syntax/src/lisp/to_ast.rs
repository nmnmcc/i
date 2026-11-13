//! Convert Lisp datums directly to source-located core nodes.
use super::{Datum, DatumKind as K, Document};
use crate::ast::*;
use crate::{Code, Diagnostic};
use num_bigint::{BigInt, Sign};

type Result<T> = std::result::Result<T, Diagnostic>;
fn bad(d: &Datum, p: &str) -> Diagnostic {
    Diagnostic::new(
        Code::Unexpected,
        d.span,
        format!("datum is not valid as {p}"),
    )
}
fn symbol(d: &Datum, s: &str) -> bool {
    matches!(&d.kind, K::Symbol(x) if x == s)
}
fn round(d: &Datum) -> Option<&[Datum]> {
    match &d.kind {
        K::Round(xs) => Some(xs),
        _ => None,
    }
}
fn ident(d: &Datum) -> Result<Ident> {
    match &d.kind {
        K::Name(s) | K::UpName(s) => Ok(Ident {
            name: s.clone(),
            span: d.span,
        }),
        _ => Err(bad(d, "a name")),
    }
}
fn qualified(d: &Datum) -> Result<(Option<Ident>, Ident)> {
    match &d.kind {
        K::Name(_) | K::UpName(_) => Ok((None, ident(d)?)),
        K::Round(xs) if matches!(xs.as_slice(), [h, _, _] if symbol(h, ".")) => {
            if !matches!(xs[1].kind, K::Name(_)) {
                return Err(bad(&xs[1], "a module name"));
            }
            Ok((Some(ident(&xs[1])?), ident(&xs[2])?))
        }
        _ => Err(bad(d, "a qualified name")),
    }
}
use super::trampoline::{Scheduler, run};

// Record labels are text, not ordinary value/type references. Only a type
// record may use an uppercase atom as a scoped label parameter.
fn record_label(d: &Datum, types: bool) -> Result<RecordLabel> {
    let (name, parameter) = match &d.kind {
        K::Name(name) | K::String(name) => (name.clone(), false),
        K::UpName(name) if types => (name.clone(), true),
        _ => return Err(bad(d, "a record label")),
    };
    Ok(RecordLabel {
        ident: Ident { name, span: d.span },
        parameter,
    })
}

fn record_start(xs: &[Datum]) -> bool {
    xs.first()
        .is_some_and(|d| symbol(d, "..") || matches!(round(d), Some([h, _, _]) if symbol(h, ":")))
}

async fn record_type<'a>(s: &Scheduler<'a>, d: &'a Datum, xs: &'a [Datum]) -> Result<TypeKind> {
    let (base, rest) = if xs.first().is_some_and(|d| symbol(d, "..")) {
        let Some(base) = xs.get(1) else {
            return Err(bad(d, "a record type spread"));
        };
        (Some(Box::new(type_child(s, base).await?)), &xs[2..])
    } else {
        (None, xs)
    };
    let mut fields = Vec::new();
    let mut removed = Vec::new();
    for field in rest {
        match round(field) {
            Some([h, label, value]) if symbol(h, ":") => {
                fields.push((record_label(label, true)?, type_child(s, value).await?))
            }
            Some([h, label]) if symbol(h, "-") && base.is_some() => {
                removed.push(record_label(label, true)?)
            }
            _ => return Err(bad(field, "a record type field")),
        }
    }
    Ok(TypeKind::Record {
        base,
        fields,
        removed,
    })
}

async fn record_expression<'a>(
    s: &Scheduler<'a>,
    d: &'a Datum,
    xs: &'a [Datum],
) -> Result<ExprKind> {
    let (base, rest) = if xs.first().is_some_and(|d| symbol(d, "..")) {
        let Some(base) = xs.get(1) else {
            return Err(bad(d, "a record spread"));
        };
        (Some(Box::new(expression_child(s, base).await?)), &xs[2..])
    } else {
        (None, xs)
    };
    let mut fields = Vec::new();
    for field in rest {
        match round(field) {
            Some([h, label, value]) if symbol(h, ":") => fields.push((
                record_label(label, false)?.ident,
                expression_child(s, value).await?,
            )),
            _ => return Err(bad(field, "a record field")),
        }
    }
    Ok(ExprKind::Record { base, fields })
}

async fn types<'a>(s: &Scheduler<'a>, ds: &'a [Datum]) -> Result<Vec<TypeExpr>> {
    let mut out = Vec::with_capacity(ds.len());
    for d in ds {
        out.push(type_child(s, d).await?);
    }
    Ok(out)
}
async fn type_child<'a>(s: &Scheduler<'a>, d: &'a Datum) -> Result<TypeExpr> {
    s.call(move |s| async move { type_inner(&s, d).await })
        .await
}
/// Read a datum in a type position, retaining the original span at each node.
pub fn parse_type(d: &Datum) -> Result<TypeExpr> {
    run(|s| async move { type_inner(&s, d).await })
}
async fn type_inner<'a>(s: &Scheduler<'a>, d: &'a Datum) -> Result<TypeExpr> {
    let kind = match &d.kind {
        K::Discard(x) if x == "_" => TypeKind::Hole,
        K::Int(n) => TypeKind::Lit(LitValue::Int(BigInt::from_biguint(Sign::Plus, n.clone()))),
        K::String(x) => TypeKind::Lit(LitValue::Str(x.clone())),
        K::Name(_) | K::UpName(_) => named_type(s, d, &[]).await?,
        K::Curly(xs) if xs.len() == 1 => return type_child(s, &xs[0]).await,
        K::Round(xs) if record_start(xs) => record_type(s, d, xs).await?,
        K::Round(xs) => match xs.as_slice() {
            [h, n] if symbol(h, "-") && matches!(n.kind, K::Int(_)) => {
                let K::Int(n) = &n.kind else { unreachable!() };
                TypeKind::Lit(LitValue::Int(-BigInt::from_biguint(Sign::Plus, n.clone())))
            }
            [h, ..] if symbol(h, ".") => named_type(s, d, &[]).await?,
            [h, rest @ ..] if symbol(h, ",") && !rest.is_empty() => {
                TypeKind::Tuple(types(s, rest).await?)
            }
            [h, rest @ ..] if symbol(h, "|") && rest.len() >= 2 => {
                TypeKind::Union(types(s, rest).await?)
            }
            [h, rest @ ..] if symbol(h, "&") && rest.len() >= 2 => {
                TypeKind::Inter(types(s, rest).await?)
            }
            [h, rest @ ..] if symbol(h, "-") && rest.len() >= 2 => {
                TypeKind::Diff(types(s, rest).await?)
            }
            [h, ps, ret] if symbol(h, "->") => {
                let ps = round(ps).ok_or_else(|| bad(ps, "function type parameters"))?;
                let mut params = Vec::with_capacity(ps.len());
                for p in ps {
                    let (label, ty) = if let Some([h, n, t]) = round(p)
                        && symbol(h, ":")
                    {
                        (Some(ident(n)?), type_child(s, t).await?)
                    } else {
                        (None, type_child(s, p).await?)
                    };
                    params.push(FnParamType { label, ty });
                }
                let result = result_type_inner(s, ret).await?;
                TypeKind::Fn {
                    params,
                    ret: Box::new(result.value),
                    err: result.err.map(Box::new),
                    env: result.env.map(Box::new),
                }
            }
            [head]
                if matches!(head.kind, K::Name(_))
                    || matches!(round(head), Some([h, _, n]) if symbol(h, ".") && matches!(n.kind, K::Name(_))) =>
            {
                let (module, name) = qualified(head)?;
                TypeKind::Call {
                    module,
                    name,
                    args: Vec::new(),
                }
            }
            [head, args @ ..] if !args.is_empty() => named_type(s, head, args).await?,
            _ => return Err(bad(d, "a type")),
        },
        _ => return Err(bad(d, "a type")),
    };
    Ok(TypeExpr { kind, span: d.span })
}
async fn named_type<'a>(s: &Scheduler<'a>, head: &'a Datum, args: &'a [Datum]) -> Result<TypeKind> {
    let (module, name) = qualified(head)?;
    if name.name.starts_with(|c: char| c.is_ascii_uppercase()) {
        Ok(TypeKind::Named {
            module,
            name,
            args: types(s, args).await?,
        })
    } else if args.is_empty() {
        Ok(TypeKind::ValueOf { module, name })
    } else {
        Ok(TypeKind::Call {
            module,
            name,
            args: types(s, args).await?,
        })
    }
}
async fn result_type_inner<'a>(s: &Scheduler<'a>, d: &'a Datum) -> Result<RetType> {
    // The outer environment wrapper is part of the result syntax. Walk it
    // explicitly so nested wrappers do not recurse on the host stack.
    let mut current = d;
    let mut envs = Vec::new();
    while let Some([h, r, e]) = round(current) {
        if !symbol(h, "!") {
            break;
        }
        envs.push(e);
        current = r;
    }
    let mut ret = if let Some([h, r, e]) = round(current)
        && symbol(h, "^")
    {
        RetType {
            value: type_child(s, r).await?,
            err: if matches!(&r.kind, K::Discard(x) if x == "_")
                && matches!(&e.kind, K::UpName(x) if x == "Never")
            {
                None
            } else {
                Some(type_child(s, e).await?)
            },
            env: None,
        }
    } else {
        RetType {
            value: type_child(s, current).await?,
            err: None,
            env: None,
        }
    };
    for env in envs.into_iter().rev() {
        ret.env = Some(type_child(s, env).await?);
    }
    Ok(ret)
}

fn binder(d: &Datum) -> Result<Binder> {
    match &d.kind {
        K::Name(name) | K::Discard(name) => Ok(Binder {
            name: name.clone(),
            discard: matches!(d.kind, K::Discard(_)),
            span: d.span,
        }),
        _ => Err(bad(d, "a binder")),
    }
}

/// Read a datum in pattern position.
pub fn parse_pattern(d: &Datum) -> Result<Pattern> {
    run(|s| async move { pattern_inner(&s, d).await })
}
async fn pattern_child<'a>(s: &Scheduler<'a>, d: &'a Datum) -> Result<Pattern> {
    s.call(move |s| async move { pattern_inner(&s, d).await })
        .await
}
async fn patterns<'a>(s: &Scheduler<'a>, ds: &'a [Datum]) -> Result<Vec<Pattern>> {
    let mut out = Vec::with_capacity(ds.len());
    for d in ds {
        out.push(pattern_child(s, d).await?);
    }
    Ok(out)
}
async fn pattern_inner<'a>(s: &Scheduler<'a>, d: &'a Datum) -> Result<Pattern> {
    let kind = match &d.kind {
        K::Quote(_) => {
            let captures = check_quote(s, d, true).await?;
            PatternKind::Quote {
                datum: Box::new(d.clone()),
                template: None,
                captures,
            }
        }
        K::Name(n) => PatternKind::Var(n.clone()),
        K::Discard(n) => PatternKind::Discard(n.clone()),
        K::UpName(_) => {
            let (module, name) = qualified(d)?;
            PatternKind::Constructor {
                module,
                name,
                args: Vec::new(),
                spread: false,
            }
        }
        K::Int(n) => PatternKind::Int {
            negative: false,
            value: n.clone(),
        },
        K::Float(n) => PatternKind::Float {
            negative: false,
            value: *n,
        },
        K::String(x) => PatternKind::String(x.clone()),
        K::Square(xs) => {
            let (items, rest) = match xs.iter().position(|x| symbol(x, "..")) {
                Some(i) if xs.len() == i + 1 => (&xs[..i], Some(None)),
                Some(i) if xs.len() == i + 2 => (&xs[..i], Some(Some(binder(&xs[i + 1])?))),
                Some(_) => return Err(bad(d, "a list pattern")),
                None => (xs.as_slice(), None),
            };
            PatternKind::List {
                items: patterns(s, items).await?,
                rest,
            }
        }
        K::Bits(xs) => {
            let mut out = Vec::with_capacity(xs.len());
            for x in xs {
                out.push(pattern_segment(s, x).await?);
            }
            PatternKind::BitArray(out)
        }
        K::Round(xs) => match xs.as_slice() {
            [] => return Err(bad(d, "a pattern")),
            [h, b, t] if symbol(h, ":") => PatternKind::Typed {
                binder: binder(b)?,
                ty: type_child(s, t).await?,
            },
            [h, n, p] if symbol(h, "=") => PatternKind::Named {
                name: ident(n)?,
                pattern: Box::new(pattern_child(s, p).await?),
            },
            [h, n] if symbol(h, "-") => {
                let K::Int(n) = &n.kind else {
                    return Err(bad(d, "a negative integer pattern"));
                };
                PatternKind::Int {
                    negative: true,
                    value: n.clone(),
                }
            }
            [h, n] if symbol(h, "-.") => {
                let K::Float(n) = n.kind else {
                    return Err(bad(d, "a negative float pattern"));
                };
                PatternKind::Float {
                    negative: true,
                    value: n,
                }
            }
            [h, x, rest] if symbol(h, "<>") => match (&x.kind, &rest.kind) {
                (K::String(prefix), _) => PatternKind::Prefix {
                    prefix: prefix.clone(),
                    suffix: String::new(),
                    rest: binder(rest)?,
                },
                (_, K::String(suffix)) => {
                    let (prefix, rest) = match &x.kind {
                        K::Round(parts) => match parts.as_slice() {
                            [h, p, r] if symbol(h, "<>") => {
                                let K::String(prefix) = &p.kind else {
                                    return Err(bad(d, "one binder between string literals"));
                                };
                                (prefix.clone(), binder(r)?)
                            }
                            _ => return Err(bad(d, "a string prefix or suffix pattern")),
                        },
                        _ => (String::new(), binder(x)?),
                    };
                    PatternKind::Prefix {
                        prefix,
                        suffix: suffix.clone(),
                        rest,
                    }
                }
                _ => {
                    return Err(bad(
                        d,
                        "a string prefix or suffix pattern with exactly one binder",
                    ));
                }
            },
            [h, ps @ ..] if symbol(h, ",") && !ps.is_empty() => {
                PatternKind::Tuple(patterns(s, ps).await?)
            }
            [h, ..] if symbol(h, ".") => {
                let (module, name) = qualified(d)?;
                PatternKind::Constructor {
                    module,
                    name,
                    args: Vec::new(),
                    spread: false,
                }
            }
            [h, ps @ ..] => {
                let (module, name) = qualified(h)?;
                if !name.name.starts_with(|c: char| c.is_ascii_uppercase()) {
                    return Err(bad(h, "a constructor"));
                }
                let spread = ps.last().is_some_and(|p| symbol(p, ".."));
                let ps = if spread { &ps[..ps.len() - 1] } else { ps };
                PatternKind::Constructor {
                    module,
                    name,
                    args: patterns(s, ps).await?,
                    spread,
                }
            }
        },
        _ => return Err(bad(d, "a pattern")),
    };
    Ok(Pattern { kind, span: d.span })
}
async fn segment_parts<'a>(s: &Scheduler<'a>, d: &'a Datum) -> Result<(&'a Datum, Option<Format>)> {
    match round(d) {
        Some([h, value, fmt]) if symbol(h, ":") => {
            let (name, arg) = match round(fmt) {
                Some([name, arg]) => (
                    ident(name)?,
                    Some(Box::new(expression_child(s, arg).await?)),
                ),
                _ => (ident(fmt)?, None),
            };
            Ok((value, Some(Format { name, arg })))
        }
        _ => Ok((d, None)),
    }
}
async fn pattern_segment<'a>(s: &Scheduler<'a>, d: &'a Datum) -> Result<Segment<Pattern>> {
    let (value, format) = segment_parts(s, d).await?;
    Ok(Segment {
        value: pattern_child(s, value).await?,
        format,
        span: d.span,
    })
}
async fn expression_segment<'a>(s: &Scheduler<'a>, d: &'a Datum) -> Result<Segment<Expr>> {
    let (value, format) = segment_parts(s, d).await?;
    Ok(Segment {
        value: expression_child(s, value).await?,
        format,
        span: d.span,
    })
}
async fn argument_inner<'a>(s: &Scheduler<'a>, d: &'a Datum) -> Result<Arg> {
    if let Some([h, n, e]) = round(d)
        && symbol(h, ":")
    {
        return Ok(Arg {
            label: Some(ident(n)?),
            value: expression_child(s, e).await?,
        });
    }
    Ok(Arg {
        label: None,
        value: expression_child(s, d).await?,
    })
}
fn argument(d: &Datum) -> Result<Arg> {
    run(|s| async move { argument_inner(&s, d).await })
}

fn binary(d: &Datum) -> Option<BinOp> {
    Some(match &d.kind {
        K::Symbol(s) => match s.as_str() {
            "||" => BinOp::Or,
            "&&" => BinOp::And,
            "==" => BinOp::Eq,
            "!=" => BinOp::NotEq,
            "<" => BinOp::Lt,
            "<=" => BinOp::LtEq,
            ">" => BinOp::Gt,
            ">=" => BinOp::GtEq,
            "<." => BinOp::LtFloat,
            "<=." => BinOp::LtEqFloat,
            ">." => BinOp::GtFloat,
            ">=." => BinOp::GtEqFloat,
            "<>" => BinOp::Concat,
            "|" => BinOp::Pipe,
            "+" => BinOp::Add,
            "-" => BinOp::Sub,
            "+." => BinOp::AddFloat,
            "-." => BinOp::SubFloat,
            "*" => BinOp::Mul,
            "/" => BinOp::Div,
            "%" => BinOp::Rem,
            "*." => BinOp::MulFloat,
            "/." => BinOp::DivFloat,
            _ => return None,
        },
        _ => return None,
    })
}

/// Read a datum in expression position without first printing normal syntax.
pub fn parse_expression(d: &Datum) -> Result<Expr> {
    run(|s| async move { expression_inner(&s, d).await })
}
async fn expression_child<'a>(s: &Scheduler<'a>, d: &'a Datum) -> Result<Expr> {
    s.call(move |s| async move { expression_inner(&s, d).await })
        .await
}
async fn expressions<'a>(s: &Scheduler<'a>, ds: &'a [Datum]) -> Result<Vec<Expr>> {
    let mut out = Vec::with_capacity(ds.len());
    for d in ds {
        out.push(expression_child(s, d).await?);
    }
    Ok(out)
}
async fn expression_inner<'a>(s: &Scheduler<'a>, d: &'a Datum) -> Result<Expr> {
    let kind = match &d.kind {
        K::Quote(_) => {
            check_quote(s, d, false).await?;
            ExprKind::Quote(Box::new(d.clone()))
        }
        K::Int(n) => ExprKind::Int(n.clone()),
        K::Float(n) => ExprKind::Float(*n),
        K::String(x) => ExprKind::String(x.clone()),
        K::Name(_) | K::UpName(_) => {
            let (module, name) = qualified(d)?;
            ExprKind::Name { module, name }
        }
        K::Square(xs) => {
            let (items, rest) = match xs.iter().position(|x| symbol(x, "..")) {
                Some(i) if i > 0 && xs.len() == i + 2 => (
                    &xs[..i],
                    Some(Box::new(expression_child(s, &xs[i + 1]).await?)),
                ),
                Some(_) => return Err(bad(d, "a list expression")),
                None => (xs.as_slice(), None),
            };
            ExprKind::List {
                items: expressions(s, items).await?,
                rest,
            }
        }
        K::Bits(xs) => {
            let mut out = Vec::with_capacity(xs.len());
            for x in xs {
                out.push(expression_segment(s, x).await?);
            }
            ExprKind::BitArray(out)
        }
        K::Curly(xs) if xs.is_empty() || xs.first().is_some_and(is_clause) => {
            ExprKind::Clauses(clauses(s, xs).await?)
        }
        K::Curly(_) => ExprKind::Block(block_inner(s, d).await?),
        K::Round(xs) if record_start(xs) => record_expression(s, d, xs).await?,
        K::Round(xs) => match xs.as_slice() {
            // `()` is the empty data, as the normal surface lowers it
            // (syntax three §7.1 row 7): the empty tuple.
            [] => ExprKind::Tuple(Vec::new()),
            [h, es @ ..] if symbol(h, ",") && !es.is_empty() => {
                ExprKind::Tuple(expressions(s, es).await?)
            }
            [h, subject, cs] if symbol(h, ".") && matches!(cs.kind, K::Curly(_)) => {
                let K::Curly(cs) = &cs.kind else {
                    unreachable!()
                };
                ExprKind::Case {
                    subject: Box::new(expression_child(s, subject).await?),
                    clauses: clauses(s, cs).await?,
                }
            }
            [h, target, field] if symbol(h, ".") => match &field.kind {
                K::Int(n) => ExprKind::TupleIndex {
                    target: Box::new(expression_child(s, target).await?),
                    index: n.clone(),
                    index_span: field.span,
                },
                K::UpName(_) if matches!(target.kind, K::Name(_)) => ExprKind::Name {
                    module: Some(ident(target)?),
                    name: ident(field)?,
                },
                K::Name(_) | K::UpName(_) => ExprKind::Field {
                    target: Box::new(expression_child(s, target).await?),
                    name: ident(field)?,
                },
                _ => return Err(bad(field, "a field")),
            },
            [h, params, ret, body] if symbol(h, "->") => {
                let ps = round(params).ok_or_else(|| bad(params, "function parameters"))?;
                let mut params = Vec::with_capacity(ps.len());
                for p in ps {
                    if let Some([h, n, ty]) = round(p)
                        && symbol(h, "!")
                    {
                        params.push(Param::Env {
                            name: ident(n)?,
                            ty: type_child(s, ty).await?,
                        });
                    } else {
                        params.push(Param::Pattern(pattern_child(s, p).await?));
                    }
                }
                let ret = if matches!(&ret.kind, K::Discard(x) if x == "_") {
                    None
                } else {
                    Some(result_type_inner(s, ret).await?)
                };
                let body = if matches!(body.kind, K::Curly(_)) {
                    block_inner(s, body).await?
                } else {
                    Block {
                        stmts: vec![Stmt::Expr(expression_child(s, body).await?)],
                        span: body.span,
                    }
                };
                ExprKind::Fn(Box::new(Function { params, ret, body }))
            }
            [h, e] if symbol(h, "-") || symbol(h, "-.") => ExprKind::Unary {
                op: if symbol(h, "-") {
                    UnOp::Neg
                } else {
                    UnOp::NegFloat
                },
                expr: Box::new(expression_child(s, e).await?),
            },
            [h, a, b] if binary(h).is_some() => ExprKind::Binary {
                op: binary(h).unwrap(),
                lhs: Box::new(expression_child(s, a).await?),
                rhs: Box::new(expression_child(s, b).await?),
            },
            [constructor, dots, base, fields @ ..] if symbol(dots, "..") => {
                let (_, name) = qualified(constructor)?;
                if !name.name.starts_with(|c: char| c.is_ascii_uppercase()) {
                    return Err(bad(constructor, "a record constructor"));
                }
                let mut result = Vec::with_capacity(fields.len());
                for field in fields {
                    match round(field) {
                        Some([h, n, e]) if symbol(h, ":") => {
                            result.push((ident(n)?, expression_child(s, e).await?))
                        }
                        _ => return Err(bad(field, "an updated field")),
                    }
                }
                ExprKind::RecordUpdate {
                    constructor: Box::new(expression_child(s, constructor).await?),
                    base: Box::new(expression_child(s, base).await?),
                    fields: result,
                }
            }
            [callee, args @ ..] if !matches!(callee.kind, K::Symbol(_)) => {
                let callee = Box::new(expression_child(s, callee).await?);
                let mut result = Vec::with_capacity(args.len());
                for arg in args {
                    result.push(argument_inner(s, arg).await?);
                }
                ExprKind::Call {
                    callee,
                    args: result,
                }
            }
            _ => return Err(bad(d, "an expression")),
        },
        _ => return Err(bad(d, "an expression")),
    };
    Ok(Expr { kind, span: d.span })
}
fn is_clause(d: &Datum) -> bool {
    matches!(round(d), Some([h, _, _]) if symbol(h, "->"))
}

async fn check_quote<'a>(
    s: &Scheduler<'a>,
    d: &'a Datum,
    pattern: bool,
) -> Result<Vec<(bool, Pattern)>> {
    let K::Quote(body) = &d.kind else {
        unreachable!("quote check requires a quote")
    };
    let mut work = vec![(&**body, false, true)];
    let mut captures = Vec::new();
    while let Some((d, in_sequence, last)) = work.pop() {
        match &d.kind {
            K::Unquote(inner) => {
                if pattern {
                    captures.push((false, pattern_child(s, inner).await?));
                } else {
                    expression_child(s, inner).await?;
                }
            }
            K::Splice(inner) => {
                if !in_sequence || (pattern && !last) {
                    return Err(bad(d, "a splice in this position"));
                }
                if pattern {
                    captures.push((true, pattern_child(s, inner).await?));
                } else {
                    expression_child(s, inner).await?;
                }
            }
            K::Round(xs) | K::Square(xs) | K::Curly(xs) | K::Bits(xs) => {
                work.extend(
                    xs.iter()
                        .enumerate()
                        .rev()
                        .map(|(i, child)| (child, true, i + 1 == xs.len())),
                );
            }
            K::Quote(_) => return Err(bad(d, "a nested quote")),
            _ => {}
        }
    }
    Ok(captures)
}
async fn clauses<'a>(s: &Scheduler<'a>, ds: &'a [Datum]) -> Result<Vec<Clause>> {
    let mut out = Vec::with_capacity(ds.len());
    for d in ds {
        out.push(clause_inner(s, d).await?);
    }
    Ok(out)
}
async fn clause_inner<'a>(s: &Scheduler<'a>, d: &'a Datum) -> Result<Clause> {
    let Some([h, p, body]) = round(d) else {
        return Err(bad(d, "a clause"));
    };
    if !symbol(h, "->") {
        return Err(bad(d, "a clause"));
    }
    let (p, guard) = match round(p) {
        Some([h, p, g]) if symbol(h, "?") => (p, Some(expression_child(s, g).await?)),
        _ => (p, None),
    };
    let patterns = match round(p) {
        Some([h, ps @ ..]) if symbol(h, "|") && ps.len() >= 2 => {
            if ps[..ps.len() - 1]
                .iter()
                .any(super::to_source::ends_in_type_test)
            {
                return Err(bad(p, "a clause choice ending in a type test"));
            }
            patterns(s, ps).await?
        }
        _ => vec![pattern_child(s, p).await?],
    };
    Ok(Clause {
        patterns,
        guard,
        body: expression_child(s, body).await?,
        span: d.span,
    })
}
async fn block_inner<'a>(s: &Scheduler<'a>, d: &'a Datum) -> Result<Block> {
    let K::Curly(xs) = &d.kind else {
        return Err(bad(d, "a function body"));
    };
    let mut stmts = Vec::with_capacity(xs.len());
    for x in xs {
        stmts.push(statement_inner(s, x).await?);
    }
    if !matches!(stmts.last(), Some(Stmt::Expr(_))) {
        return Err(Diagnostic::new(
            Code::BlockEnd,
            d.span,
            "a block must end with an expression",
        ));
    }
    Ok(Block {
        stmts,
        span: d.span,
    })
}
async fn statement_inner<'a>(s: &Scheduler<'a>, d: &'a Datum) -> Result<Stmt> {
    if let Some([h, left, right]) = round(d) {
        if symbol(h, "!") {
            return Ok(Stmt::Env {
                ty: type_child(s, left).await?,
                value: expression_child(s, right).await?,
                span: d.span,
            });
        }
        if symbol(h, "=") {
            return Ok(Stmt::Bind(binding_inner(s, d, left, right).await?));
        }
    }
    Ok(Stmt::Expr(expression_child(s, d).await?))
}
fn binding<'a>(d: &'a Datum, left: &'a Datum, right: &'a Datum) -> Result<Binding> {
    run(|s| async move { binding_inner(&s, d, left, right).await })
}
async fn binding_inner<'a>(
    s: &Scheduler<'a>,
    d: &'a Datum,
    left: &'a Datum,
    right: &'a Datum,
) -> Result<Binding> {
    let (left, annotation) = match round(left) {
        Some([h, n, t]) if symbol(h, ":") && round(n).is_some() => (n, Some(t)),
        _ => (left, None),
    };
    let (pattern, type_params) = match round(left) {
        Some([n, args @ ..])
            if matches!(n.kind, K::Name(_))
                && !args.is_empty()
                && args.iter().all(|x| matches!(x.kind, K::UpName(_))) =>
        {
            let type_params = args.iter().map(ident).collect::<Result<_>>()?;
            let pattern = if let Some(ty) = annotation {
                Pattern {
                    kind: PatternKind::Typed {
                        binder: binder(n)?,
                        ty: type_child(s, ty).await?,
                    },
                    span: left.span.to(ty.span),
                }
            } else {
                pattern_child(s, n).await?
            };
            (pattern, type_params)
        }
        _ if annotation.is_none() => (pattern_child(s, left).await?, Vec::new()),
        _ => return Err(bad(left, "a binding head")),
    };
    Ok(Binding {
        pattern,
        value: expression_child(s, right).await?,
        span: d.span,
        type_params,
    })
}

/// Read a complete module directly from its datums.
pub fn parse_module(document: &Document) -> Result<Module> {
    let mut module = Module::default();
    let mut stage = 0;
    for d in &document.datums {
        for doc in &d.docs {
            if doc.module {
                module.doc.push(doc.text.clone());
            }
        }
        if let Some([h, rest @ ..]) = round(d)
            && symbol(h, ",")
        {
            if stage >= 2 || rest.is_empty() {
                return Err(bad(d, "a module header"));
            }
            if rest.iter().any(is_import) {
                if stage != 0 {
                    return Err(bad(d, "an imports header"));
                }
                module.imports = rest.iter().map(import).collect::<Result<_>>()?;
                module.imports_span = Some(d.span);
                stage = 1;
            } else {
                module.exports = rest.iter().map(export).collect::<Result<_>>()?;
                module.exports_span = Some(d.span);
                stage = 2;
            }
            continue;
        }
        stage = 3;
        let mut item = parse_item(d)?;
        item.doc = d
            .docs
            .iter()
            .filter(|doc| !doc.module)
            .map(|doc| doc.text.clone())
            .collect();
        module.items.push(item);
    }
    for doc in &document.trailing_docs {
        if doc.module {
            module.doc.push(doc.text.clone());
        }
    }
    Ok(module)
}
fn is_import(d: &Datum) -> bool {
    let mut work: Vec<_> = round(d).unwrap_or(&[]).iter().collect();
    while let Some(d) = work.pop() {
        if symbol(d, "/") {
            return true;
        }
        if let Some(xs) = round(d) {
            work.extend(xs);
        }
    }
    false
}
fn import(d: &Datum) -> Result<ImportItem> {
    let mut alias = None;
    let mut names = Vec::new();
    let mut path = d;
    if let Some([h, a, b]) = round(d)
        && symbol(h, "=")
    {
        if matches!(a.kind, K::Name(_)) {
            alias = Some(ident(a)?);
        } else {
            names = import_names(a)?;
        }
        path = b;
    }
    if let Some([h, a, ns, p]) = round(d)
        && symbol(h, "=")
    {
        alias = Some(ident(a)?);
        names = import_names(ns)?;
        path = p;
    }
    let Some([h, parts @ ..]) = round(path) else {
        return Err(bad(path, "a module path"));
    };
    if !symbol(h, "/") || parts.len() < 2 || parts.iter().any(|x| !matches!(x.kind, K::Name(_))) {
        return Err(bad(path, "a module path"));
    }
    Ok(ImportItem {
        alias,
        names,
        path: parts.iter().map(ident).collect::<Result<_>>()?,
        span: d.span,
    })
}
fn import_names(d: &Datum) -> Result<Vec<Ident>> {
    let Some([h, names @ ..]) = round(d) else {
        return Err(bad(d, "import names"));
    };
    if !symbol(h, ",") || names.is_empty() {
        return Err(bad(d, "import names"));
    }
    names.iter().map(ident).collect()
}
fn export(d: &Datum) -> Result<ExportItem> {
    match round(d) {
        Some([name, dots]) if symbol(dots, "..") => Ok(ExportItem {
            name: ident(name)?,
            constructors: true,
            span: d.span,
        }),
        _ => Ok(ExportItem {
            name: ident(d)?,
            constructors: false,
            span: d.span,
        }),
    }
}
/// Read one item, without accepting module headers.
pub fn parse_item(d: &Datum) -> Result<Item> {
    let mut current = d;
    let mut attributes = Vec::new();
    while let Some([h, attr, body]) = round(current) {
        if !symbol(h, "@") {
            break;
        }
        attributes.push(attribute(attr)?);
        current = body;
    }
    let xs = round(current).ok_or_else(|| bad(current, "an item"))?;
    let kind = match xs {
        [h, left, right] if symbol(h, "=") && upper_head(left) => {
            ItemKind::TypeDef(type_def(left, Some(right), None)?)
        }
        [h, left, right] if symbol(h, "=") => ItemKind::Binding(binding(current, left, right)?),
        [h, left, ty] if symbol(h, ":") => {
            let (name, type_params) = binding_head(left)?;
            ItemKind::Declaration {
                name,
                type_params,
                ty: parse_type(ty)?,
            }
        }
        [head, body] if upper_head(head) && matches!(body.kind, K::Curly(_)) => {
            ItemKind::TypeDef(type_def(head, None, Some(body))?)
        }
        [head, args @ ..] => {
            let (module, name) = qualified(head)?;
            ItemKind::MacroItem {
                module,
                name,
                args: args.iter().map(argument).collect::<Result<_>>()?,
            }
        }
        _ => return Err(bad(current, "an item")),
    };
    Ok(Item {
        doc: Vec::new(),
        attributes,
        kind,
        span: d.span,
    })
}
/// Read a macro's item result: one item or a top-level curly list of items.
pub fn parse_items(d: &Datum) -> Result<Vec<Item>> {
    match &d.kind {
        K::Curly(items) => items.iter().map(parse_item).collect(),
        _ => Ok(vec![parse_item(d)?]),
    }
}

fn attribute(d: &Datum) -> Result<Attribute> {
    let (head, args) = match round(d) {
        Some([h, args @ ..]) if !symbol(h, ".") => (h, Some(args)),
        _ => (d, None),
    };
    let (module, name) = qualified(head)?;
    let mut strings = Vec::new();
    let call = if let Some(args) = args {
        if !args.is_empty() && args.iter().all(|d| matches!(d.kind, K::String(_))) {
            strings = args
                .iter()
                .map(|d| {
                    let K::String(s) = &d.kind else {
                        unreachable!()
                    };
                    (s.clone(), d.span)
                })
                .collect();
            None
        } else {
            Some((args.iter().map(argument).collect::<Result<_>>()?, d.span))
        }
    } else {
        None
    };
    Ok(Attribute {
        module,
        name,
        args: strings,
        call,
        span: d.span,
    })
}
fn binding_head(d: &Datum) -> Result<(Ident, Vec<Ident>)> {
    if let Some([n, params @ ..]) = round(d) {
        if params.is_empty() {
            return Err(bad(d, "a binding head"));
        }
        return Ok((ident(n)?, params.iter().map(ident).collect::<Result<_>>()?));
    }
    Ok((ident(d)?, Vec::new()))
}
fn upper_head(d: &Datum) -> bool {
    matches!(d.kind, K::UpName(_))
        || round(d).is_some_and(|xs| xs.first().is_some_and(|n| matches!(n.kind, K::UpName(_))))
}
fn type_def(head: &Datum, alias: Option<&Datum>, custom: Option<&Datum>) -> Result<TypeDef> {
    let (name, written) = match round(head) {
        Some([n, args @ ..]) if !args.is_empty() => (
            ident(n)?,
            args.iter().map(parse_type).collect::<Result<_>>()?,
        ),
        Some(_) => return Err(bad(head, "a type head")),
        None => (ident(head)?, Vec::new()),
    };
    let simple = written
        .iter()
        .all(|t| matches!(&t.kind, TypeKind::Named { module: None, args, .. } if args.is_empty()));
    let mut params = if simple {
        written
            .iter()
            .map(|t| {
                let TypeKind::Named { name, .. } = &t.kind else {
                    unreachable!()
                };
                name.clone()
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut actual_head = if simple { Vec::new() } else { written };
    let mut alias_pattern = false;
    let body = if let Some(a) = alias {
        let ty = parse_type(a)?;
        alias_pattern = (!params.is_empty() || !actual_head.is_empty())
            && matches!(ty.kind, TypeKind::ValueOf { .. });
        if alias_pattern {
            actual_head.extend(params.drain(..).map(|name| TypeExpr {
                span: name.span,
                kind: TypeKind::Named {
                    module: None,
                    name,
                    args: Vec::new(),
                },
            }));
        }
        TypeBody::Alias(ty)
    } else {
        let d = custom.expect("type definition has a body");
        let K::Curly(cs) = &d.kind else {
            return Err(bad(d, "a type body"));
        };
        TypeBody::Custom(cs.iter().map(constructor).collect::<Result<_>>()?)
    };
    Ok(TypeDef {
        alias_pattern,
        name,
        params,
        head: actual_head,
        body,
    })
}
fn constructor(d: &Datum) -> Result<Constructor> {
    let (name, fields) = match round(d) {
        Some([n, fs @ ..]) if !fs.is_empty() => {
            (ident(n)?, fs.iter().map(field).collect::<Result<_>>()?)
        }
        None => (ident(d)?, Vec::new()),
        _ => return Err(bad(d, "a constructor")),
    };
    Ok(Constructor {
        name,
        fields,
        span: d.span,
    })
}
fn field(d: &Datum) -> Result<Field> {
    let (label, ty) = match round(d) {
        Some([h, n, ty]) if symbol(h, ":") => (Some(ident(n)?), parse_type(ty)?),
        _ => (None, parse_type(d)?),
    };
    Ok(Field {
        label,
        ty,
        span: d.span,
    })
}
