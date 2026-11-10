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
fn types(ds: &[Datum]) -> Result<Vec<TypeExpr>> {
    ds.iter().map(parse_type).collect()
}

/// Read a datum in a type position, retaining the original span at each node.
pub fn parse_type(d: &Datum) -> Result<TypeExpr> {
    let kind = match &d.kind {
        K::Discard(s) if s == "_" => TypeKind::Hole,
        K::Int(n) => TypeKind::Lit(LitValue::Int(BigInt::from_biguint(Sign::Plus, n.clone()))),
        K::String(s) => TypeKind::Lit(LitValue::Str(s.clone())),
        K::Name(_) | K::UpName(_) => named_type(d, &[])?,
        K::Curly(xs) if xs.len() == 1 => return parse_type(&xs[0]),
        K::Round(xs) => match xs.as_slice() {
            [h, n] if symbol(h, "-") && matches!(n.kind, K::Int(_)) => {
                let K::Int(n) = &n.kind else { unreachable!() };
                TypeKind::Lit(LitValue::Int(-BigInt::from_biguint(Sign::Plus, n.clone())))
            }
            [h, ..] if symbol(h, ".") => named_type(d, &[])?,
            [h, rest @ ..] if symbol(h, ",") && !rest.is_empty() => TypeKind::Tuple(types(rest)?),
            [h, rest @ ..] if symbol(h, "|") && rest.len() >= 2 => TypeKind::Union(types(rest)?),
            [h, rest @ ..] if symbol(h, "&") && rest.len() >= 2 => TypeKind::Inter(types(rest)?),
            [h, rest @ ..] if symbol(h, "-") && rest.len() >= 2 => TypeKind::Diff(types(rest)?),
            [h, ps, ret] if symbol(h, "->") => {
                let ps = round(ps).ok_or_else(|| bad(ps, "function type parameters"))?;
                let params = ps
                    .iter()
                    .map(|p| {
                        if let Some([h, n, t]) = round(p)
                            && symbol(h, ":")
                        {
                            return Ok(FnParamType {
                                label: Some(ident(n)?),
                                ty: parse_type(t)?,
                            });
                        }
                        Ok(FnParamType {
                            label: None,
                            ty: parse_type(p)?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let result = result_type(ret)?;
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
            [head, args @ ..] if !args.is_empty() => named_type(head, args)?,
            _ => return Err(bad(d, "a type")),
        },
        _ => return Err(bad(d, "a type")),
    };
    Ok(TypeExpr { kind, span: d.span })
}
fn named_type(head: &Datum, args: &[Datum]) -> Result<TypeKind> {
    let (module, name) = qualified(head)?;
    if name.name.starts_with(|c: char| c.is_ascii_uppercase()) {
        Ok(TypeKind::Named {
            module,
            name,
            args: types(args)?,
        })
    } else if args.is_empty() {
        Ok(TypeKind::ValueOf { module, name })
    } else {
        Ok(TypeKind::Call {
            module,
            name,
            args: types(args)?,
        })
    }
}
fn result_type(d: &Datum) -> Result<RetType> {
    if let Some([h, r, e]) = round(d) {
        if symbol(h, "!") {
            let mut ret = result_type(r)?;
            ret.env = Some(parse_type(e)?);
            return Ok(ret);
        }
        if symbol(h, "^") {
            return Ok(RetType {
                value: parse_type(r)?,
                err: if matches!(&r.kind, K::Discard(s) if s == "_")
                    && matches!(&e.kind, K::UpName(s) if s == "Never")
                {
                    None
                } else {
                    Some(parse_type(e)?)
                },
                env: None,
            });
        }
    }
    Ok(RetType {
        value: parse_type(d)?,
        err: None,
        env: None,
    })
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
    let kind = match &d.kind {
        K::Quote(_) => {
            check_quote(d, true)?;
            PatternKind::Quote(Box::new(d.clone()))
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
        K::String(s) => PatternKind::String(s.clone()),
        K::Square(xs) => {
            let (items, rest) = match xs.iter().position(|x| symbol(x, "..")) {
                Some(i) if xs.len() == i + 1 => (&xs[..i], Some(None)),
                Some(i) if xs.len() == i + 2 => (&xs[..i], Some(Some(binder(&xs[i + 1])?))),
                Some(_) => return Err(bad(d, "a list pattern")),
                None => (xs.as_slice(), None),
            };
            PatternKind::List {
                items: items.iter().map(parse_pattern).collect::<Result<_>>()?,
                rest,
            }
        }
        K::Bits(xs) => PatternKind::BitArray(
            xs.iter()
                .map(|x| segment(x, parse_pattern))
                .collect::<Result<_>>()?,
        ),
        K::Round(xs) => match xs.as_slice() {
            [] => PatternKind::Tuple(Vec::new()),
            [h, b, t] if symbol(h, ":") => PatternKind::Typed {
                binder: binder(b)?,
                ty: parse_type(t)?,
            },
            [h, n, p] if symbol(h, "=") => PatternKind::Named {
                name: ident(n)?,
                pattern: Box::new(parse_pattern(p)?),
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
            [h, s, rest] if symbol(h, "<>") => {
                let K::String(s) = &s.kind else {
                    return Err(bad(d, "a prefix pattern"));
                };
                PatternKind::Prefix {
                    prefix: s.clone(),
                    rest: binder(rest)?,
                }
            }
            [h, ps @ ..] if symbol(h, ",") && !ps.is_empty() => {
                PatternKind::Tuple(ps.iter().map(parse_pattern).collect::<Result<_>>()?)
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
                    args: ps.iter().map(parse_pattern).collect::<Result<_>>()?,
                    spread,
                }
            }
        },
        _ => return Err(bad(d, "a pattern")),
    };
    Ok(Pattern { kind, span: d.span })
}

fn segment<T>(d: &Datum, parse: fn(&Datum) -> Result<T>) -> Result<Segment<T>> {
    let (value, format) = match round(d) {
        Some([h, value, fmt]) if symbol(h, ":") => {
            let (name, arg) = match round(fmt) {
                Some([name, arg]) => (ident(name)?, Some(Box::new(parse_expression(arg)?))),
                _ => (ident(fmt)?, None),
            };
            (parse(value)?, Some(Format { name, arg }))
        }
        _ => (parse(d)?, None),
    };
    Ok(Segment {
        value,
        format,
        span: d.span,
    })
}
fn argument(d: &Datum) -> Result<Arg> {
    if let Some([h, n, e]) = round(d)
        && symbol(h, ":")
    {
        return Ok(Arg {
            label: Some(ident(n)?),
            value: parse_expression(e)?,
        });
    }
    Ok(Arg {
        label: None,
        value: parse_expression(d)?,
    })
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
    let kind = match &d.kind {
        K::Quote(_) => {
            check_quote(d, false)?;
            ExprKind::Quote(Box::new(d.clone()))
        }
        K::Int(n) => ExprKind::Int(n.clone()),
        K::Float(n) => ExprKind::Float(*n),
        K::String(s) => ExprKind::String(s.clone()),
        K::Name(_) | K::UpName(_) => {
            let (module, name) = qualified(d)?;
            ExprKind::Name { module, name }
        }
        K::Square(xs) => {
            let (items, rest) = match xs.iter().position(|x| symbol(x, "..")) {
                Some(i) if i > 0 && xs.len() == i + 2 => {
                    (&xs[..i], Some(Box::new(parse_expression(&xs[i + 1])?)))
                }
                Some(_) => return Err(bad(d, "a list expression")),
                None => (xs.as_slice(), None),
            };
            ExprKind::List {
                items: items.iter().map(parse_expression).collect::<Result<_>>()?,
                rest,
            }
        }
        K::Bits(xs) => ExprKind::BitArray(
            xs.iter()
                .map(|x| segment(x, parse_expression))
                .collect::<Result<_>>()?,
        ),
        K::Curly(xs) if xs.is_empty() || xs.first().is_some_and(is_clause) => {
            ExprKind::Clauses(xs.iter().map(clause).collect::<Result<_>>()?)
        }
        K::Curly(_) => ExprKind::Block(block(d)?),
        K::Round(xs) => match xs.as_slice() {
            [] => ExprKind::Tuple(Vec::new()),
            [h, es @ ..] if symbol(h, ",") && !es.is_empty() => {
                ExprKind::Tuple(es.iter().map(parse_expression).collect::<Result<_>>()?)
            }
            [h, subject, cs] if symbol(h, ".") && matches!(cs.kind, K::Curly(_)) => {
                let K::Curly(cs) = &cs.kind else {
                    unreachable!()
                };
                ExprKind::Case {
                    subject: Box::new(parse_expression(subject)?),
                    clauses: cs.iter().map(clause).collect::<Result<_>>()?,
                }
            }
            [h, target, field] if symbol(h, ".") => match &field.kind {
                K::Int(n) => ExprKind::TupleIndex {
                    target: Box::new(parse_expression(target)?),
                    index: n.clone(),
                    index_span: field.span,
                },
                K::UpName(_) if matches!(target.kind, K::Name(_)) => ExprKind::Name {
                    module: Some(ident(target)?),
                    name: ident(field)?,
                },
                K::Name(_) | K::UpName(_) => ExprKind::Field {
                    target: Box::new(parse_expression(target)?),
                    name: ident(field)?,
                },
                _ => return Err(bad(field, "a field")),
            },
            [h, params, ret, body] if symbol(h, "->") => {
                let params = round(params).ok_or_else(|| bad(params, "function parameters"))?;
                let params = params
                    .iter()
                    .map(|p| {
                        if let Some([h, n, ty]) = round(p)
                            && symbol(h, "!")
                        {
                            return Ok(Param::Env {
                                name: ident(n)?,
                                ty: parse_type(ty)?,
                            });
                        }
                        Ok(Param::Pattern(parse_pattern(p)?))
                    })
                    .collect::<Result<_>>()?;
                let ret = if matches!(&ret.kind, K::Discard(s) if s == "_") {
                    None
                } else {
                    Some(result_type(ret)?)
                };
                let body = if matches!(body.kind, K::Curly(_)) {
                    block(body)?
                } else {
                    Block {
                        stmts: vec![Stmt::Expr(parse_expression(body)?)],
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
                expr: Box::new(parse_expression(e)?),
            },
            [h, a, b] if binary(h).is_some() => ExprKind::Binary {
                op: binary(h).unwrap(),
                lhs: Box::new(parse_expression(a)?),
                rhs: Box::new(parse_expression(b)?),
            },
            [constructor, dots, base, fields @ ..] if symbol(dots, "..") => {
                let (_, name) = qualified(constructor)?;
                if !name.name.starts_with(|c: char| c.is_ascii_uppercase()) {
                    return Err(bad(constructor, "a record constructor"));
                }
                let fields = fields
                    .iter()
                    .map(|field| match round(field) {
                        Some([h, n, e]) if symbol(h, ":") => Ok((ident(n)?, parse_expression(e)?)),
                        _ => Err(bad(field, "an updated field")),
                    })
                    .collect::<Result<_>>()?;
                ExprKind::RecordUpdate {
                    constructor: Box::new(parse_expression(constructor)?),
                    base: Box::new(parse_expression(base)?),
                    fields,
                }
            }
            [callee, args @ ..] if !matches!(callee.kind, K::Symbol(_)) => ExprKind::Call {
                callee: Box::new(parse_expression(callee)?),
                args: args.iter().map(argument).collect::<Result<_>>()?,
            },
            _ => return Err(bad(d, "an expression")),
        },
        _ => return Err(bad(d, "an expression")),
    };
    Ok(Expr { kind, span: d.span })
}
fn is_clause(d: &Datum) -> bool {
    matches!(round(d), Some([h, _, _]) if symbol(h, "->"))
}

fn check_quote(d: &Datum, pattern: bool) -> Result<()> {
    let K::Quote(body) = &d.kind else {
        unreachable!("quote check requires a quote")
    };
    fn part(d: &Datum, pattern: bool, in_sequence: bool, last: bool) -> Result<()> {
        match &d.kind {
            K::Unquote(inner) => {
                if pattern {
                    parse_pattern(inner)?;
                } else {
                    parse_expression(inner)?;
                }
            }
            K::Splice(inner) => {
                if !in_sequence || (pattern && !last) {
                    return Err(bad(d, "a splice in this position"));
                }
                if pattern {
                    parse_pattern(inner)?;
                } else {
                    parse_expression(inner)?;
                }
            }
            K::Round(xs) | K::Square(xs) | K::Curly(xs) | K::Bits(xs) => {
                for (i, child) in xs.iter().enumerate() {
                    part(child, pattern, true, i + 1 == xs.len())?;
                }
            }
            K::Quote(_) => return Err(bad(d, "a nested quote")),
            _ => {}
        }
        Ok(())
    }
    part(body, pattern, false, true)
}
fn clause(d: &Datum) -> Result<Clause> {
    let Some([h, p, body]) = round(d) else {
        return Err(bad(d, "a clause"));
    };
    if !symbol(h, "->") {
        return Err(bad(d, "a clause"));
    }
    let (p, guard) = match round(p) {
        Some([h, p, g]) if symbol(h, "?") => (p, Some(parse_expression(g)?)),
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
            ps.iter().map(parse_pattern).collect::<Result<_>>()?
        }
        _ => vec![parse_pattern(p)?],
    };
    Ok(Clause {
        patterns,
        guard,
        body: parse_expression(body)?,
        span: d.span,
    })
}
fn block(d: &Datum) -> Result<Block> {
    let K::Curly(xs) = &d.kind else {
        return Err(bad(d, "a function body"));
    };
    let stmts = xs.iter().map(statement).collect::<Result<Vec<_>>>()?;
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
fn statement(d: &Datum) -> Result<Stmt> {
    if let Some([h, left, right]) = round(d) {
        if symbol(h, "!") {
            return Ok(Stmt::Env {
                ty: parse_type(left)?,
                value: parse_expression(right)?,
                span: d.span,
            });
        }
        if symbol(h, "=") {
            return Ok(Stmt::Bind(binding(d, left, right)?));
        }
    }
    Ok(Stmt::Expr(parse_expression(d)?))
}
fn binding(d: &Datum, left: &Datum, right: &Datum) -> Result<Binding> {
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
                        ty: parse_type(ty)?,
                    },
                    span: left.span.to(ty.span),
                }
            } else {
                parse_pattern(n)?
            };
            (pattern, type_params)
        }
        _ if annotation.is_none() => (parse_pattern(left)?, Vec::new()),
        _ => return Err(bad(left, "a binding head")),
    };
    Ok(Binding {
        pattern,
        value: parse_expression(right)?,
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
        let mut item = item(d)?;
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
    round(d).is_some_and(|xs| xs.iter().any(|x| symbol(x, "/") || is_import(x)))
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
fn item(d: &Datum) -> Result<Item> {
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
        Some([n, args @ ..]) if !args.is_empty() => (ident(n)?, types(args)?),
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
