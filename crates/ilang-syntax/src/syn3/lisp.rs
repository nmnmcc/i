//! The Lisp surface of syntax three (docs/design/syntax-3.md §6.8, with the
//! services forms): the tree to Lisp text, and Lisp datums to the tree.
//!
//! Rows that §6.8 does not list are those of meta.md §2.3. The services
//! forms are: a function type `(-> P (<- R S))`, a function head
//! `(-> (, p…) (<- R (, (: db Db) Log .. N)) body)` with `_` for no result,
//! type parameters as a leading Square, `(-> [A N] (, x) R e)`, and the
//! provide `(<- e s)`.

use super::ast::*;
use crate::lisp::{Datum, DatumKind as K, show};
use crate::span::Span;
use crate::{Code, Diagnostic};

/// Readers of one datum by the positions of `i/syntax` (§6.7).
pub mod position;

// ---- The tree as Lisp datums ----
//
// One builder serves the Lisp printer (`to_lisp` is `show` of each datum),
// and the syntax given to macros. An atom that stands for a written name has
// the span of that name; a compound has the span of its node. A symbol that
// the tree does not keep has the span of its token when it is an operator
// written between or before its operands (`+`, `.`, `:`, `=`, `..`, `@`,
// `<-`) and the builder runs inside [`with_tokens`]; every other symbol
// (`,`, `->`, `|`, `?`) has the span of its node, and an operator whose
// token is not found has the empty span at the start of the gap searched.

/// The tokens of a normal-surface file, for the spans of operators.
pub struct TokenIndex {
    /// Start, end and spelling of each symbol token, in source order.
    tokens: Vec<(Span, &'static str)>,
}

impl TokenIndex {
    pub fn new(source: &str) -> TokenIndex {
        let lexed = crate::lexer::lex3(source);
        TokenIndex {
            tokens: lexed
                .tokens
                .iter()
                .filter(|t| t.span.start != t.span.end)
                .map(|t| (t.span, t.kind.text()))
                .collect(),
        }
    }

    /// The first token spelled `text` that lies in `gap`.
    fn between(&self, text: &str, gap: Span) -> Option<Span> {
        let first = self.tokens.partition_point(|(s, _)| s.start < gap.start);
        self.tokens[first..]
            .iter()
            .take_while(|(s, _)| s.end <= gap.end)
            .find(|(_, t)| *t == text)
            .map(|(s, _)| *s)
    }

    /// The token spelled `text` that ends right before `at`.
    fn before(&self, text: &str, at: u32) -> Option<Span> {
        let next = self.tokens.partition_point(|(s, _)| s.end <= at);
        let (span, found) = self.tokens.get(next.checked_sub(1)?)?;
        (*found == text).then_some(*span)
    }
}

thread_local! {
    static TOKENS: std::cell::Cell<Option<std::sync::Arc<TokenIndex>>> =
        const { std::cell::Cell::new(None) };
}

/// Builds datums with the spans of the operator tokens of `index`.
pub fn with_tokens<T>(index: &std::sync::Arc<TokenIndex>, f: impl FnOnce() -> T) -> T {
    let before = TOKENS.with(|t| t.replace(Some(index.clone())));
    let out = f();
    TOKENS.with(|t| t.set(before));
    out
}

fn token(find: impl FnOnce(&TokenIndex) -> Option<Span>) -> Option<Span> {
    TOKENS.with(|t| {
        let index = t.take();
        let found = index.as_deref().and_then(find);
        t.set(index);
        found
    })
}

fn at(kind: K, span: Span) -> Datum {
    Datum {
        kind,
        span,
        docs: Vec::new(),
    }
}

/// A symbol that stands for its whole node.
fn op(text: &str, span: Span) -> Datum {
    at(K::Symbol(text.into()), span)
}

/// An operator written in `gap`.
fn op_in(text: &str, gap: Span) -> Datum {
    let span = token(|i| i.between(text, gap)).unwrap_or(Span::new(gap.start, gap.start));
    at(K::Symbol(text.into()), span)
}

/// `..` written right before the node at `next`.
fn spread(next: Span) -> Datum {
    let span = token(|i| i.before("..", next.start)).unwrap_or(Span::new(next.start, next.start));
    at(K::Symbol("..".into()), span)
}

/// A name as the reader gives it: `Up`, `_discard`, or `name`.
fn atom(text: &str, span: Span) -> Datum {
    let kind = if text.starts_with('_') {
        K::Discard(text.into())
    } else if text.chars().next().is_some_and(char::is_uppercase) {
        K::UpName(text.into())
    } else {
        K::Name(text.into())
    };
    at(kind, span)
}

fn name(n: &Ident) -> Datum {
    atom(&n.name, n.span.0)
}

fn binder(b: &Binder) -> Datum {
    atom(&b.name, b.span.0)
}

fn list(span: Span, parts: Vec<Datum>) -> Datum {
    at(K::Round(parts), span)
}

fn form(head: &str, span: Span, parts: impl IntoIterator<Item = Datum>) -> Datum {
    list(span, std::iter::once(op(head, span)).chain(parts).collect())
}

/// `(head parts…)` whose head is an operator written in `gap`.
fn form_in(head: &str, gap: Span, span: Span, parts: impl IntoIterator<Item = Datum>) -> Datum {
    list(
        span,
        std::iter::once(op_in(head, gap)).chain(parts).collect(),
    )
}

/// The gap between two nodes.
fn gap(a: Span, b: Span) -> Span {
    Span::new(a.end, b.start.max(a.end))
}

/// `(head parts…)` with a head that is a datum itself (a call).
fn apply(head: Datum, span: Span, parts: impl IntoIterator<Item = Datum>) -> Datum {
    list(span, std::iter::once(head).chain(parts).collect())
}

fn qualified(module: &Option<Ident>, n: &Ident, span: Span) -> Datum {
    match module {
        Some(m) => form_in(
            ".",
            gap(m.span.0, n.span.0),
            m.span.0.to(n.span.0).to(span),
            [name(m), name(n)],
        ),
        None => name(n),
    }
}

fn string(s: &str, span: Span) -> Datum {
    at(K::String(s.into()), span)
}

/// Writes a module in the Lisp surface, one top-level datum per line.
pub fn to_lisp(m: &Module) -> String {
    fn text(head: &str, parts: impl IntoIterator<Item = String>) -> String {
        let parts = parts.into_iter().collect::<Vec<_>>();
        if parts.is_empty() {
            format!("({head})")
        } else {
            format!("({head} {})", parts.join(" "))
        }
    }
    let plain = |n: &Ident| n.name.clone();
    let mut lines = Vec::new();
    lines.extend(m.doc.iter().map(|d| format!("////{d}")));
    if !m.imports.is_empty() {
        lines.push(text(
            ",",
            m.imports.iter().map(|i| {
                let path = text("/", i.path.iter().map(plain));
                match (&i.alias, i.names.is_empty()) {
                    (None, true) => path,
                    (Some(a), true) => text("=", [plain(a), path]),
                    (None, false) => text("=", [text(",", i.names.iter().map(plain)), path]),
                    (Some(a), false) => {
                        text("=", [plain(a), text(",", i.names.iter().map(plain)), path])
                    }
                }
            }),
        ));
    }
    if !m.exports.is_empty() {
        lines.push(text(
            ",",
            m.exports.iter().map(|e| {
                if e.constructors {
                    text(&e.name.name, ["..".into()])
                } else {
                    plain(&e.name)
                }
            }),
        ));
    }
    for item in &m.items {
        lines.extend(item.doc.iter().map(|d| format!("///{d}")));
        lines.push(show(&item_datum(item)));
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

/// An item as a datum of the rows of §6.8.
pub fn item_datum(item: &Item) -> Datum {
    let span = item.span.0;
    let mut out = match &item.kind {
        ItemKind::Bind(b) => binding(b),
        ItemKind::Decl { name: n, ty } => {
            form_in(":", gap(n.span.0, ty.span.0), span, [name(n), typ(ty)])
        }
        ItemKind::TypeDef(def) => {
            let head = if def.head.is_empty() {
                name(&def.name)
            } else {
                let end = def.head.last().map_or(def.name.span.0, |t| t.span.0);
                apply(
                    name(&def.name),
                    def.name.span.0.to(end),
                    def.head.iter().map(typ),
                )
            };
            let rhs = match &def.rhs {
                TypeRhs::Opaque => atom("Opaque", Span::new(span.end, span.end)),
                TypeRhs::Data(items) => data_type(items, def.name.span.0.to(span)),
                TypeRhs::Type(ty) => {
                    let grouped = match &ty.kind {
                        TypeKind::Data(_) => true,
                        TypeKind::Named {
                            module: None,
                            name,
                            args,
                        } => name.name == "Opaque" && args.is_empty(),
                        _ => false,
                    };
                    if grouped {
                        at(K::Curly(vec![typ(ty)]), ty.span.0)
                    } else {
                        typ(ty)
                    }
                }
            };
            form("=", span, [head, rhs])
        }
        ItemKind::Macro { module, name, args } => {
            apply(qualified(module, name, name.span.0), span, ditems(args))
        }
    };
    for a in item.attributes.iter().rev() {
        let n = qualified(&a.module, &a.name, a.name.span.0);
        let a = match &a.args {
            Some(args) => apply(n, a.span.0, ditems(args)),
            None => n,
        };
        let span = a.span.to(out.span);
        out = form_in("@", Span::new(span.start, a.span.end), span, [a, out]);
    }
    out
}

fn binding(b: &Binding) -> Datum {
    form_in(
        "=",
        gap(b.pattern.span.0, b.value.span.0),
        b.span.0,
        [pattern(&b.pattern), expr(&b.value)],
    )
}

fn data_type(items: &[TItem], span: Span) -> Datum {
    form(
        ",",
        span,
        items.iter().flat_map(|i| match i {
            TItem::Pos(t) => vec![typ(t)],
            TItem::Label(n, t) => vec![form_in(
                "=",
                gap(n.span.0, t.span.0),
                n.span.0.to(t.span.0),
                [name(n), typ(t)],
            )],
            TItem::Spread(t) => vec![spread(t.span.0), typ(t)],
        }),
    )
}

fn generics(gs: &[Ident]) -> Option<Datum> {
    let first = gs.first()?;
    let last = gs.last()?;
    Some(at(
        K::Square(gs.iter().map(name).collect()),
        first.span.0.to(last.span.0),
    ))
}

/// A type as a datum of the rows of §6.8.
pub fn type_datum(t: &Type) -> Datum {
    typ(t)
}

fn typ(t: &Type) -> Datum {
    let span = t.span.0;
    match &t.kind {
        TypeKind::Lit(LitValue::Int(n)) => {
            let value = at(K::Int(n.magnitude().clone()), span);
            if n.sign() == num_bigint::Sign::Minus {
                form("-", span, [value])
            } else {
                value
            }
        }
        TypeKind::Lit(LitValue::Str(s)) => string(s, span),
        TypeKind::Named { module, name, args } => {
            let n = qualified(module, name, name.span.0);
            if args.is_empty() {
                n
            } else {
                apply(n, span, args.iter().map(typ))
            }
        }
        TypeKind::Call { module, name, args } => apply(
            qualified(module, name, name.span.0),
            span,
            args.iter().map(typ),
        ),
        TypeKind::ValueOf { module, name } => qualified(module, name, span),
        TypeKind::Hole => atom("_", span),
        TypeKind::Data(items) => data_type(items, span),
        TypeKind::Fn(f) => {
            let ret = typ(&f.ret);
            let ret = match &f.services {
                Some(s) => form("<-", f.ret.span.0.to(s.span.0), [ret, typ(s)]),
                None => ret,
            };
            form(
                "->",
                span,
                generics(&f.generics)
                    .into_iter()
                    .chain([typ(&f.param), ret]),
            )
        }
        TypeKind::Union(ts) => form("|", span, ts.iter().map(typ)),
        TypeKind::Inter(ts) => form("&", span, ts.iter().map(typ)),
        TypeKind::Diff(ts) => form("-", span, ts.iter().map(typ)),
    }
}

fn ditems(items: &[DItem]) -> Vec<Datum> {
    items
        .iter()
        .flat_map(|i| match i {
            DItem::Pos(e) => vec![expr(e)],
            DItem::Label(n, e) => vec![form_in(
                "=",
                gap(n.span.0, e.span.0),
                n.span.0.to(e.span.0),
                [name(n), expr(e)],
            )],
            DItem::Spread(e) => vec![spread(e.span.0), expr(e)],
        })
        .collect()
}

fn segment<X>(s: &Segment<X>, f: impl Fn(&X) -> Datum) -> Datum {
    let v = f(&s.value);
    match &s.format {
        None => v,
        Some(fmt) => {
            let fmt = match &fmt.arg {
                None => name(&fmt.name),
                Some(arg) => apply(name(&fmt.name), fmt.name.span.0.to(arg.span.0), [expr(arg)]),
            };
            let span = v.span.to(fmt.span);
            form(":", span, [v, fmt])
        }
    }
}

fn function(f: &Function) -> Datum {
    let span = f.span.0;
    let head_end = f.body.span.0.start;
    let head = Span::new(span.start, head_end.max(span.start));
    let items = form(
        ",",
        head,
        f.params.iter().map(|p| {
            let cover = p
                .choices
                .iter()
                .map(|c| c.span.0)
                .reduce(Span::to)
                .unwrap_or(head);
            let choices = if p.choices.len() == 1 {
                pattern_parts(&p.choices[0])
            } else {
                vec![form("|", cover, p.choices.iter().map(pattern))]
            };
            match &p.key {
                Some(k) => form(
                    "=",
                    k.span.0.to(cover),
                    std::iter::once(name(k)).chain(choices),
                ),
                None if choices.len() == 1 => choices.into_iter().next().unwrap(),
                None => form("=", cover, choices),
            }
        }),
    );
    let params = match &f.guard {
        Some(g) => form("?", head, [items, expr(g)]),
        None => items,
    };
    let ret = match &f.ret {
        None => atom("_", Span::new(head_end, head_end)),
        Some(t) if matches!(t.kind, TypeKind::Hole) => at(K::Curly(vec![typ(t)]), t.span.0),
        Some(t) => typ(t),
    };
    let ret = match &f.services {
        None => ret,
        Some(items) => {
            let services = form(
                ",",
                head,
                items.iter().flat_map(|s| {
                    let item = form(
                        ":",
                        s.name.span.0.to(s.ty.span.0),
                        [name(&s.name), typ(&s.ty)],
                    );
                    if s.rest {
                        vec![spread(s.name.span.0), item]
                    } else {
                        vec![item]
                    }
                }),
            );
            form("<-", head, [ret, services])
        }
    };
    let body = match f.body.stmts.as_slice() {
        [Stmt::Expr(e)] => expr(e),
        _ => block(&f.body),
    };
    form(
        "->",
        span,
        generics(&f.generics).into_iter().chain([params, ret, body]),
    )
}

fn block(b: &Block) -> Datum {
    at(
        K::Curly(
            b.stmts
                .iter()
                .map(|s| match s {
                    Stmt::Bind(b) => binding(b),
                    Stmt::Expr(e) => expr(e),
                })
                .collect(),
        ),
        b.span.0,
    )
}

/// An expression as a datum of the rows of §6.8.
pub fn expr_datum(e: &Expr) -> Datum {
    expr(e)
}

fn expr(e: &Expr) -> Datum {
    let span = e.span.0;
    match &e.kind {
        ExprKind::Quote(d) => (*d.0).clone(),
        ExprKind::Int(n) => at(K::Int(n.clone()), span),
        ExprKind::Float(n) => at(K::Float(*n), span),
        ExprKind::String(s) => string(s, span),
        ExprKind::Interp(parts) => {
            // The Lisp surface has no interpolation: the `<>` of the parts.
            let mut operands: Vec<Datum> = parts
                .iter()
                .map(|p| match p {
                    InterpPart::Text(s) => string(s, Span::new(span.start, span.start)),
                    InterpPart::Expr(e) => expr(e),
                })
                .collect();
            if operands.len() == 1 {
                operands.insert(0, string("", Span::new(span.start, span.start)));
            }
            let mut it = operands.into_iter();
            let first = it
                .next()
                .unwrap_or_else(|| string("", Span::new(span.start, span.start)));
            it.fold(first, |acc, x| form("<>", span, [acc, x]))
        }
        ExprKind::Name { module, name } => qualified(module, name, span),
        ExprKind::Data(items) => form(",", span, ditems(items)),
        ExprKind::List { items, rest } => at(
            K::Square(
                items
                    .iter()
                    .map(expr)
                    .chain(rest.iter().flat_map(|r| [spread(r.span.0), expr(r)]))
                    .collect(),
            ),
            span,
        ),
        ExprKind::BitArray(xs) => at(K::Bits(xs.iter().map(|s| segment(s, expr)).collect()), span),
        ExprKind::Block(b) => block(b),
        ExprKind::Chain(fs) if fs.len() == 1 => function(&fs[0]),
        ExprKind::Chain(fs) => form("|", span, fs.iter().map(function)),
        ExprKind::Call { callee, args } => apply(expr(callee), span, ditems(args)),
        ExprKind::Provide { target, services } => form_in(
            "<-",
            gap(target.span.0, services.span.0),
            span,
            [expr(target), expr(services)],
        ),
        ExprKind::Field { target, name: n } => form_in(
            ".",
            gap(target.span.0, n.span.0),
            span,
            [expr(target), name(n)],
        ),
        ExprKind::Index { target, index } => form(
            ".",
            span,
            [
                expr(target),
                at(K::Int(index.clone()), Span::new(span.end, span.end)),
            ],
        ),
        ExprKind::Unary { op, expr: e } => form_in(
            match op {
                UnOp::Neg => "-",
                UnOp::NegFloat => "-.",
            },
            Span::new(span.start, e.span.0.start.max(span.start)),
            span,
            [expr(e)],
        ),
        ExprKind::Binary { op, lhs, rhs } => form_in(
            op.text(),
            gap(lhs.span.0, rhs.span.0),
            span,
            [expr(lhs), expr(rhs)],
        ),
        ExprKind::Pipe { lhs, rhs } => form_in(
            "|>",
            gap(lhs.span.0, rhs.span.0),
            span,
            [expr(lhs), expr(rhs)],
        ),
    }
}

/// The parts of a conjunction, or the one pattern.
fn pattern_parts(p: &Pattern) -> Vec<Datum> {
    match &p.kind {
        PatternKind::And(parts) => parts.iter().map(pattern).collect(),
        _ => vec![pattern(p)],
    }
}

fn dpitems(items: &[DPItem], span: Span) -> Vec<Datum> {
    items
        .iter()
        .flat_map(|i| match i {
            DPItem::Pos(p) => vec![pattern(p)],
            DPItem::Key(k, p) => vec![form(
                "=",
                k.span.0.to(p.span.0),
                std::iter::once(name(k)).chain(pattern_parts(p)),
            )],
            DPItem::Rest(None) => vec![op("..", span)],
            DPItem::Rest(Some(b)) => vec![spread(b.span.0), binder(b)],
        })
        .collect()
}

/// A pattern as a datum of the rows of §6.8.
pub fn pattern_datum(p: &Pattern) -> Datum {
    pattern(p)
}

fn pattern(p: &Pattern) -> Datum {
    let span = p.span.0;
    let number = |negative: bool, head: &str, value: Datum| {
        if negative {
            form(head, span, [value])
        } else {
            value
        }
    };
    match &p.kind {
        PatternKind::Quote(d) => (*d.0).clone(),
        PatternKind::Var(s) | PatternKind::Discard(s) => atom(s, span),
        PatternKind::Typed { binder: b, ty } => {
            form_in(":", gap(b.span.0, ty.span.0), span, [binder(b), typ(ty)])
        }
        PatternKind::Int { negative, value } => {
            number(*negative, "-", at(K::Int(value.clone()), span))
        }
        PatternKind::Float { negative, value } => {
            number(*negative, "-.", at(K::Float(*value), span))
        }
        PatternKind::String(s) => string(s, span),
        PatternKind::Prefix {
            prefix,
            suffix,
            rest,
        } => {
            let start = if !prefix.is_empty() || suffix.is_empty() {
                form(
                    "<>",
                    span,
                    [
                        string(prefix, Span::new(span.start, span.start)),
                        binder(rest),
                    ],
                )
            } else {
                binder(rest)
            };
            if suffix.is_empty() {
                start
            } else {
                form(
                    "<>",
                    span,
                    [start, string(suffix, Span::new(span.end, span.end))],
                )
            }
        }
        PatternKind::Ctor {
            module,
            name: n,
            args,
        } => {
            let head = qualified(module, n, n.span.0);
            match args {
                None => head,
                Some(args) => apply(head, span, dpitems(args, span)),
            }
        }
        PatternKind::Data(items) => form(",", span, dpitems(items, span)),
        PatternKind::List { items, rest } => {
            at(
                K::Square(
                    items
                        .iter()
                        .map(pattern)
                        .chain(rest.iter().flat_map(|r| {
                            std::iter::once(op("..", span)).chain(r.iter().map(binder))
                        }))
                        .collect(),
                ),
                span,
            )
        }
        PatternKind::BitArray(xs) => at(
            K::Bits(xs.iter().map(|s| segment(s, pattern)).collect()),
            span,
        ),
        PatternKind::And(parts) => form("=", span, parts.iter().map(pattern)),
    }
}

// ---- Lisp datums as the tree ----

type R<X> = Result<X, Diagnostic>;

fn err<X>(d: &Datum, msg: impl Into<String>) -> R<X> {
    Err(Diagnostic::new(Code::Unexpected, d.span, msg))
}

/// Reads a syntax-three Lisp file.
pub fn from_lisp(source: &str) -> (Module, Vec<Diagnostic>) {
    let document = match crate::lisp::parse3(source) {
        Ok(d) => d,
        Err(diags) => return (Module::default(), diags),
    };
    let mut module = Module::default();
    let mut diags = Vec::new();
    let mut header = true;
    for d in &document.datums {
        for doc in &d.docs {
            if doc.module {
                module.doc.push(doc.text.clone());
            }
        }
        let result = (|| -> R<()> {
            if let Some(parts) = round(d, ",") {
                if !header {
                    return err(
                        d,
                        "the imports and the exports are the first datums of a file",
                    );
                }
                if parts
                    .iter()
                    .any(|p| round(p, "/").is_some() || import_path(p).is_some())
                {
                    module.imports = parts.iter().map(import).collect::<R<_>>()?;
                } else {
                    module.exports = parts.iter().map(export).collect::<R<_>>()?;
                }
                return Ok(());
            }
            header = false;
            let mut it = item(d)?;
            it.doc = d
                .docs
                .iter()
                .filter(|doc| !doc.module)
                .map(|doc| doc.text.clone())
                .collect();
            module.items.push(it);
            Ok(())
        })();
        if let Err(e) = result {
            diags.push(e);
        }
    }
    for doc in &document.trailing_docs {
        if doc.module {
            module.doc.push(doc.text.clone());
        }
    }
    (module, diags)
}

/// The unquoted parts of a quote, checked: `~e` and `~..e` hold expressions
/// in a template and patterns in a quote pattern; a splice stands in a
/// sequence (last, in a pattern); a quote does not hold a quote. The
/// captures of a quote pattern are returned in matcher order, a splice
/// marked `true`.
pub fn quote_captures(d: &Datum, pattern: bool) -> Result<Vec<(bool, Pattern)>, Diagnostic> {
    let bad = |d: &Datum, what: &str| {
        Diagnostic::new(
            Code::Unexpected,
            d.span,
            format!("datum is not valid as {what}"),
        )
    };
    let K::Quote(body) = &d.kind else {
        return Err(bad(d, "a quote"));
    };
    let mut work = vec![(&**body, false, true)];
    let mut captures = Vec::new();
    while let Some((d, in_sequence, last)) = work.pop() {
        match &d.kind {
            K::Unquote(inner) => {
                if pattern {
                    captures.push((false, pattern_of(inner)?));
                } else {
                    expr_of(inner)?;
                }
            }
            K::Splice(inner) => {
                if !in_sequence || (pattern && !last) {
                    return Err(bad(d, "a splice in this position"));
                }
                if pattern {
                    captures.push((true, pattern_of(inner)?));
                } else {
                    expr_of(inner)?;
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

/// Reads one item datum.
pub fn item_from_datum(d: &Datum) -> Result<Item, Diagnostic> {
    item(d)
}

/// Reads the item result of a macro: one item, or a Curly of items.
pub fn items_from_datum(d: &Datum) -> Result<Vec<Item>, Diagnostic> {
    match &d.kind {
        K::Curly(items) => items.iter().map(item).collect(),
        _ => Ok(vec![item(d)?]),
    }
}

/// Reads one expression datum.
pub fn expr_from_datum(d: &Datum) -> Result<Expr, Diagnostic> {
    expr_of(d)
}

/// Reads one type datum.
pub fn type_from_datum(d: &Datum) -> Result<Type, Diagnostic> {
    type_of(d)
}

/// Reads one pattern datum.
pub fn pattern_from_datum(d: &Datum) -> Result<Pattern, Diagnostic> {
    pattern_of(d)
}

/// The parts after the head of a Round whose head is the symbol `head`.
fn round<'a>(d: &'a Datum, head: &str) -> Option<&'a [Datum]> {
    match &d.kind {
        K::Round(parts) => match parts.first().map(|p| &p.kind) {
            Some(K::Symbol(s)) if s == head => Some(&parts[1..]),
            _ => None,
        },
        _ => None,
    }
}

fn symbol(d: &Datum) -> Option<&str> {
    match &d.kind {
        K::Symbol(s) => Some(s),
        _ => None,
    }
}

fn lower(d: &Datum) -> R<Ident> {
    match &d.kind {
        K::Name(s) => Ok(Ident::new(s.clone(), d.span)),
        _ => err(d, "expected a name"),
    }
}

fn upper(d: &Datum) -> R<Ident> {
    match &d.kind {
        K::UpName(s) => Ok(Ident::new(s.clone(), d.span)),
        _ => err(d, "expected an UpName"),
    }
}

fn any_name(d: &Datum) -> R<Ident> {
    match &d.kind {
        K::Name(s) | K::UpName(s) => Ok(Ident::new(s.clone(), d.span)),
        _ => err(d, "expected a name"),
    }
}

fn binder_of(d: &Datum) -> R<Binder> {
    match &d.kind {
        K::Name(s) => Ok(Binder {
            name: s.clone(),
            discard: false,
            span: Sp(d.span),
        }),
        K::Discard(s) => Ok(Binder {
            name: s.clone(),
            discard: true,
            span: Sp(d.span),
        }),
        _ => err(d, "expected a name or a discard"),
    }
}

fn import_path(d: &Datum) -> Option<&[Datum]> {
    let parts = round(d, "=")?;
    round(parts.last()?, "/")
}

fn import(d: &Datum) -> R<Import> {
    let path_of = |d: &Datum| -> R<Vec<Ident>> {
        match round(d, "/") {
            Some(parts) if parts.len() >= 2 => parts.iter().map(lower).collect(),
            _ => err(d, "a module path `(/ a b)` has two or more names"),
        }
    };
    let names_of = |d: &Datum| -> R<Vec<Ident>> {
        match round(d, ",") {
            Some(parts) if !parts.is_empty() => parts.iter().map(any_name).collect(),
            _ => err(d, "expected a list of import names `(, C y)`"),
        }
    };
    let (alias, names, path) = match round(d, "=") {
        None => (None, Vec::new(), path_of(d)?),
        Some([a, p]) if round(a, ",").is_some() => (None, names_of(a)?, path_of(p)?),
        Some([a, p]) => (Some(lower(a)?), Vec::new(), path_of(p)?),
        Some([a, ns, p]) => (Some(lower(a)?), names_of(ns)?, path_of(p)?),
        Some(_) => return err(d, "an import item `(= …)` has two or three parts"),
    };
    Ok(Import {
        alias,
        names,
        path,
        span: Sp(d.span),
    })
}

fn export(d: &Datum) -> R<Export> {
    if let K::Round(parts) = &d.kind
        && let [n, dots] = parts.as_slice()
        && symbol(dots) == Some("..")
    {
        return Ok(Export {
            name: upper(n)?,
            constructors: true,
            span: Sp(d.span),
        });
    }
    Ok(Export {
        name: any_name(d)?,
        constructors: false,
        span: Sp(d.span),
    })
}

/// `x`, or `(. a x)`.
fn member(d: &Datum) -> Option<(Option<Ident>, Ident)> {
    match &d.kind {
        K::Name(s) => Some((None, Ident::new(s.clone(), d.span))),
        _ => match round(d, ".")? {
            [m, n] => Some((Some(lower(m).ok()?), lower(n).ok()?)),
            _ => None,
        },
    }
}

fn item(d: &Datum) -> R<Item> {
    let mut attributes = Vec::new();
    let mut cur = d;
    while let Some(parts) = round(cur, "@") {
        let [a, inner] = parts else {
            return err(cur, "`(@ attribute Item)` has two parts");
        };
        let (module, name, args) = match member(a) {
            Some((module, name)) => (module, name, None),
            None => match &a.kind {
                K::Round(parts) if !parts.is_empty() => {
                    let Some((module, name)) = member(&parts[0]) else {
                        return err(a, "expected the name of an attribute");
                    };
                    (module, name, Some(ditems_of(&parts[1..])?))
                }
                _ => return err(a, "expected an attribute"),
            },
        };
        attributes.push(Attribute {
            module,
            name,
            args,
            span: Sp(a.span),
        });
        cur = inner;
    }
    let kind = if let Some(parts) = round(cur, "=") {
        let [head, value] = parts else {
            return err(cur, "a definition `(= …)` has two parts");
        };
        let type_head = match &head.kind {
            K::UpName(_) => Some((upper(head)?, Vec::new())),
            K::Round(hs) if matches!(hs.first().map(|h| &h.kind), Some(K::UpName(_))) => Some((
                upper(&hs[0])?,
                hs[1..].iter().map(type_of).collect::<R<Vec<_>>>()?,
            )),
            _ => None,
        };
        match type_head {
            Some((name, head)) => {
                let rhs = match &value.kind {
                    K::UpName(s) if s == "Opaque" => TypeRhs::Opaque,
                    K::Round(_) if round(value, ",").is_some() => {
                        TypeRhs::Data(titems_of(round(value, ",").unwrap(), false)?)
                    }
                    _ => TypeRhs::Type(type_of(value)?),
                };
                ItemKind::TypeDef(TypeDef { name, head, rhs })
            }
            None => ItemKind::Bind(Binding {
                pattern: pattern_of(head)?,
                value: expr_of(value)?,
                span: Sp(cur.span),
            }),
        }
    } else if let Some(parts) = round(cur, ":") {
        let [n, ty] = parts else {
            return err(cur, "a declaration `(: x T)` has two parts");
        };
        ItemKind::Decl {
            name: lower(n)?,
            ty: type_of(ty)?,
        }
    } else if let K::Round(parts) = &cur.kind
        && let Some((module, name)) = parts.first().and_then(member)
    {
        ItemKind::Macro {
            module,
            name,
            args: ditems_of(&parts[1..])?,
        }
    } else {
        return err(cur, "expected an item: `(= …)`, `(: x T)` or a macro call");
    };
    Ok(Item {
        doc: Vec::new(),
        attributes,
        kind,
        span: Sp(d.span),
    })
}

// Types.

fn titems_of(parts: &[Datum], services: bool) -> R<Vec<TItem>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        let d = &parts[i];
        if symbol(d) == Some("..") {
            let Some(next) = parts.get(i + 1) else {
                return err(d, "`..` needs a type after it");
            };
            if !services {
                return err(
                    d,
                    "a data type has no spread; `.. N` is written only after `<-`",
                );
            }
            out.push(TItem::Spread(type_of(next)?));
            i += 2;
            continue;
        }
        match round(d, "=") {
            Some([l, t]) => out.push(TItem::Label(lower(l)?, type_of(t)?)),
            Some(_) => return err(d, "a labelled item `(= label T)` has two parts"),
            None => out.push(TItem::Pos(type_of(d)?)),
        }
        i += 1;
    }
    Ok(out)
}

fn generics_of(d: &Datum) -> Option<R<Vec<Ident>>> {
    match &d.kind {
        K::Square(parts) => Some(parts.iter().map(upper).collect()),
        _ => None,
    }
}

fn type_of(d: &Datum) -> R<Type> {
    let named = |module, name, args| TypeKind::Named { module, name, args };
    let kind = match &d.kind {
        K::Int(n) => TypeKind::Lit(LitValue::Int(n.clone().into())),
        K::String(s) => TypeKind::Lit(LitValue::Str(s.clone())),
        K::UpName(_) => named(None, upper(d)?, Vec::new()),
        K::Name(_) => TypeKind::ValueOf {
            module: None,
            name: lower(d)?,
        },
        K::Discard(s) if s == "_" => TypeKind::Hole,
        K::Curly(parts) => match parts.as_slice() {
            [inner] => return type_of(inner),
            _ => return err(d, "a type group `{ T }` has one part"),
        },
        K::Round(parts) => {
            let Some(head) = parts.first() else {
                return err(d, "`()` is not a type; the empty data type is `(,)`");
            };
            let rest = &parts[1..];
            match &head.kind {
                K::Symbol(s) => match (s.as_str(), rest) {
                    (",", _) => TypeKind::Data(titems_of(rest, false)?),
                    ("|", [_, _, ..]) => {
                        TypeKind::Union(rest.iter().map(type_of).collect::<R<_>>()?)
                    }
                    ("&", [_, _, ..]) => {
                        TypeKind::Inter(rest.iter().map(type_of).collect::<R<_>>()?)
                    }
                    ("-", [k]) => match &k.kind {
                        K::Int(n) => {
                            TypeKind::Lit(LitValue::Int(-num_bigint::BigInt::from(n.clone())))
                        }
                        _ => return err(k, "`(- k)` in a type is a negative integer literal"),
                    },
                    ("-", [_, _, ..]) => {
                        TypeKind::Diff(rest.iter().map(type_of).collect::<R<_>>()?)
                    }
                    (".", [m, n]) => match &n.kind {
                        K::UpName(_) => named(Some(lower(m)?), upper(n)?, Vec::new()),
                        _ => TypeKind::ValueOf {
                            module: Some(lower(m)?),
                            name: lower(n)?,
                        },
                    },
                    ("->", _) => {
                        let (generics, rest) = match rest.first().and_then(generics_of) {
                            Some(g) => (g?, &rest[1..]),
                            None => (Vec::new(), rest),
                        };
                        let [param, ret] = rest else {
                            return err(
                                d,
                                "a function type `(-> P R)` has a parameter part and a result",
                            );
                        };
                        let (ret, services) = match round(ret, "<-") {
                            Some([r, s]) => {
                                let s = match round(s, ",") {
                                    Some(items) => {
                                        Type::new(TypeKind::Data(titems_of(items, true)?), s.span)
                                    }
                                    None => type_of(s)?,
                                };
                                (type_of(r)?, Some(s))
                            }
                            Some(_) => return err(ret, "`(<- R S)` has two parts"),
                            None => (type_of(ret)?, None),
                        };
                        TypeKind::Fn(Box::new(FnType {
                            generics,
                            param: type_of(param)?,
                            services,
                            ret,
                        }))
                    }
                    _ => return err(d, "this is not a type"),
                },
                K::UpName(_) if !rest.is_empty() => named(
                    None,
                    upper(head)?,
                    rest.iter().map(type_of).collect::<R<_>>()?,
                ),
                K::Name(_) => TypeKind::Call {
                    module: None,
                    name: lower(head)?,
                    args: rest.iter().map(type_of).collect::<R<_>>()?,
                },
                K::Round(_) => match round(head, ".") {
                    Some([m, n]) => {
                        let args = rest.iter().map(type_of).collect::<R<_>>()?;
                        match &n.kind {
                            K::UpName(_) if !rest.is_empty() => {
                                named(Some(lower(m)?), upper(n)?, args)
                            }
                            K::Name(_) => TypeKind::Call {
                                module: Some(lower(m)?),
                                name: lower(n)?,
                                args,
                            },
                            _ => return err(d, "a named type with arguments has one or more"),
                        }
                    }
                    _ => return err(d, "this is not a type"),
                },
                _ => return err(d, "this is not a type"),
            }
        }
        _ => return err(d, "this is not a type"),
    };
    Ok(Type::new(kind, d.span))
}

// Expressions.

fn ditems_of(parts: &[Datum]) -> R<Vec<DItem>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        let d = &parts[i];
        if symbol(d) == Some("..") {
            let Some(next) = parts.get(i + 1) else {
                return err(d, "`..` needs an expression after it");
            };
            out.push(DItem::Spread(expr_of(next)?));
            i += 2;
            continue;
        }
        match round(d, "=") {
            Some([l, e]) => out.push(DItem::Label(lower(l)?, expr_of(e)?)),
            Some(_) => return err(d, "a labelled item `(= label e)` has two parts"),
            None => out.push(DItem::Pos(expr_of(d)?)),
        }
        i += 1;
    }
    Ok(out)
}

fn segment_of<X>(d: &Datum, value: impl Fn(&Datum) -> R<X>) -> R<Segment<X>> {
    let Some([v, f]) = round(d, ":") else {
        return Ok(Segment {
            value: value(d)?,
            format: None,
        });
    };
    let format = match &f.kind {
        K::Name(_) => Format {
            name: lower(f)?,
            arg: None,
        },
        K::Round(parts) => match parts.as_slice() {
            [n, arg] => Format {
                name: lower(n)?,
                arg: Some(Box::new(expr_of(arg)?)),
            },
            _ => return err(f, "a format is `f` or `(f e)`"),
        },
        _ => return err(f, "a format is `f` or `(f e)`"),
    };
    Ok(Segment {
        value: value(v)?,
        format: Some(format),
    })
}

fn stmt_of(d: &Datum) -> R<Stmt> {
    match round(d, "=") {
        Some([p, e]) => Ok(Stmt::Bind(Binding {
            pattern: pattern_of(p)?,
            value: expr_of(e)?,
            span: Sp(d.span),
        })),
        Some(_) => err(d, "a binding `(= p e)` has two parts"),
        None => Ok(Stmt::Expr(expr_of(d)?)),
    }
}

fn block_of(d: &Datum, parts: &[Datum]) -> R<Block> {
    if parts.is_empty() {
        return err(
            d,
            "a block has one or more parts; the empty match is `core.never`",
        );
    }
    let stmts = parts.iter().map(stmt_of).collect::<R<Vec<_>>>()?;
    if !matches!(stmts.last(), Some(Stmt::Expr(_))) {
        return err(d, "a block ends with an expression");
    }
    Ok(Block {
        stmts,
        span: Sp(d.span),
    })
}

fn pitem_of(d: &Datum) -> R<PItem> {
    let choices = |d: &Datum| -> R<Vec<Pattern>> {
        match round(d, "|") {
            Some(ps) if ps.len() >= 2 => ps.iter().map(pattern_of).collect(),
            Some(_) => err(d, "alternatives `(| p1 p2 …)` have two or more parts"),
            None => Ok(vec![pattern_of(d)?]),
        }
    };
    if let Some(parts) = round(d, "=")
        && let Some(K::Name(_)) = parts.first().map(|p| &p.kind)
    {
        // P7: a key item.
        let key = lower(&parts[0])?;
        let choices = match &parts[1..] {
            [] => return err(d, "a key item `(= key p)` has a pattern"),
            [one] => choices(one)?,
            more => vec![Pattern::new(
                PatternKind::And(more.iter().map(pattern_of).collect::<R<_>>()?),
                d.span,
            )],
        };
        return Ok(PItem {
            key: Some(key),
            choices,
        });
    }
    Ok(PItem {
        key: None,
        choices: choices(d)?,
    })
}

fn function_of(d: &Datum) -> R<Function> {
    let Some(rest) = round(d, "->") else {
        return err(d, "expected a function `(-> params R body)`");
    };
    let (generics, rest) = match rest.first().and_then(generics_of) {
        Some(g) => (g?, &rest[1..]),
        None => (Vec::new(), rest),
    };
    let [params, ret, body] = rest else {
        return err(d, "a function `(-> params R body)` has three parts");
    };
    let (params, guard) = match round(params, "?") {
        Some([ps, g]) => (ps, Some(expr_of(g)?)),
        Some(_) => return err(params, "a guard `(? (, …) g)` has two parts"),
        None => (params, None),
    };
    let Some(items) = round(params, ",") else {
        return err(
            params,
            "the parameters of a function are a data pattern `(, …)`",
        );
    };
    if items.iter().any(|i| symbol(i) == Some("..")) {
        return err(params, "a parameter group has no rest `..` at its top");
    }
    let params = items.iter().map(pitem_of).collect::<R<Vec<_>>>()?;
    let (ret, services) = match round(ret, "<-") {
        Some([r, s]) => {
            let mut out = Vec::new();
            let Some(parts) = round(s, ",") else {
                return err(s, "the service pattern of a function is `(, (: name T) …)`");
            };
            let mut i = 0;
            while i < parts.len() {
                let rest = symbol(&parts[i]) == Some("..");
                if rest {
                    i += 1;
                }
                let Some(p) = parts.get(i) else {
                    return err(s, "`..` needs `(: name N)` after it");
                };
                let Some([b, t]) = round(p, ":") else {
                    return err(p, "a service is written `(: name T)`");
                };
                out.push(SItem {
                    rest,
                    name: lower(b)?,
                    ty: type_of(t)?,
                });
                i += 1;
            }
            (r, Some(out))
        }
        Some(_) => return err(ret, "`(<- R S)` has two parts"),
        None => (ret, None),
    };
    let ret = match &ret.kind {
        K::Discard(s) if s == "_" => None,
        _ => Some(type_of(ret)?),
    };
    // A body is one expression. A Curly with one expression is a group in
    // the body; any other Curly is the body's block.
    let body = match &body.kind {
        K::Curly(parts) if parts.len() != 1 || round(&parts[0], "=").is_some() => {
            block_of(body, parts)?
        }
        _ => Block {
            stmts: vec![Stmt::Expr(expr_of(body)?)],
            span: Sp(body.span),
        },
    };
    Ok(Function {
        generics,
        params,
        guard,
        services,
        ret,
        body,
        span: Sp(d.span),
    })
}

fn binop(s: &str) -> Option<BinOp> {
    use BinOp::*;
    [
        Or, And, Eq, NotEq, Lt, LtEq, Gt, GtEq, LtFloat, LtEqFloat, GtFloat, GtEqFloat, Concat,
        Add, Sub, AddFloat, SubFloat, Mul, Div, Rem, MulFloat, DivFloat,
    ]
    .into_iter()
    .find(|op| op.text() == s)
}

fn expr_of(d: &Datum) -> R<Expr> {
    let kind = match &d.kind {
        K::Int(n) => ExprKind::Int(n.clone()),
        K::Float(x) => ExprKind::Float(*x),
        K::String(s) => ExprKind::String(s.clone()),
        K::Name(_) | K::UpName(_) => ExprKind::Name {
            module: None,
            name: any_name(d)?,
        },
        K::Discard(_) => {
            return err(
                d,
                "a discard binds nothing, so it may not be used as a value",
            );
        }
        K::Quote(_) => ExprKind::Quote(Quoted(Box::new(d.clone()))),
        K::Square(parts) => {
            let mut items = Vec::new();
            let mut rest = None;
            let mut i = 0;
            while i < parts.len() {
                if symbol(&parts[i]) == Some("..") {
                    if items.is_empty() || i + 2 != parts.len() {
                        return err(&parts[i], "`.. rest` comes last, after the parts of a list");
                    }
                    rest = Some(Box::new(expr_of(&parts[i + 1])?));
                    break;
                }
                items.push(expr_of(&parts[i])?);
                i += 1;
            }
            ExprKind::List { items, rest }
        }
        K::Bits(parts) => ExprKind::BitArray(
            parts
                .iter()
                .map(|p| segment_of(p, expr_of))
                .collect::<R<_>>()?,
        ),
        K::Curly(parts) => ExprKind::Block(block_of(d, parts)?),
        K::Round(parts) => {
            let Some(head) = parts.first() else {
                return err(
                    d,
                    "the empty Round `()` has no position; the empty data is `(,)`",
                );
            };
            let rest = &parts[1..];
            match symbol(head) {
                Some(",") => ExprKind::Data(ditems_of(rest)?),
                Some("->") => ExprKind::Chain(vec![function_of(d)?]),
                Some("|") => {
                    if rest.len() < 2 {
                        return err(d, "a chain `(| F1 F2 …)` has two or more functions");
                    }
                    ExprKind::Chain(rest.iter().map(function_of).collect::<R<_>>()?)
                }
                Some("|>") => match rest {
                    [l, r] => ExprKind::Pipe {
                        lhs: Box::new(expr_of(l)?),
                        rhs: Box::new(expr_of(r)?),
                    },
                    _ => return err(d, "a pipe `(|> e f)` has two parts"),
                },
                Some("<-") => match rest {
                    [e, s] => ExprKind::Provide {
                        target: Box::new(expr_of(e)?),
                        services: Box::new(expr_of(s)?),
                    },
                    _ => return err(d, "a provide `(<- e s)` has two parts"),
                },
                Some(".") => match rest {
                    [t, n] => match (&t.kind, &n.kind) {
                        (K::Name(_), K::UpName(_)) => ExprKind::Name {
                            module: Some(lower(t)?),
                            name: upper(n)?,
                        },
                        (_, K::Name(_)) => ExprKind::Field {
                            target: Box::new(expr_of(t)?),
                            name: lower(n)?,
                        },
                        (_, K::Int(index)) => ExprKind::Index {
                            target: Box::new(expr_of(t)?),
                            index: index.clone(),
                        },
                        _ => return err(n, "expected a label or a decimal after the target"),
                    },
                    _ => return err(d, "`(. e x)` has two parts"),
                },
                Some(op @ ("-" | "-.")) if rest.len() == 1 => ExprKind::Unary {
                    op: if op == "-" { UnOp::Neg } else { UnOp::NegFloat },
                    expr: Box::new(expr_of(&rest[0])?),
                },
                Some(op) => match (binop(op), rest) {
                    (Some(op), [l, r]) => ExprKind::Binary {
                        op,
                        lhs: Box::new(expr_of(l)?),
                        rhs: Box::new(expr_of(r)?),
                    },
                    _ => {
                        return err(
                            d,
                            format!("`{op}` is not an expression form with these parts"),
                        );
                    }
                },
                None => ExprKind::Call {
                    callee: Box::new(expr_of(head)?),
                    args: ditems_of(rest)?,
                },
            }
        }
        _ => return err(d, "this is not an expression"),
    };
    Ok(Expr::new(kind, d.span))
}

// Patterns.

fn dpitems_of(parts: &[Datum]) -> R<Vec<DPItem>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        let d = &parts[i];
        if symbol(d) == Some("..") {
            let binder = match parts.get(i + 1) {
                Some(b) if i + 2 == parts.len() => Some(binder_of(b)?),
                Some(b) => return err(b, "the rest `..` is the last item of a data pattern"),
                None => None,
            };
            out.push(DPItem::Rest(binder));
            break;
        }
        match round(d, "=") {
            Some(ps) if matches!(ps.first().map(|p| &p.kind), Some(K::Name(_))) => {
                // P7: a key item.
                let key = lower(&ps[0])?;
                let pattern = match &ps[1..] {
                    [] => return err(d, "a key item `(= key p)` has a pattern"),
                    [one] => pattern_of(one)?,
                    more => Pattern::new(
                        PatternKind::And(more.iter().map(pattern_of).collect::<R<_>>()?),
                        d.span,
                    ),
                };
                out.push(DPItem::Key(key, pattern));
            }
            _ => out.push(DPItem::Pos(pattern_of(d)?)),
        }
        i += 1;
    }
    Ok(out)
}

fn string_of(d: &Datum) -> Option<&str> {
    match &d.kind {
        K::String(s) => Some(s),
        _ => None,
    }
}

fn pattern_of(d: &Datum) -> R<Pattern> {
    let kind = match &d.kind {
        K::Name(s) => PatternKind::Var(s.clone()),
        K::Discard(s) => PatternKind::Discard(s.clone()),
        K::Int(n) => PatternKind::Int {
            negative: false,
            value: n.clone(),
        },
        K::Float(x) => PatternKind::Float {
            negative: false,
            value: *x,
        },
        K::String(s) => PatternKind::String(s.clone()),
        K::UpName(_) => PatternKind::Ctor {
            module: None,
            name: upper(d)?,
            args: None,
        },
        K::Quote(_) => PatternKind::Quote(Quoted(Box::new(d.clone()))),
        K::Square(parts) => {
            let mut items = Vec::new();
            let mut rest = None;
            let mut i = 0;
            while i < parts.len() {
                if symbol(&parts[i]) == Some("..") {
                    rest = Some(match parts.get(i + 1) {
                        Some(b) if i + 2 == parts.len() => Some(binder_of(b)?),
                        Some(b) => return err(b, "the rest comes last in a list pattern"),
                        None => None,
                    });
                    break;
                }
                items.push(pattern_of(&parts[i])?);
                i += 1;
            }
            PatternKind::List { items, rest }
        }
        K::Bits(parts) => PatternKind::BitArray(
            parts
                .iter()
                .map(|p| segment_of(p, pattern_of))
                .collect::<R<_>>()?,
        ),
        K::Round(parts) => {
            let Some(head) = parts.first() else {
                return err(
                    d,
                    "the empty Round `()` has no position; the empty data pattern is `(,)`",
                );
            };
            let rest = &parts[1..];
            match (symbol(head), rest) {
                (Some(","), _) => PatternKind::Data(dpitems_of(rest)?),
                (Some(":"), [b, t]) => PatternKind::Typed {
                    binder: binder_of(b)?,
                    ty: type_of(t)?,
                },
                (Some("="), [_, _, ..]) => {
                    PatternKind::And(rest.iter().map(pattern_of).collect::<R<_>>()?)
                }
                (Some("-"), [k]) => match &k.kind {
                    K::Int(n) => PatternKind::Int {
                        negative: true,
                        value: n.clone(),
                    },
                    _ => return err(k, "`(- k)` in a pattern is a negative int literal"),
                },
                (Some("-."), [k]) => match &k.kind {
                    K::Float(x) => PatternKind::Float {
                        negative: true,
                        value: *x,
                    },
                    _ => return err(k, "`(-. k)` in a pattern is a negative float literal"),
                },
                (Some("<>"), [a, b]) => {
                    if let Some([p, x]) = round(a, "<>")
                        && let (Some(prefix), Some(suffix)) = (string_of(p), string_of(b))
                    {
                        PatternKind::Prefix {
                            prefix: prefix.to_string(),
                            suffix: suffix.to_string(),
                            rest: binder_of(x)?,
                        }
                    } else if let Some(prefix) = string_of(a) {
                        PatternKind::Prefix {
                            prefix: prefix.to_string(),
                            suffix: String::new(),
                            rest: binder_of(b)?,
                        }
                    } else if let Some(suffix) = string_of(b) {
                        PatternKind::Prefix {
                            prefix: String::new(),
                            suffix: suffix.to_string(),
                            rest: binder_of(a)?,
                        }
                    } else {
                        return err(d, "a string pattern has a string literal");
                    }
                }
                (Some("."), [m, n]) => PatternKind::Ctor {
                    module: Some(lower(m)?),
                    name: upper(n)?,
                    args: None,
                },
                (Some("|"), _) => {
                    return err(
                        d,
                        "alternatives `(| …)` are written only in the items of a parameter group",
                    );
                }
                (Some("?"), _) => {
                    return err(
                        d,
                        "a guard `(? (, …) g)` is written only around the parameters of a function",
                    );
                }
                (None, _) => {
                    let (module, name) = match &head.kind {
                        K::UpName(_) => (None, upper(head)?),
                        _ => match round(head, ".") {
                            Some([m, n]) => (Some(lower(m)?), upper(n)?),
                            _ => return err(head, "a constructor pattern begins with an UpName"),
                        },
                    };
                    PatternKind::Ctor {
                        module,
                        name,
                        args: Some(dpitems_of(rest)?),
                    }
                }
                _ => return err(d, "this is not a pattern"),
            }
        }
        _ => return err(d, "this is not a pattern"),
    };
    Ok(Pattern::new(kind, d.span))
}

// ---- Quotes of the older Lisp rows ----

fn sym(s: &str, span: Span) -> Datum {
    Datum {
        kind: K::Symbol(s.into()),
        span,
        docs: Vec::new(),
    }
}

fn node(kind: K, span: Span) -> Datum {
    Datum {
        kind,
        span,
        docs: Vec::new(),
    }
}

/// Rewrites a quoted datum written with the older Lisp rows to the rows of
/// §6.8, as far as the datum shows its position: functions get data-form
/// parameters and a service pattern, function types a data type and needs,
/// clause blocks become chains, the pipe and the match get the head `|>`,
/// and records become data. Labels `(: x e)` in calls and environment
/// statements `(! N e)` are left, because a datum does not show whether
/// `(: x T)` is a label or a typed pattern, nor where a block ends.
pub fn convert_quote(d: &Datum) -> Datum {
    let span = d.span;
    let all = |xs: &[Datum]| xs.iter().map(convert_quote).collect::<Vec<_>>();
    let is_clause = |c: &Datum| round(c, "->").is_some_and(|r| r.len() == 2);
    let clause = |c: &Datum| -> Datum {
        let [head, body] = round(c, "->").unwrap() else {
            unreachable!()
        };
        let (pat, guard) = match round(head, "?") {
            Some([p, g]) => (p, Some(g)),
            _ => (head, None),
        };
        let data = node(
            K::Round(vec![sym(",", pat.span), convert_quote(pat)]),
            pat.span,
        );
        let params = match guard {
            Some(g) => node(
                K::Round(vec![sym("?", head.span), data, convert_quote(g)]),
                head.span,
            ),
            None => data,
        };
        node(
            K::Round(vec![
                sym("->", c.span),
                params,
                node(K::Discard("_".into()), c.span),
                convert_quote(body),
            ]),
            c.span,
        )
    };
    let chain = |cs: &[Datum], span: Span| -> Datum {
        if let [one] = cs {
            return clause(one);
        }
        let mut parts = vec![sym("|", span)];
        parts.extend(cs.iter().map(clause));
        node(K::Round(parts), span)
    };
    let kind = match &d.kind {
        K::Curly(parts) if !parts.is_empty() && parts.iter().all(is_clause) => {
            return chain(parts, span);
        }
        K::Curly(parts)
            if parts
                .first()
                .is_some_and(|p| round(p, ":").is_some() || symbol(p) == Some("..")) =>
        {
            // A structural record.
            let mut out = vec![sym(",", span)];
            for p in parts {
                match round(p, ":") {
                    Some([l, v]) => out.push(node(
                        K::Round(vec![sym("=", p.span), l.clone(), convert_quote(v)]),
                        p.span,
                    )),
                    _ => out.push(convert_quote(p)),
                }
            }
            K::Round(out)
        }
        K::Round(parts) => match (parts.first().and_then(symbol), &parts[..]) {
            (Some("->"), [arrow, params, ret, body]) if matches!(params.kind, K::Round(_)) => {
                // A function: data-form parameters; the environment
                // parameters become the service pattern; the `^` and `!`
                // parts go (the needs are inferred).
                let K::Round(ps) = &params.kind else {
                    unreachable!()
                };
                let mut items = vec![sym(",", params.span)];
                let mut services = Vec::new();
                for p in ps {
                    match round(p, "!") {
                        Some([x, n]) => services.push(node(
                            K::Round(vec![sym(":", p.span), x.clone(), convert_quote(n)]),
                            p.span,
                        )),
                        _ => items.push(convert_quote(p)),
                    }
                }
                let (value, _, _) = split_result(ret);
                let mut result = convert_quote(value);
                if !services.is_empty() {
                    let mut group = vec![sym(",", ret.span)];
                    group.extend(services);
                    result = node(
                        K::Round(vec![
                            sym("<-", ret.span),
                            result,
                            node(K::Round(group), ret.span),
                        ]),
                        ret.span,
                    );
                }
                K::Round(vec![
                    arrow.clone(),
                    node(K::Round(items), params.span),
                    result,
                    convert_quote(body),
                ])
            }
            (Some("->"), [arrow, params, ret]) if matches!(&params.kind, K::Round(ps) if ps.first().and_then(symbol).is_none()) =>
            {
                // A function type: a data type with no labels, and the needs.
                let K::Round(ps) = &params.kind else {
                    unreachable!()
                };
                let mut items = vec![sym(",", params.span)];
                for p in ps {
                    match round(p, ":") {
                        Some([_, t]) => items.push(convert_quote(t)),
                        _ => items.push(convert_quote(p)),
                    }
                }
                let (value, error, env) = split_result(ret);
                let mut needs = Vec::new();
                if let Some(e) = error
                    && !matches!(&e.kind, K::UpName(s) if s == "Never")
                {
                    needs.push(node(
                        K::Round(vec![
                            node(K::UpName("Fail".into()), e.span),
                            convert_quote(e),
                        ]),
                        e.span,
                    ));
                }
                if let Some(v) = env {
                    match round(v, "|") {
                        Some(ms) => needs.extend(ms.iter().map(convert_quote)),
                        None => needs.push(convert_quote(v)),
                    }
                }
                let mut result = convert_quote(value);
                if !needs.is_empty() {
                    let mut group = vec![sym(",", ret.span)];
                    group.extend(needs);
                    result = node(
                        K::Round(vec![
                            sym("<-", ret.span),
                            result,
                            node(K::Round(group), ret.span),
                        ]),
                        ret.span,
                    );
                }
                K::Round(vec![
                    arrow.clone(),
                    node(K::Round(items), params.span),
                    result,
                ])
            }
            (Some("|" | "."), [_, subject, block]) if matches!(&block.kind, K::Curly(cs) if !cs.is_empty() && cs.iter().all(is_clause)) =>
            {
                let K::Curly(cs) = &block.kind else {
                    unreachable!()
                };
                K::Round(vec![
                    sym("|>", span),
                    convert_quote(subject),
                    chain(cs, block.span),
                ])
            }
            (Some("|"), [_, lhs, rhs]) if !matches!(lhs.kind, K::UpName(_)) => K::Round(vec![
                sym("|>", span),
                convert_quote(lhs),
                convert_quote(rhs),
            ]),
            _ => K::Round(all(parts)),
        },
        K::Square(parts) => K::Square(all(parts)),
        K::Curly(parts) => K::Curly(all(parts)),
        K::Bits(parts) => K::Bits(all(parts)),
        K::Quote(x) => K::Quote(Box::new(convert_quote(x))),
        K::Unquote(x) => K::Unquote(x.clone()),
        K::Splice(x) => K::Splice(x.clone()),
        other => other.clone(),
    };
    node(kind, span)
}

/// The parts of the result of an older function or function type: `R`,
/// `(^ R E)`, `(! R V)`, `(! (^ R E) V)`.
fn split_result(ret: &Datum) -> (&Datum, Option<&Datum>, Option<&Datum>) {
    let (ret, env) = match round(ret, "!") {
        Some([r, v]) => (r, Some(v)),
        _ => (ret, None),
    };
    match round(ret, "^") {
        Some([r, e]) => (r, Some(e), env),
        _ => (ret, None, env),
    }
}

/// The reverse of [`convert_quote`]: a quoted datum in the rows of §6.8 is
/// rewritten to the older rows, which the macro engine of this stage reads
/// (the transition front keeps the current tree, so the syntax that a macro
/// sees and gives is in the older rows).
pub fn unconvert_quote(d: &Datum) -> Datum {
    let span = d.span;
    let all = |xs: &[Datum]| xs.iter().map(unconvert_quote).collect::<Vec<_>>();
    // `(-> (, p) _ e)` or `(-> (? (, p) g) _ e)`: a branch of one position.
    fn branch(c: &Datum) -> Option<(&Datum, Option<&Datum>, &Datum)> {
        let [params, ret, body] = round(c, "->")? else {
            return None;
        };
        if !matches!(ret.kind, K::Discard(_)) {
            return None;
        }
        let (data, guard) = match round(params, "?") {
            Some([data, g]) => (data, Some(g)),
            _ => (params, None),
        };
        match round(data, ",") {
            Some([p]) => Some((p, guard, body)),
            _ => None,
        }
    }
    let clause = |c: &Datum| -> Datum {
        let (p, guard, body) = branch(c).unwrap();
        let head = match guard {
            Some(g) => node(
                K::Round(vec![
                    sym("?", p.span),
                    unconvert_quote(p),
                    unconvert_quote(g),
                ]),
                p.span,
            ),
            None => unconvert_quote(p),
        };
        node(
            K::Round(vec![sym("->", c.span), head, unconvert_quote(body)]),
            c.span,
        )
    };
    // The result of a function or a function type: `(<- R (, needs…))`.
    let needs = |ret: &Datum| -> (Datum, Vec<Datum>) {
        match round(ret, "<-") {
            Some([r, group]) => match round(group, ",") {
                Some(items) => (unconvert_quote(r), items.to_vec()),
                None => (unconvert_quote(r), vec![group.clone()]),
            },
            _ => (unconvert_quote(ret), Vec::new()),
        }
    };
    let kind = match &d.kind {
        K::Round(parts) => match (parts.first().and_then(symbol), &parts[..]) {
            (Some("->"), [arrow, params, ret, body]) if round(params, ",").is_some() => {
                let mut ps = all(round(params, ",").unwrap());
                let (result, services) = needs(ret);
                for s in services {
                    match round(&s, ":") {
                        Some([x, n]) => ps.push(node(
                            K::Round(vec![sym("!", s.span), x.clone(), unconvert_quote(n)]),
                            s.span,
                        )),
                        _ => ps.push(unconvert_quote(&s)),
                    }
                }
                K::Round(vec![
                    arrow.clone(),
                    node(K::Round(ps), params.span),
                    result,
                    unconvert_quote(body),
                ])
            }
            (Some("->"), [arrow, params, ret]) if round(params, ",").is_some() => {
                let ps = all(round(params, ",").unwrap());
                let (mut result, items) = needs(ret);
                let mut env = Vec::new();
                let mut error = None;
                for item in items {
                    match &item.kind {
                        K::Round(xs)
                            if xs.len() == 2
                                && matches!(&xs[0].kind, K::UpName(s) if s == "Fail") =>
                        {
                            error = Some(unconvert_quote(&xs[1]));
                        }
                        _ => env.push(unconvert_quote(&item)),
                    }
                }
                if let Some(e) = error {
                    result = node(K::Round(vec![sym("^", ret.span), result, e]), ret.span);
                }
                if !env.is_empty() {
                    let v = if env.len() == 1 {
                        env.pop().unwrap()
                    } else {
                        let mut parts = vec![sym("|", ret.span)];
                        parts.extend(env);
                        node(K::Round(parts), ret.span)
                    };
                    result = node(K::Round(vec![sym("!", ret.span), result, v]), ret.span);
                }
                K::Round(vec![arrow.clone(), node(K::Round(ps), params.span), result])
            }
            (Some("|>"), [_, subject, rhs]) => {
                let clauses: Option<Vec<&Datum>> = match round(rhs, "|") {
                    Some(cs) if !cs.is_empty() && cs.iter().all(|c| branch(c).is_some()) => {
                        Some(cs.iter().collect())
                    }
                    _ if branch(rhs).is_some() => Some(vec![rhs]),
                    _ => None,
                };
                match clauses {
                    Some(cs) => K::Round(vec![
                        sym("|", span),
                        unconvert_quote(subject),
                        node(K::Curly(cs.into_iter().map(clause).collect()), rhs.span),
                    ]),
                    None => K::Round(vec![
                        sym("|", span),
                        unconvert_quote(subject),
                        unconvert_quote(rhs),
                    ]),
                }
            }
            (Some("|"), [_, cs @ ..]) if cs.len() > 1 && cs.iter().all(|c| branch(c).is_some()) => {
                K::Curly(cs.iter().map(clause).collect())
            }
            (Some(","), [_, items @ ..])
                if items
                    .first()
                    .is_some_and(|p| round(p, "=").is_some() || symbol(p) == Some("..")) =>
            {
                let mut out = Vec::new();
                for p in items {
                    match round(p, "=") {
                        Some([l, v]) => out.push(node(
                            K::Round(vec![sym(":", p.span), l.clone(), unconvert_quote(v)]),
                            p.span,
                        )),
                        _ => out.push(unconvert_quote(p)),
                    }
                }
                K::Curly(out)
            }
            _ => K::Round(all(parts)),
        },
        K::Square(parts) => K::Square(all(parts)),
        K::Curly(parts) => K::Curly(all(parts)),
        K::Bits(parts) => K::Bits(all(parts)),
        K::Quote(x) => K::Quote(Box::new(unconvert_quote(x))),
        K::Unquote(x) => K::Unquote(x.clone()),
        K::Splice(x) => K::Splice(x.clone()),
        other => other.clone(),
    };
    node(kind, span)
}
