//! Encode the parsed core tree as hygienic syntax, without reparsing text.
use super::syntax::{Node, Scope, Syntax};
use crate::ast::*;
use crate::span::Span;
use crate::token::{Token, TokenKind};

pub fn expression_syntax(e: &Expr, scope: &Scope, source: &str) -> Syntax {
    Encoder::new(scope, source).expr(e)
}
pub fn item_syntax(item: &Item, scope: &Scope, source: &str) -> Syntax {
    Encoder::new(scope, source).item(item)
}
pub fn pattern_syntax(p: &Pattern, scope: &Scope, source: &str) -> Syntax {
    Encoder::new(scope, source).pattern(p)
}
pub fn type_syntax(t: &TypeExpr, scope: &Scope, source: &str) -> Syntax {
    Encoder::new(scope, source).ty(t)
}

struct Encoder<'a> {
    scope: &'a Scope,
    tokens: Vec<Token>,
}
impl Encoder<'_> {
    fn new<'a>(scope: &'a Scope, source: &str) -> Encoder<'a> {
        Encoder {
            scope,
            tokens: crate::lex(source).tokens,
        }
    }
    fn node(&self, n: Node, span: Span) -> Syntax {
        Syntax::written(n, self.scope, span)
    }
    fn symbol(&self, s: &str, span: Span) -> Syntax {
        self.node(Node::Symbol(s.into()), span)
    }
    fn token_span(&self, range: Span, matches: impl Fn(&TokenKind) -> bool) -> Span {
        self.tokens
            .iter()
            .find(|t| range.start <= t.span.start && t.span.end <= range.end && matches(&t.kind))
            .unwrap_or_else(|| panic!("parsed syntax has no expected token in {range:?}"))
            .span
    }
    fn operator(&self, spelling: &str, range: Span) -> Syntax {
        let span = self.token_span(range, |kind| {
            kind.text() == spelling
                || (spelling == "|" && *kind == TokenKind::PipeGt)
                || (spelling == "^" && *kind == TokenKind::Question)
        });
        self.symbol(spelling, span)
    }
    fn name(&self, n: &Ident) -> Syntax {
        self.node(
            if n.name.starts_with(|c: char| c.is_ascii_uppercase()) {
                Node::Up(n.name.clone())
            } else {
                Node::Name(n.name.clone())
            },
            n.span,
        )
    }
    fn binder(&self, b: &Binder) -> Syntax {
        self.node(
            if b.discard {
                Node::Discard(b.name.clone())
            } else {
                Node::Name(b.name.clone())
            },
            b.span,
        )
    }
    fn form(&self, head: &str, xs: impl IntoIterator<Item = Syntax>, span: Span) -> Syntax {
        self.node(
            Node::Round(std::iter::once(self.symbol(head, span)).chain(xs).collect()),
            span,
        )
    }
    fn form_token(
        &self,
        head: &str,
        range: Span,
        xs: impl IntoIterator<Item = Syntax>,
        span: Span,
    ) -> Syntax {
        self.call(self.operator(head, range), xs, span)
    }
    fn call(&self, head: Syntax, xs: impl IntoIterator<Item = Syntax>, span: Span) -> Syntax {
        self.node(Node::Round(std::iter::once(head).chain(xs).collect()), span)
    }
    fn qualified(&self, module: &Option<Ident>, name: &Ident) -> Syntax {
        match module {
            None => self.name(name),
            Some(m) => self.form_token(
                ".",
                Span::new(m.span.end, name.span.start),
                [self.name(m), self.name(name)],
                m.span.to(name.span),
            ),
        }
    }
    fn arg(&self, a: &Arg) -> Syntax {
        let value = self.expr(&a.value);
        match &a.label {
            Some(n) => self.form_token(
                ":",
                Span::new(n.span.end, a.value.span.start),
                [self.name(n), value],
                n.span.to(a.value.span),
            ),
            None => value,
        }
    }
    fn head(&self, name: &Ident, params: &[Ident], span: Span) -> Syntax {
        if params.is_empty() {
            self.name(name)
        } else {
            self.call(self.name(name), params.iter().map(|p| self.name(p)), span)
        }
    }
    fn constructor(&self, c: &Constructor) -> Syntax {
        let head = self.name(&c.name);
        if c.fields.is_empty() {
            return head;
        }
        self.call(
            head,
            c.fields.iter().map(|f| {
                let ty = self.ty(&f.ty);
                match &f.label {
                    Some(n) => self.form_token(
                        ":",
                        Span::new(n.span.end, f.ty.span.start),
                        [self.name(n), ty],
                        f.span,
                    ),
                    None => ty,
                }
            }),
            c.span,
        )
    }
    fn item(&self, item: &Item) -> Syntax {
        let mut result = match &item.kind {
            ItemKind::Binding(b) => self.binding(b),
            ItemKind::Declaration {
                name,
                type_params,
                ty,
            } => {
                let head = self.head(name, type_params, name.span.to(ty.span));
                let end = type_params.last().map_or(name.span.end, |p| p.span.end);
                self.form_token(
                    ":",
                    Span::new(end, ty.span.start),
                    [head, self.ty(ty)],
                    item.span,
                )
            }
            ItemKind::TypeDef(t) => {
                let head = if !t.head.is_empty() {
                    self.call(
                        self.name(&t.name),
                        t.head.iter().map(|x| self.ty(x)),
                        item.span,
                    )
                } else {
                    self.head(&t.name, &t.params, item.span)
                };
                match &t.body {
                    TypeBody::Alias(ty) => {
                        let end = t.head.last().map_or_else(
                            || t.params.last().map_or(t.name.span.end, |p| p.span.end),
                            |p| p.span.end,
                        );
                        self.form_token(
                            "=",
                            Span::new(end, ty.span.start),
                            [head, self.ty(ty)],
                            item.span,
                        )
                    }
                    TypeBody::Custom(cs) => self.call(
                        head,
                        [self.node(
                            Node::Curly(cs.iter().map(|c| self.constructor(c)).collect()),
                            item.span,
                        )],
                        item.span,
                    ),
                }
            }
            ItemKind::MacroItem { module, name, args } => self.call(
                self.qualified(module, name),
                args.iter().map(|a| self.arg(a)),
                item.span,
            ),
        };
        for attr in item.attributes.iter().rev() {
            let mut head = self.qualified(&attr.module, &attr.name);
            if let Some((args, span)) = &attr.call {
                head = self.call(head, args.iter().map(|a| self.arg(a)), *span);
            } else if !attr.args.is_empty() {
                head = self.call(
                    head,
                    attr.args
                        .iter()
                        .map(|(s, span)| self.node(Node::Text(s.clone()), *span)),
                    attr.span,
                );
            }
            let at = self.operator("@", Span::new(attr.span.start, attr.name.span.start));
            result = self.call(at, [head, result], item.span);
        }
        result
    }
    fn ty(&self, t: &TypeExpr) -> Syntax {
        match &t.kind {
            TypeKind::Lit(LitValue::Str(s)) => self.node(Node::Text(s.clone()), t.span),
            TypeKind::Lit(LitValue::Int(n)) => {
                let (sign, value) = n.to_bytes_be();
                let number_span = self.token_span(t.span, |k| matches!(k, TokenKind::Int(_)));
                let value = self.node(
                    Node::Int(num_bigint::BigUint::from_bytes_be(&value)),
                    number_span,
                );
                if sign == num_bigint::Sign::Minus {
                    self.call(
                        self.operator("-", Span::new(t.span.start, number_span.start)),
                        [value],
                        t.span,
                    )
                } else {
                    value
                }
            }
            TypeKind::Named { module, name, args } => {
                let head = self.qualified(module, name);
                if args.is_empty() {
                    head
                } else {
                    self.call(head, args.iter().map(|x| self.ty(x)), t.span)
                }
            }
            TypeKind::ValueOf { module, name } => self.qualified(module, name),
            TypeKind::Call { module, name, args } => self.call(
                self.qualified(module, name),
                args.iter().map(|x| self.ty(x)),
                t.span,
            ),
            TypeKind::Hole => self.node(Node::Discard("_".into()), t.span),
            TypeKind::Tuple(ts) => self.form(",", ts.iter().map(|x| self.ty(x)), t.span),
            TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => {
                let ps = self.node(
                    Node::Round(
                        params
                            .iter()
                            .map(|p| {
                                let t = self.ty(&p.ty);
                                match &p.label {
                                    Some(n) => {
                                        self.form(":", [self.name(n), t], n.span.to(p.ty.span))
                                    }
                                    None => t,
                                }
                            })
                            .collect(),
                    ),
                    t.span,
                );
                let result = self.result(ret, err.as_deref(), env.as_deref(), t.span);
                self.form("->", [ps, result], t.span)
            }
            TypeKind::Union(ts) => self.form("|", ts.iter().map(|x| self.ty(x)), t.span),
            TypeKind::Inter(ts) => self.form("&", ts.iter().map(|x| self.ty(x)), t.span),
            TypeKind::Diff(ts) => self.form("-", ts.iter().map(|x| self.ty(x)), t.span),
        }
    }
    fn result(
        &self,
        value: &TypeExpr,
        err: Option<&TypeExpr>,
        env: Option<&TypeExpr>,
        span: Span,
    ) -> Syntax {
        let mut value_syntax = self.ty(value);
        if let Some(e) = err {
            value_syntax = self.form_token(
                "^",
                Span::new(value.span.end, e.span.start),
                [value_syntax, self.ty(e)],
                span,
            );
        } else if matches!(value.kind, TypeKind::Hole) {
            value_syntax = self.form(
                "^",
                [value_syntax, self.node(Node::Up("Never".into()), span)],
                span,
            );
        }
        if let Some(e) = env {
            let previous_end = err.map_or(value.span.end, |ty| ty.span.end);
            value_syntax = self.form_token(
                "!",
                Span::new(previous_end, e.span.start),
                [value_syntax, self.ty(e)],
                span,
            );
        }
        value_syntax
    }
    fn segment<T>(&self, s: &Segment<T>, f: impl Fn(&Self, &T) -> Syntax) -> Syntax {
        let value = f(self, &s.value);
        match &s.format {
            None => value,
            Some(fmt) => {
                let mut name = self.name(&fmt.name);
                if let Some(arg) = &fmt.arg {
                    name = self.call(name, [self.expr(arg)], fmt.name.span.to(arg.span));
                }
                self.form_token(
                    ":",
                    Span::new(value.span().end, fmt.name.span.start),
                    [value, name],
                    s.span,
                )
            }
        }
    }
    fn binding(&self, b: &Binding) -> Syntax {
        let head = if b.type_params.is_empty() {
            self.pattern(&b.pattern)
        } else {
            let (name, ty) = match &b.pattern.kind {
                PatternKind::Var(s) => (self.node(Node::Name(s.clone()), b.pattern.span), None),
                PatternKind::Typed { binder, ty } => (self.binder(binder), Some(ty)),
                _ => panic!("type parameters on a non-name binding"),
            };
            let h = self.call(
                name,
                b.type_params.iter().map(|p| self.name(p)),
                b.pattern.span,
            );
            if let Some(t) = ty {
                self.form(":", [h, self.ty(t)], b.pattern.span)
            } else {
                h
            }
        };
        self.form_token(
            "=",
            Span::new(b.pattern.span.end, b.value.span.start),
            [head, self.expr(&b.value)],
            b.span,
        )
    }
    fn block(&self, b: &Block) -> Syntax {
        let parts = b
            .stmts
            .iter()
            .map(|s| match s {
                Stmt::Bind(b) => self.binding(b),
                Stmt::Env { ty, value, span } => {
                    self.form("!", [self.ty(ty), self.expr(value)], *span)
                }
                Stmt::Expr(e) => self.expr(e),
            })
            .collect();
        self.node(Node::Curly(parts), b.span)
    }
    fn clauses(&self, cs: &[Clause], span: Span) -> Syntax {
        let parts = cs
            .iter()
            .map(|c| {
                let p = if c.patterns.len() == 1 {
                    self.pattern(&c.patterns[0])
                } else {
                    self.form("|", c.patterns.iter().map(|p| self.pattern(p)), c.span)
                };
                let p = if let Some(g) = &c.guard {
                    self.form("?", [p, self.expr(g)], c.span)
                } else {
                    p
                };
                self.form("->", [p, self.expr(&c.body)], c.span)
            })
            .collect();
        self.node(Node::Curly(parts), span)
    }
    fn expr(&self, e: &Expr) -> Syntax {
        match &e.kind {
            ExprKind::Quote(d) => Syntax::from_datum(d, self.scope),
            ExprKind::Int(n) => self.node(Node::Int(n.clone()), e.span),
            ExprKind::Float(n) => self.node(Node::Float(*n), e.span),
            ExprKind::String(s) => self.node(Node::Text(s.clone()), e.span),
            ExprKind::Name { module, name } => self.qualified(module, name),
            ExprKind::Tuple(xs) if xs.is_empty() => self.node(Node::Round(Vec::new()), e.span),
            ExprKind::Tuple(xs) => self.form(",", xs.iter().map(|x| self.expr(x)), e.span),
            ExprKind::List { items, rest } => {
                let mut xs: Vec<_> = items.iter().map(|x| self.expr(x)).collect();
                if let Some(r) = rest {
                    xs.push(self.symbol("..", r.span));
                    xs.push(self.expr(r));
                }
                self.node(Node::Square(xs), e.span)
            }
            ExprKind::BitArray(xs) => self.node(
                Node::Bits(xs.iter().map(|s| self.segment(s, Self::expr)).collect()),
                e.span,
            ),
            ExprKind::Block(b) => match b.stmts.as_slice() {
                [Stmt::Expr(inner)] => self.expr(inner),
                _ => self.block(b),
            },
            ExprKind::Fn(f) => {
                let ps = self.node(
                    Node::Round(
                        f.params
                            .iter()
                            .map(|p| match p {
                                Param::Pattern(p) => self.pattern(p),
                                Param::Env { name, ty } => self.form(
                                    "!",
                                    [self.name(name), self.ty(ty)],
                                    name.span.to(ty.span),
                                ),
                            })
                            .collect(),
                    ),
                    e.span,
                );
                let result = f.ret.as_ref().map_or_else(
                    || self.node(Node::Discard("_".into()), e.span),
                    |r| self.result(&r.value, r.err.as_ref(), r.env.as_ref(), e.span),
                );
                self.form("->", [ps, result, self.block(&f.body)], e.span)
            }
            ExprKind::Case { subject, clauses } => self.form(
                ".",
                [self.expr(subject), self.clauses(clauses, e.span)],
                e.span,
            ),
            ExprKind::Clauses(cs) => self.clauses(cs, e.span),
            ExprKind::Call { callee, args } => {
                self.call(self.expr(callee), args.iter().map(|x| self.arg(x)), e.span)
            }
            ExprKind::Field { target, name } => self.form_token(
                ".",
                Span::new(target.span.end, name.span.start),
                [self.expr(target), self.name(name)],
                e.span,
            ),
            ExprKind::TupleIndex {
                target,
                index,
                index_span,
            } => self.form_token(
                ".",
                Span::new(target.span.end, index_span.start),
                [
                    self.expr(target),
                    self.node(Node::Int(index.clone()), *index_span),
                ],
                e.span,
            ),
            ExprKind::RecordUpdate {
                constructor,
                base,
                fields,
            } => self.call(
                self.expr(constructor),
                [self.symbol("..", base.span), self.expr(base)]
                    .into_iter()
                    .chain(fields.iter().map(|(n, v)| {
                        self.form(":", [self.name(n), self.expr(v)], n.span.to(v.span))
                    })),
                e.span,
            ),
            ExprKind::Unary { op, expr } => {
                let spelling = match op {
                    UnOp::Neg => "-",
                    UnOp::NegFloat => "-.",
                };
                self.call(
                    self.operator(spelling, Span::new(e.span.start, expr.span.start)),
                    [self.expr(expr)],
                    e.span,
                )
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let spelling = if *op == BinOp::Pipe { "|" } else { op.text() };
                self.call(
                    self.operator(spelling, Span::new(lhs.span.end, rhs.span.start)),
                    [self.expr(lhs), self.expr(rhs)],
                    e.span,
                )
            }
        }
    }
    fn pattern(&self, p: &Pattern) -> Syntax {
        match &p.kind {
            PatternKind::Quote(d) => Syntax::from_datum(d, self.scope),
            PatternKind::Var(n) => self.node(Node::Name(n.clone()), p.span),
            PatternKind::Discard(n) => self.node(Node::Discard(n.clone()), p.span),
            PatternKind::Typed { binder, ty } => self.form_token(
                ":",
                Span::new(binder.span.end, ty.span.start),
                [self.binder(binder), self.ty(ty)],
                p.span,
            ),
            PatternKind::Named { name, pattern } => self.form_token(
                "=",
                Span::new(name.span.end, pattern.span.start),
                [self.name(name), self.pattern(pattern)],
                p.span,
            ),
            PatternKind::Int { negative, value } => {
                let span = self.token_span(p.span, |k| matches!(k, TokenKind::Int(_)));
                let n = self.node(Node::Int(value.clone()), span);
                if *negative {
                    self.call(
                        self.operator("-", Span::new(p.span.start, span.start)),
                        [n],
                        p.span,
                    )
                } else {
                    n
                }
            }
            PatternKind::Float { negative, value } => {
                let span = self.token_span(p.span, |k| matches!(k, TokenKind::Float(_)));
                let n = self.node(Node::Float(*value), span);
                if *negative {
                    self.call(
                        self.operator("-.", Span::new(p.span.start, span.start)),
                        [n],
                        p.span,
                    )
                } else {
                    n
                }
            }
            PatternKind::String(s) => self.node(Node::Text(s.clone()), p.span),
            PatternKind::Prefix { prefix, rest } => {
                let span = self.token_span(p.span, |k| matches!(k, TokenKind::Str(_)));
                self.form(
                    "<>",
                    [
                        self.node(Node::Text(prefix.clone()), span),
                        self.binder(rest),
                    ],
                    p.span,
                )
            }
            PatternKind::Constructor {
                module,
                name,
                args,
                spread,
            } => {
                let head = self.qualified(module, name);
                if args.is_empty() && !spread {
                    head
                } else {
                    self.call(
                        head,
                        args.iter()
                            .map(|p| self.pattern(p))
                            .chain(spread.then(|| self.symbol("..", p.span))),
                        p.span,
                    )
                }
            }
            PatternKind::Tuple(xs) if xs.is_empty() => self.node(Node::Round(Vec::new()), p.span),
            PatternKind::Tuple(xs) => self.form(",", xs.iter().map(|p| self.pattern(p)), p.span),
            PatternKind::List { items, rest } => {
                let mut xs: Vec<_> = items.iter().map(|p| self.pattern(p)).collect();
                if let Some(rest) = rest {
                    xs.push(self.symbol("..", p.span));
                    if let Some(b) = rest {
                        xs.push(self.binder(b));
                    }
                }
                self.node(Node::Square(xs), p.span)
            }
            PatternKind::BitArray(xs) => self.node(
                Node::Bits(xs.iter().map(|s| self.segment(s, Self::pattern)).collect()),
                p.span,
            ),
        }
    }
}
