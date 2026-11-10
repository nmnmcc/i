//! Write the core AST as Lisp datums, following the position tables in meta §2.3.
use crate::ast::*;
use crate::print::string_literal;

fn form(head: &str, parts: impl IntoIterator<Item = String>) -> String {
    let parts = parts.into_iter().collect::<Vec<_>>();
    if parts.is_empty() {
        format!("({head})")
    } else {
        format!("({head} {})", parts.join(" "))
    }
}
fn braced(open: &str, close: &str, parts: impl IntoIterator<Item = String>) -> String {
    format!(
        "{open}{}{close}",
        parts.into_iter().collect::<Vec<_>>().join(" ")
    )
}
fn name(n: &Ident) -> String {
    n.name.clone()
}
fn qualified(module: &Option<Ident>, n: &Ident) -> String {
    module
        .as_ref()
        .map_or_else(|| name(n), |m| form(".", [name(m), name(n)]))
}
fn float(n: f64) -> String {
    format!("{n:?}")
}

/// Render a normal surface module in the Lisp surface.
pub fn print_module(m: &Module) -> String {
    let mut lines = Vec::new();
    lines.extend(m.doc.iter().map(|d| format!("////{d}")));
    if !m.imports.is_empty() {
        lines.push(form(
            ",",
            m.imports.iter().map(|i| {
                let path = form("/", i.path.iter().map(name));
                match (&i.alias, i.names.is_empty()) {
                    (None, true) => path,
                    (Some(a), true) => form("=", [name(a), path]),
                    (None, false) => form("=", [form(",", i.names.iter().map(name)), path]),
                    (Some(a), false) => {
                        form("=", [name(a), form(",", i.names.iter().map(name)), path])
                    }
                }
            }),
        ));
    }
    if !m.exports.is_empty() {
        lines.push(form(
            ",",
            m.exports.iter().map(|e| {
                if e.constructors {
                    form(&e.name.name, ["..".into()])
                } else {
                    name(&e.name)
                }
            }),
        ));
    }
    for item in &m.items {
        lines.extend(item.doc.iter().map(|d| format!("///{d}")));
        let mut text = match &item.kind {
            ItemKind::Binding(b) => binding(b),
            ItemKind::Declaration {
                name: n,
                type_params,
                ty,
            } => form(":", [head(n, type_params), typ(ty)]),
            ItemKind::TypeDef(t) => {
                let h = if !t.head.is_empty() {
                    form(&t.name.name, t.head.iter().map(typ))
                } else {
                    head(&t.name, &t.params)
                };
                match &t.body {
                    TypeBody::Alias(t) => form("=", [h, typ(t)]),
                    TypeBody::Custom(cs) => {
                        form(&h, [braced("{", "}", cs.iter().map(constructor))])
                    }
                }
            }
            ItemKind::MacroItem { module, name, args } => {
                form(&qualified(module, name), args.iter().map(arg))
            }
        };
        for a in item.attributes.iter().rev() {
            let n = qualified(&a.module, &a.name);
            let a = if let Some((args, _)) = &a.call {
                form(&n, args.iter().map(arg))
            } else if a.args.is_empty() {
                n
            } else {
                form(&n, a.args.iter().map(|(s, _)| string_literal(s)))
            };
            text = form("@", [a, text]);
        }
        lines.push(text);
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn head(n: &Ident, params: &[Ident]) -> String {
    if params.is_empty() {
        name(n)
    } else {
        form(&n.name, params.iter().map(name))
    }
}
fn constructor(c: &Constructor) -> String {
    if c.fields.is_empty() {
        name(&c.name)
    } else {
        form(
            &c.name.name,
            c.fields.iter().map(|f| match &f.label {
                Some(n) => form(":", [name(n), typ(&f.ty)]),
                None => typ(&f.ty),
            }),
        )
    }
}
fn binding(b: &Binding) -> String {
    let p = if b.type_params.is_empty() {
        pattern(&b.pattern)
    } else {
        match &b.pattern.kind {
            PatternKind::Var(n) => form(n, b.type_params.iter().map(name)),
            PatternKind::Typed { binder, ty } => form(
                ":",
                [form(&binder.name, b.type_params.iter().map(name)), typ(ty)],
            ),
            _ => panic!("type parameters on a non-name binding"),
        }
    };
    form("=", [p, expr(&b.value)])
}
fn result(r: &RetType) -> String {
    let value = typ(&r.value);
    let value = r
        .err
        .as_ref()
        .map_or(value.clone(), |e| form("^", [value, typ(e)]));
    r.env
        .as_ref()
        .map_or(value.clone(), |e| form("!", [value, typ(e)]))
}
fn typ(t: &TypeExpr) -> String {
    match &t.kind {
        TypeKind::Lit(v) => match v {
            LitValue::Int(n) if n.sign() == num_bigint::Sign::Minus => {
                form("-", [(-n).to_string()])
            }
            _ => v.text(),
        },
        TypeKind::Named { module, name, args } => {
            let n = qualified(module, name);
            if args.is_empty() {
                n
            } else {
                form(&n, args.iter().map(typ))
            }
        }
        TypeKind::Call { module, name, args } => {
            form(&qualified(module, name), args.iter().map(typ))
        }
        TypeKind::ValueOf { module, name } => qualified(module, name),
        TypeKind::Hole => "_".into(),
        TypeKind::Tuple(ts) => form(",", ts.iter().map(typ)),
        TypeKind::Fn {
            params,
            ret,
            err,
            env,
        } => {
            let r = result(&RetType {
                value: (**ret).clone(),
                err: err.as_deref().cloned(),
                env: env.as_deref().cloned(),
            });
            form(
                "->",
                [
                    braced(
                        "(",
                        ")",
                        params.iter().map(|p| {
                            p.label
                                .as_ref()
                                .map_or_else(|| typ(&p.ty), |n| form(":", [name(n), typ(&p.ty)]))
                        }),
                    ),
                    r,
                ],
            )
        }
        TypeKind::Union(ts) => form("|", ts.iter().map(typ)),
        TypeKind::Inter(ts) => form("&", ts.iter().map(typ)),
        TypeKind::Diff(ts) => form("-", ts.iter().map(typ)),
    }
}
fn arg(a: &Arg) -> String {
    a.label
        .as_ref()
        .map_or_else(|| expr(&a.value), |n| form(":", [name(n), expr(&a.value)]))
}
fn segment<T>(s: &Segment<T>, f: impl Fn(&T) -> String) -> String {
    let v = f(&s.value);
    match &s.format {
        None => v,
        Some(fmt) => {
            let fmt = fmt
                .arg
                .as_ref()
                .map_or_else(|| name(&fmt.name), |arg| form(&fmt.name.name, [expr(arg)]));
            form(":", [v, fmt])
        }
    }
}
fn expr(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Int(n) => n.to_string(),
        ExprKind::Float(n) => float(*n),
        ExprKind::String(s) => string_literal(s),
        ExprKind::Name { module, name } => qualified(module, name),
        ExprKind::Tuple(es) => form(",", es.iter().map(expr)),
        ExprKind::List { items, rest } => braced(
            "[",
            "]",
            items
                .iter()
                .map(expr)
                .chain(rest.iter().flat_map(|r| ["..".into(), expr(r)])),
        ),
        ExprKind::BitArray(xs) => braced("<< ", " >>", xs.iter().map(|s| segment(s, expr))),
        ExprKind::Block(b) => match b.stmts.as_slice() {
            [Stmt::Expr(e)] => expr(e),
            _ => block(b),
        },
        ExprKind::Fn(f) => form(
            "->",
            [
                braced(
                    "(",
                    ")",
                    f.params.iter().map(|p| match p {
                        Param::Pattern(p) => pattern(p),
                        Param::Env { name: n, ty } => form("!", [name(n), typ(ty)]),
                    }),
                ),
                f.ret.as_ref().map_or_else(|| "_".into(), result),
                block(&f.body),
            ],
        ),
        ExprKind::Case { subject, clauses } => form(".", [expr(subject), clause_block(clauses)]),
        ExprKind::Clauses(cs) => clause_block(cs),
        ExprKind::Call { callee, args } => form(&expr(callee), args.iter().map(arg)),
        ExprKind::Field { target, name: n } => form(".", [expr(target), name(n)]),
        ExprKind::TupleIndex { target, index, .. } => form(".", [expr(target), index.to_string()]),
        ExprKind::RecordUpdate {
            constructor,
            base,
            fields,
        } => form(
            &expr(constructor),
            ["..".into(), expr(base)]
                .into_iter()
                .chain(fields.iter().map(|(n, e)| form(":", [name(n), expr(e)]))),
        ),
        ExprKind::Unary { op, expr: e } => form(
            match op {
                UnOp::Neg => "-",
                UnOp::NegFloat => "-.",
            },
            [expr(e)],
        ),
        ExprKind::Binary { op, lhs, rhs } => form(
            if *op == BinOp::Pipe { "|" } else { op.text() },
            [expr(lhs), expr(rhs)],
        ),
    }
}
fn block(b: &Block) -> String {
    braced(
        "{",
        "}",
        b.stmts.iter().map(|s| match s {
            Stmt::Bind(b) => binding(b),
            Stmt::Env { ty, value, .. } => form("!", [typ(ty), expr(value)]),
            Stmt::Expr(e) => expr(e),
        }),
    )
}
fn clause_block(cs: &[Clause]) -> String {
    braced(
        "{",
        "}",
        cs.iter().map(|c| {
            let p = if c.patterns.len() == 1 {
                pattern(&c.patterns[0])
            } else {
                form("|", c.patterns.iter().map(pattern))
            };
            let p = c
                .guard
                .as_ref()
                .map_or(p.clone(), |g| form("?", [p, expr(g)]));
            form("->", [p, expr(&c.body)])
        }),
    )
}
fn pattern(p: &Pattern) -> String {
    match &p.kind {
        PatternKind::Var(s) | PatternKind::Discard(s) => s.clone(),
        PatternKind::Typed { binder, ty } => form(":", [binder.name.clone(), typ(ty)]),
        PatternKind::Named {
            name: n,
            pattern: p,
        } => form("=", [name(n), pattern(p)]),
        PatternKind::Int { negative, value } => {
            if *negative {
                form("-", [value.to_string()])
            } else {
                value.to_string()
            }
        }
        PatternKind::Float { negative, value } => {
            if *negative {
                form("-.", [float(*value)])
            } else {
                float(*value)
            }
        }
        PatternKind::String(s) => string_literal(s),
        PatternKind::Prefix { prefix, rest } => {
            form("<>", [string_literal(prefix), rest.name.clone()])
        }
        PatternKind::Constructor {
            module,
            name: n,
            args,
            spread,
        } => {
            let n = qualified(module, n);
            if args.is_empty() && !spread {
                n
            } else {
                form(
                    &n,
                    args.iter()
                        .map(pattern)
                        .chain(spread.then_some("..".into())),
                )
            }
        }
        PatternKind::Tuple(ps) => form(",", ps.iter().map(pattern)),
        PatternKind::List { items, rest } => braced(
            "[",
            "]",
            items.iter().map(pattern).chain(rest.iter().flat_map(|r| {
                std::iter::once("..".into()).chain(r.iter().map(|b| b.name.clone()))
            })),
        ),
        PatternKind::BitArray(xs) => braced("<< ", " >>", xs.iter().map(|s| segment(s, pattern))),
    }
}
