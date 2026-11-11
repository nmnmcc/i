//! Encode the parsed core tree as hygienic syntax, without reparsing text.
use super::syntax::{Node, Scope, Syntax};
use crate::ast::*;
use crate::span::Span;
use crate::token::{Token, TokenKind};
use std::cell::RefCell;
use std::collections::HashMap;

pub fn expression_syntax(e: &Expr, scope: &Scope, source: &str) -> Syntax {
    SourceEncoder::new(scope, source).expression(e)
}
pub fn item_syntax(item: &Item, scope: &Scope, source: &str) -> Syntax {
    SourceEncoder::new(scope, source).item(item)
}
pub fn pattern_syntax(p: &Pattern, scope: &Scope, source: &str) -> Syntax {
    SourceEncoder::new(scope, source).pattern(p)
}
pub fn type_syntax(t: &TypeExpr, scope: &Scope, source: &str) -> Syntax {
    SourceEncoder::new(scope, source).ty(t)
}

/// Encodes nodes from one source using a reusable lexical index.
///
/// The input nodes must belong to the source passed to `new`. Results carry
/// the scope and the original spans of those nodes.
pub struct SourceEncoder {
    scope: Scope,
    tokens: Vec<Token>,
    close: Vec<Option<u32>>,
    positions: Vec<usize>,
}
impl SourceEncoder {
    pub fn new(scope: &Scope, source: &str) -> Self {
        let lexed = crate::lex(source);
        // The first token beginning at or after each byte boundary. The
        // source is indexed once, so separate items do not re-lex it.
        let mut positions = vec![lexed.tokens.len(); source.len() + 1];
        let mut next = lexed.tokens.len();
        for offset in (0..=source.len()).rev() {
            while next > 0 && lexed.tokens[next - 1].span.start as usize >= offset {
                next -= 1;
            }
            positions[offset] = next;
        }
        Self {
            scope: scope.clone(),
            tokens: lexed.tokens,
            close: lexed.close,
            positions,
        }
    }
    pub fn expression(&self, expression: &Expr) -> Syntax {
        let encoder = Encoder::new(self);
        encoder.prepare(vec![Work::Expr(expression, false)]);
        encoder.expr(expression)
    }
    pub fn item(&self, item: &Item) -> Syntax {
        let encoder = Encoder::new(self);
        encoder.prepare(vec![Work::Item(item)]);
        encoder.item(item)
    }
    /// Visit exactly the charges of copying this written item's encoded
    /// Syntax, in the same order as the host's output-tree traversal. No
    /// Syntax or atom copies are constructed; an error stops immediately.
    pub fn visit_item_charge<E>(
        &self,
        item: &Item,
        visit: impl FnMut(super::SyntaxCharge) -> Result<(), E>,
    ) -> Result<(), E> {
        super::encode_charge::item(item, visit)
    }
    pub fn pattern(&self, pattern: &Pattern) -> Syntax {
        let encoder = Encoder::new(self);
        encoder.prepare(vec![Work::Pattern(pattern, false)]);
        encoder.pattern(pattern)
    }
    pub fn ty(&self, ty: &TypeExpr) -> Syntax {
        let encoder = Encoder::new(self);
        encoder.prepare(vec![Work::Type(ty, false)]);
        encoder.ty(ty)
    }
}

struct Encoder<'a> {
    scope: &'a Scope,
    tokens: &'a [Token],
    close: &'a [Option<u32>],
    positions: &'a [usize],
    expressions: RefCell<HashMap<*const Expr, Syntax>>,
    patterns: RefCell<HashMap<*const Pattern, Syntax>>,
    types: RefCell<HashMap<*const TypeExpr, Syntax>>,
}

// Visit nodes in postorder, so encoding a parent only reads already encoded
// children. Auxiliary structures are work items as well: nested functions,
// blocks, segments and type annotations must not reintroduce recursion.
enum Work<'a> {
    Expr(&'a Expr, bool),
    Pattern(&'a Pattern, bool),
    Type(&'a TypeExpr, bool),
    Item(&'a Item),
    Binding(&'a Binding),
    Block(&'a Block),
    Clause(&'a Clause),
    Arg(&'a Arg),
}
impl Encoder<'_> {
    fn new(source: &SourceEncoder) -> Encoder<'_> {
        Encoder {
            scope: &source.scope,
            tokens: &source.tokens,
            close: &source.close,
            positions: &source.positions,
            expressions: RefCell::new(HashMap::new()),
            patterns: RefCell::new(HashMap::new()),
            types: RefCell::new(HashMap::new()),
        }
    }
    fn prepare<'a>(&self, mut work: Vec<Work<'a>>) {
        while let Some(task) = work.pop() {
            match task {
                Work::Expr(e, true) => {
                    let value = self.encode_expr(e);
                    self.expressions
                        .borrow_mut()
                        .insert(e as *const Expr, value);
                }
                Work::Pattern(p, true) => {
                    let value = self.encode_pattern(p);
                    self.patterns
                        .borrow_mut()
                        .insert(p as *const Pattern, value);
                }
                Work::Type(t, true) => {
                    let value = self.encode_ty(t);
                    self.types.borrow_mut().insert(t as *const TypeExpr, value);
                }
                Work::Expr(e, false) => {
                    work.push(Work::Expr(e, true));
                    match &e.kind {
                        ExprKind::Quote(_)
                        | ExprKind::Int(_)
                        | ExprKind::Float(_)
                        | ExprKind::String(_)
                        | ExprKind::Name { .. } => {}
                        ExprKind::Tuple(xs) => work.extend(xs.iter().map(|x| Work::Expr(x, false))),
                        ExprKind::Record { base, fields } => {
                            if let Some(base) = base {
                                work.push(Work::Expr(base, false));
                            }
                            work.extend(fields.iter().map(|(_, x)| Work::Expr(x, false)));
                        }
                        ExprKind::List { items, rest } => {
                            work.extend(items.iter().map(|x| Work::Expr(x, false)));
                            if let Some(x) = rest {
                                work.push(Work::Expr(x, false));
                            }
                        }
                        ExprKind::BitArray(xs) => {
                            for x in xs {
                                work.push(Work::Expr(&x.value, false));
                                if let Some(arg) = x.format.as_ref().and_then(|f| f.arg.as_deref())
                                {
                                    work.push(Work::Expr(arg, false));
                                }
                            }
                        }
                        ExprKind::Block(b) => work.push(Work::Block(b)),
                        ExprKind::Fn(f) => {
                            work.push(Work::Block(&f.body));
                            for p in &f.params {
                                match p {
                                    Param::Pattern(p) => work.push(Work::Pattern(p, false)),
                                    Param::Env { ty, .. } => work.push(Work::Type(ty, false)),
                                }
                            }
                            if let Some(r) = &f.ret {
                                work.push(Work::Type(&r.value, false));
                                if let Some(t) = &r.err {
                                    work.push(Work::Type(t, false));
                                }
                                if let Some(t) = &r.env {
                                    work.push(Work::Type(t, false));
                                }
                            }
                        }
                        ExprKind::Case { subject, clauses } => {
                            work.push(Work::Expr(subject, false));
                            work.extend(clauses.iter().map(Work::Clause));
                        }
                        ExprKind::Clauses(cs) => work.extend(cs.iter().map(Work::Clause)),
                        ExprKind::Call { callee, args } => {
                            work.push(Work::Expr(callee, false));
                            work.extend(args.iter().map(Work::Arg));
                        }
                        ExprKind::Field { target, .. }
                        | ExprKind::TupleIndex { target, .. }
                        | ExprKind::Unary { expr: target, .. } => {
                            work.push(Work::Expr(target, false))
                        }
                        ExprKind::RecordUpdate {
                            constructor,
                            base,
                            fields,
                        } => {
                            work.push(Work::Expr(constructor, false));
                            work.push(Work::Expr(base, false));
                            work.extend(fields.iter().map(|(_, e)| Work::Expr(e, false)));
                        }
                        ExprKind::Binary { lhs, rhs, .. } => {
                            work.push(Work::Expr(lhs, false));
                            work.push(Work::Expr(rhs, false));
                        }
                    }
                }
                Work::Pattern(p, false) => {
                    work.push(Work::Pattern(p, true));
                    match &p.kind {
                        PatternKind::Quote(_)
                        | PatternKind::Var(_)
                        | PatternKind::Discard(_)
                        | PatternKind::Int { .. }
                        | PatternKind::Float { .. }
                        | PatternKind::String(_)
                        | PatternKind::Prefix { .. } => {}
                        PatternKind::Typed { ty, .. } => work.push(Work::Type(ty, false)),
                        PatternKind::Named { pattern, .. } => {
                            work.push(Work::Pattern(pattern, false))
                        }
                        PatternKind::Constructor { args, .. } | PatternKind::Tuple(args) => {
                            work.extend(args.iter().map(|p| Work::Pattern(p, false)));
                        }
                        PatternKind::List { items, .. } => {
                            work.extend(items.iter().map(|p| Work::Pattern(p, false)));
                        }
                        PatternKind::BitArray(xs) => {
                            for x in xs {
                                work.push(Work::Pattern(&x.value, false));
                                if let Some(arg) = x.format.as_ref().and_then(|f| f.arg.as_deref())
                                {
                                    work.push(Work::Expr(arg, false));
                                }
                            }
                        }
                    }
                }
                Work::Type(t, false) => {
                    work.push(Work::Type(t, true));
                    match &t.kind {
                        TypeKind::Lit(_) | TypeKind::ValueOf { .. } | TypeKind::Hole => {}
                        TypeKind::Named { args, .. }
                        | TypeKind::Call { args, .. }
                        | TypeKind::Tuple(args)
                        | TypeKind::Union(args)
                        | TypeKind::Inter(args)
                        | TypeKind::Diff(args) => {
                            work.extend(args.iter().map(|t| Work::Type(t, false)));
                        }
                        TypeKind::Record { base, fields, .. } => {
                            if let Some(base) = base {
                                work.push(Work::Type(base, false));
                            }
                            work.extend(fields.iter().map(|(_, x)| Work::Type(x, false)));
                        }
                        TypeKind::Fn {
                            params,
                            ret,
                            err,
                            env,
                        } => {
                            work.extend(params.iter().map(|p| Work::Type(&p.ty, false)));
                            work.push(Work::Type(ret, false));
                            if let Some(t) = err {
                                work.push(Work::Type(t, false));
                            }
                            if let Some(t) = env {
                                work.push(Work::Type(t, false));
                            }
                        }
                    }
                }
                Work::Item(item) => {
                    for attr in &item.attributes {
                        if let Some((args, _)) = &attr.call {
                            work.extend(args.iter().map(Work::Arg));
                        }
                    }
                    match &item.kind {
                        ItemKind::Binding(b) => work.push(Work::Binding(b)),
                        ItemKind::Declaration { ty, .. } => work.push(Work::Type(ty, false)),
                        ItemKind::TypeDef(t) => {
                            work.extend(t.head.iter().map(|t| Work::Type(t, false)));
                            match &t.body {
                                TypeBody::Alias(t) => work.push(Work::Type(t, false)),
                                TypeBody::Custom(cs) => {
                                    for c in cs {
                                        work.extend(
                                            c.fields.iter().map(|f| Work::Type(&f.ty, false)),
                                        );
                                    }
                                }
                            }
                        }
                        ItemKind::MacroItem { args, .. } => work.extend(args.iter().map(Work::Arg)),
                    }
                }
                Work::Binding(b) => {
                    work.push(Work::Pattern(&b.pattern, false));
                    work.push(Work::Expr(&b.value, false));
                }
                Work::Block(b) => {
                    for stmt in &b.stmts {
                        match stmt {
                            Stmt::Bind(b) => work.push(Work::Binding(b)),
                            Stmt::Env { ty, value, .. } => {
                                work.push(Work::Type(ty, false));
                                work.push(Work::Expr(value, false));
                            }
                            Stmt::Expr(e) => work.push(Work::Expr(e, false)),
                        }
                    }
                }
                Work::Clause(c) => {
                    work.extend(c.patterns.iter().map(|p| Work::Pattern(p, false)));
                    if let Some(g) = &c.guard {
                        work.push(Work::Expr(g, false));
                    }
                    work.push(Work::Expr(&c.body, false));
                }
                Work::Arg(a) => work.push(Work::Expr(&a.value, false)),
            }
        }
    }
    fn node(&self, n: Node, span: Span) -> Syntax {
        Syntax::written(n, self.scope, span)
    }
    fn symbol(&self, s: &str, span: Span) -> Syntax {
        self.node(Node::Symbol(s.into()), span)
    }
    fn token_span(&self, range: Span, matches: impl Fn(&TokenKind) -> bool) -> Span {
        self.tokens[self.positions[range.start as usize]..]
            .iter()
            .take_while(|t| t.span.start <= range.end)
            .find(|t| t.span.end <= range.end && matches(&t.kind))
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
    fn bracket(&self, range: Span, kind: TokenKind) -> Span {
        let start = self.positions[range.start as usize];
        let (i, token) = self
            .tokens
            .iter()
            .enumerate()
            .skip(start)
            .take_while(|(_, t)| t.span.start <= range.end)
            .find(|(_, t)| t.span.end <= range.end && t.kind == kind)
            .unwrap_or_else(|| panic!("parsed syntax has no expected bracket in {range:?}"));
        let close = self.close[i].expect("parsed syntax has an unmatched bracket") as usize;
        token.span.to(self.tokens[close].span)
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
    fn record_label(&self, n: &Ident) -> Syntax {
        let token = &self.tokens[self.positions[n.span.start as usize]];
        if matches!(token.kind, TokenKind::Str(_)) {
            self.node(Node::Text(n.name.clone()), n.span)
        } else {
            self.name(n)
        }
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
            result = self.call(
                at,
                [head, result],
                Span::new(attr.span.start, item.span.end),
            );
        }
        result
    }
    fn ty(&self, t: &TypeExpr) -> Syntax {
        self.types
            .borrow()
            .get(&(t as *const TypeExpr))
            .expect("type child was not prepared")
            .clone()
    }
    fn encode_ty(&self, t: &TypeExpr) -> Syntax {
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
            TypeKind::Record {
                base,
                fields,
                removed,
            } => {
                let mut nodes = Vec::with_capacity(
                    fields.len() + removed.len() + usize::from(base.is_some()) * 2,
                );
                if let Some(base) = base {
                    nodes.push(self.operator("..", Span::new(t.span.start, base.span.start)));
                    nodes.push(self.ty(base));
                }
                for n in removed {
                    let i = self.positions[n.span.start as usize];
                    let minus = self.tokens[i - 1].span;
                    nodes.push(self.call(
                        self.symbol("-", minus),
                        [self.record_label(&n.ident)],
                        minus.to(n.span),
                    ));
                }
                for (n, value) in fields {
                    nodes.push(self.form_token(
                        ":",
                        Span::new(n.span.end, value.span.start),
                        [self.record_label(&n.ident), self.ty(value)],
                        n.span.to(value.span),
                    ));
                }
                self.node(Node::Curly(nodes), t.span)
            }
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
        self.expressions
            .borrow()
            .get(&(e as *const Expr))
            .expect("expression child was not prepared")
            .clone()
    }
    fn encode_expr(&self, e: &Expr) -> Syntax {
        match &e.kind {
            ExprKind::Quote(d) => Syntax::from_datum(d, self.scope),
            ExprKind::Int(n) => self.node(Node::Int(n.clone()), e.span),
            ExprKind::Float(n) => self.node(Node::Float(*n), e.span),
            ExprKind::String(s) => self.node(Node::Text(s.clone()), e.span),
            ExprKind::Name { module, name } => self.qualified(module, name),
            ExprKind::Record { base, fields } if base.is_none() && fields.is_empty() => {
                self.node(Node::Round(Vec::new()), e.span)
            }
            ExprKind::Record { base, fields } => {
                let mut nodes = Vec::with_capacity(fields.len() + usize::from(base.is_some()) * 2);
                if let Some(base) = base {
                    nodes.push(self.operator("..", Span::new(e.span.start, base.span.start)));
                    nodes.push(self.expr(base));
                }
                for (n, value) in fields {
                    nodes.push(self.form_token(
                        ":",
                        Span::new(n.span.end, value.span.start),
                        [self.record_label(n), self.expr(value)],
                        n.span.to(value.span),
                    ));
                }
                self.node(Node::Curly(nodes), e.span)
            }
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
                    self.bracket(e.span, TokenKind::LParen),
                );
                let result = f.ret.as_ref().map_or_else(
                    || self.node(Node::Discard("_".into()), e.span),
                    |r| self.result(&r.value, r.err.as_ref(), r.env.as_ref(), e.span),
                );
                self.form("->", [ps, result, self.block(&f.body)], e.span)
            }
            ExprKind::Case { subject, clauses } => {
                let body_span =
                    self.bracket(Span::new(subject.span.end, e.span.end), TokenKind::LBrace);
                self.form(
                    ".",
                    [self.expr(subject), self.clauses(clauses, body_span)],
                    e.span,
                )
            }
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
        self.patterns
            .borrow()
            .get(&(p as *const Pattern))
            .expect("pattern child was not prepared")
            .clone()
    }
    fn encode_pattern(&self, p: &Pattern) -> Syntax {
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
