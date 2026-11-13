//! A canonical printer from the AST back to i source. It puts in `{ }` where
//! the meaning needs them, so printing and then parsing gives the same tree.
//! The explicit mode also puts every operator inside another one in `{ }`.
//!
//! A tree that does not come from the parser (from
//! another surface or a macro) may have no groups; the printer adds a group
//! only where the parser would read the text without it in another way.
//! [`unprintable`] finds the trees that have no text with the same meaning.

use crate::ast::*;
use crate::span::Span;

pub fn print_module(module: &Module) -> String {
    Printer::new(false).module(module)
}

/// Like `print_module`, with every nested operator in `{ }`, to show grouping.
pub fn print_module_explicit(module: &Module) -> String {
    Printer::new(true).module(module)
}

struct Printer {
    out: String,
    indent: usize,
    explicit: bool,
    /// Set while a clause pattern is printed that `?` or `|` comes after,
    /// which a function type at its end would take (§7.1).
    fn_next: bool,
}

/// The binding strength of an expression: binary levels 1 to 8 (§6.9), 9 for
/// unary operators, and 10 for the rest.
pub(crate) fn level(e: &Expr) -> u8 {
    match &e.kind {
        ExprKind::Binary { op, .. } => op.level(),
        ExprKind::Unary { .. } => 9,
        _ => 10,
    }
}

/// An expression with no block, function, or `case` inside it.
fn is_simple(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Quote(_)
        | ExprKind::Block(_)
        | ExprKind::Fn(_)
        | ExprKind::Case { .. }
        | ExprKind::Clauses(_) => false,
        ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::String(_) | ExprKind::Name { .. } => true,
        ExprKind::Tuple(items) => items.iter().all(is_simple),
        ExprKind::Record { base, fields } => {
            base.as_deref().is_none_or(is_simple) && fields.iter().all(|(_, e)| is_simple(e))
        }
        ExprKind::List { items, rest } => {
            items.iter().all(is_simple) && rest.as_deref().is_none_or(is_simple)
        }
        ExprKind::BitArray(segs) => segs.iter().all(|s| {
            is_simple(&s.value)
                && s.format
                    .as_ref()
                    .is_none_or(|f| f.arg.as_deref().is_none_or(is_simple))
        }),
        ExprKind::Call { callee, args } => {
            is_simple(callee) && args.iter().all(|a| is_simple(&a.value))
        }
        ExprKind::Field { target, .. } | ExprKind::TupleIndex { target, .. } => is_simple(target),
        ExprKind::RecordUpdate { base, fields, .. } => {
            is_simple(base) && fields.iter().all(|(_, e)| is_simple(e))
        }
        ExprKind::Unary { expr, .. } => is_simple(expr),
        ExprKind::Binary { lhs, rhs, .. } => is_simple(lhs) && is_simple(rhs),
    }
}

/// A match `e |> { … }` (§6.12).
#[cfg(feature = "legacy")]
pub(crate) fn is_match(e: &Expr) -> bool {
    matches!(e.kind, ExprKind::Case { .. })
}

/// The function that `e` starts with, through operators, calls, `.f`, and
/// `.k`.
pub(crate) fn leading_fn(e: &Expr) -> Option<&Function> {
    match &e.kind {
        ExprKind::Fn(f) => Some(f),
        ExprKind::Binary { lhs: e, .. }
        | ExprKind::Call { callee: e, .. }
        | ExprKind::Field { target: e, .. }
        | ExprKind::TupleIndex { target: e, .. } => leading_fn(e),
        _ => None,
    }
}

/// Whether the first clause of a clause block is in a place where a clause
/// expression that starts with a function would be read in another way
/// (§6.1): a first pattern `n1 = … = ( … )`.
pub(crate) fn first_clause_needs_group(clauses: &[Clause]) -> bool {
    if clauses
        .first()
        .is_some_and(|c| c.guard.is_some() || c.patterns.len() != 1)
    {
        return false;
    }
    let Some(first) = clauses.first().and_then(|c| c.patterns.first()) else {
        return false;
    };
    match &first.kind {
        PatternKind::Named { .. } => {
            let mut p = first;
            while let PatternKind::Named { pattern, .. } = &p.kind {
                p = pattern;
            }
            matches!(p.kind, PatternKind::Tuple(_))
        }
        _ => false,
    }
}

/// The span of the first part of `module` that has no text in the normal
/// surface with the same meaning: a `{` that begins an expression whose
/// first statement starts with a function with a `->` part and only
/// patterns as parameters, which §6.1 reads as a clause block; and the first
/// clause expression of a clause block in a place of
/// [`first_clause_needs_group`] that starts with such a function; and a
/// non-final choice ending in a type annotation, whose type takes `|`.
pub fn unprintable(module: &Module) -> Option<Span> {
    let mut found = None;
    for item in &module.items {
        match &item.kind {
            ItemKind::Binding(b) => find_unprintable(&b.value, &mut found),
            ItemKind::MacroItem { args, .. } => args
                .iter()
                .for_each(|a| find_unprintable(&a.value, &mut found)),
            _ => {}
        }
        for a in &item.attributes {
            if let Some((args, _)) = &a.call {
                args.iter()
                    .for_each(|a| find_unprintable(&a.value, &mut found));
            }
        }
    }
    found
}

fn fn_with_tail(f: &Function) -> bool {
    f.ret.is_some() && f.params.iter().all(|p| matches!(p, Param::Pattern(_)))
}

fn find_unprintable(e: &Expr, found: &mut Option<Span>) {
    if found.is_some() {
        return;
    }
    let mut note = |span: Span| {
        if found.is_none() {
            *found = Some(span);
        }
    };
    match &e.kind {
        ExprKind::Block(b) => {
            if let Some(Stmt::Expr(first)) = b.stmts.first()
                && leading_fn(first).is_some_and(fn_with_tail)
            {
                note(b.span);
            }
        }
        ExprKind::Clauses(cs) | ExprKind::Case { clauses: cs, .. } => {
            for c in cs {
                for p in c.patterns.iter().take(c.patterns.len().saturating_sub(1)) {
                    let mut last = p;
                    while let PatternKind::Named { pattern, .. } = &last.kind {
                        last = pattern;
                    }
                    if matches!(last.kind, PatternKind::Typed { .. }) {
                        note(p.span);
                    }
                }
            }
            if first_clause_needs_group(cs) && leading_fn(&cs[0].body).is_some_and(fn_with_tail) {
                note(cs[0].body.span);
            }
        }
        _ => {}
    }
    walk_expr(e, &mut |x| find_unprintable(x, found));
}

/// Calls `f` on each expression directly inside `e`.
fn walk_expr(e: &Expr, f: &mut impl FnMut(&Expr)) {
    let block = |b: &Block, f: &mut dyn FnMut(&Expr)| {
        for s in &b.stmts {
            match s {
                Stmt::Bind(b) => f(&b.value),
                Stmt::Env { value, .. } => f(value),
                Stmt::Expr(e) => f(e),
            }
        }
    };
    let clauses = |cs: &[Clause], f: &mut dyn FnMut(&Expr)| {
        for c in cs {
            c.guard.iter().for_each(&mut *f);
            f(&c.body);
        }
    };
    match &e.kind {
        ExprKind::Quote(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::String(_)
        | ExprKind::Name { .. } => {}
        ExprKind::Tuple(items) => items.iter().for_each(f),
        ExprKind::Record { base, fields } => {
            base.iter().for_each(|e| f(e));
            fields.iter().for_each(|(_, e)| f(e));
        }
        ExprKind::List { items, rest } => items.iter().chain(rest.as_deref()).for_each(f),
        ExprKind::BitArray(segs) => {
            for s in segs {
                f(&s.value);
                if let Some(arg) = s.format.as_ref().and_then(|x| x.arg.as_deref()) {
                    f(arg);
                }
            }
        }
        ExprKind::Block(b) => block(b, f),
        ExprKind::Fn(func) => block(&func.body, f),
        ExprKind::Case {
            subject,
            clauses: cs,
            ..
        } => {
            f(subject);
            clauses(cs, f);
        }
        ExprKind::Clauses(cs) => clauses(cs, f),
        ExprKind::Call { callee, args } => {
            f(callee);
            args.iter().for_each(|a| f(&a.value));
        }
        ExprKind::Field { target, .. } | ExprKind::TupleIndex { target, .. } => f(target),
        ExprKind::RecordUpdate {
            constructor,
            base,
            fields,
        } => {
            f(constructor);
            f(base);
            fields.iter().for_each(|(_, x)| f(x));
        }
        ExprKind::Unary { expr, .. } => f(expr),
        ExprKind::Binary { lhs, rhs, .. } => {
            f(lhs);
            f(rhs);
        }
    }
}

pub(crate) fn type_prec(t: &TypeExpr) -> u8 {
    match t.kind {
        TypeKind::Union(_) => 1,
        TypeKind::Inter(_) => 2,
        TypeKind::Diff(_) => 3,
        _ => 4,
    }
}

fn float(x: f64) -> String {
    let s = format!("{x:?}");
    match s.split_once('e') {
        Some((m, e)) if !m.contains('.') => format!("{m}.0e{e}"),
        _ => s,
    }
}

pub use crate::syn3::record_label;

pub use crate::syn3::string_literal;

impl Printer {
    fn new(explicit: bool) -> Printer {
        Printer {
            out: String::new(),
            indent: 0,
            explicit,
            fn_next: false,
        }
    }

    fn w(&mut self, s: &str) {
        self.out.push_str(s);
    }

    fn nl(&mut self) {
        self.out.push('\n');
        for _ in 0..self.indent {
            self.out.push_str("  ");
        }
    }

    fn list<X>(&mut self, xs: &[X], mut f: impl FnMut(&mut Self, &X)) {
        for (i, x) in xs.iter().enumerate() {
            if i > 0 {
                self.w(", ");
            }
            f(self, x);
        }
    }

    fn module(mut self, m: &Module) -> String {
        for d in &m.doc {
            self.w(&format!("////{d}\n"));
        }
        if !m.imports.is_empty() || m.imports_span.is_some() {
            self.w("(");
            self.indent += 1;
            for item in &m.imports {
                self.nl();
                if let Some(alias) = &item.alias {
                    self.w(&format!("{} = ", alias.name));
                }
                if !item.names.is_empty() {
                    self.w("(");
                    self.list(&item.names, |p, n| p.w(&n.name));
                    self.w(") = ");
                }
                let path: Vec<&str> = item.path.iter().map(|p| p.name.as_str()).collect();
                self.w(&path.join("/"));
            }
            self.indent -= 1;
            self.w("\n)\n");
        }
        if !m.exports.is_empty() || m.exports_span.is_some() {
            self.w("(");
            self.list(&m.exports, |p, e| {
                p.w(&e.name.name);
                if e.constructors {
                    p.w("(..)");
                }
            });
            self.w(")\n");
        }
        for item in &m.items {
            if !self.out.is_empty() {
                self.w("\n");
            }
            self.item(item);
            self.w("\n");
        }
        self.out
    }

    fn item(&mut self, item: &Item) {
        for d in &item.doc {
            self.w(&format!("///{d}\n"));
        }
        for a in &item.attributes {
            self.w("@");
            if let Some(m) = &a.module {
                self.w(&format!("{}.", m.name));
            }
            self.w(&a.name.name);
            if let Some((args, _)) = &a.call {
                self.args(args);
            } else if !a.args.is_empty() {
                self.w("(");
                self.list(&a.args, |p, (s, _)| p.w(&string_literal(s)));
                self.w(")");
            }
            self.w("\n");
        }
        match &item.kind {
            ItemKind::TypeDef(def) => self.type_def(def),
            ItemKind::Binding(b) => self.binding(b),
            ItemKind::Declaration {
                name,
                type_params,
                ty,
            } => {
                self.w(&name.name);
                self.type_params(type_params);
                self.w(": ");
                self.ty(ty);
            }
            ItemKind::MacroItem { module, name, args } => {
                if let Some(m) = module {
                    self.w(&format!("{}.", m.name));
                }
                self.w(&name.name);
                self.args(args);
            }
        }
    }

    fn type_params(&mut self, names: &[Ident]) {
        if !names.is_empty() {
            self.w("(");
            self.list(names, |p, n| p.w(&n.name));
            self.w(")");
        }
    }

    fn args(&mut self, args: &[Arg]) {
        self.w("(");
        self.list(args, |p, a| {
            if let Some(l) = &a.label {
                p.w(&format!("{}: ", l.name));
            }
            p.expr(&a.value, 0);
        });
        self.w(")");
    }

    fn type_def(&mut self, def: &TypeDef) {
        self.w(&def.name.name);
        if !def.head.is_empty() {
            self.w("(");
            self.list(&def.head, |p, t| p.ty(t));
            self.w(")");
        } else if !def.params.is_empty() {
            self.w("(");
            self.list(&def.params, |p, n| p.w(&n.name));
            self.w(")");
        }
        let constructors = match &def.body {
            TypeBody::Alias(t) => {
                self.w(" = ");
                return self.ty(t);
            }
            TypeBody::Custom(cs) if cs.is_empty() => return self.w(" {}"),
            TypeBody::Custom(cs) => cs,
        };
        self.w(" {");
        self.indent += 1;
        for c in constructors {
            self.nl();
            self.w(&c.name.name);
            if !c.fields.is_empty() {
                self.w("(");
                self.list(&c.fields, |p, f| {
                    if let Some(l) = &f.label {
                        p.w(&format!("{}: ", l.name));
                    }
                    p.ty(&f.ty);
                });
                self.w(")");
            }
        }
        self.indent -= 1;
        self.nl();
        self.w("}");
    }

    fn binding(&mut self, b: &Binding) {
        match (&b.pattern.kind, b.type_params.is_empty()) {
            (PatternKind::Var(n), false) => {
                self.w(n);
                self.type_params(&b.type_params);
            }
            (PatternKind::Typed { binder, ty }, false) => {
                self.w(&binder.name);
                self.type_params(&b.type_params);
                self.w(": ");
                self.ty(ty);
            }
            _ => self.pattern(&b.pattern, false),
        }
        self.w(" = ");
        self.expr(&b.value, 0);
    }

    fn stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::Bind(b) => self.binding(b),
            Stmt::Env { ty, value, .. } => {
                self.w("! ");
                self.ty(ty);
                self.w(" = ");
                self.expr(value, 0);
            }
            Stmt::Expr(e) => self.expr(e, 0),
        }
    }

    fn block(&mut self, b: &Block) {
        if let [Stmt::Expr(e)] = &b.stmts[..]
            && is_simple(e)
        {
            self.w("{ ");
            self.expr(e, 0);
            return self.w(" }");
        }
        self.w("{");
        self.indent += 1;
        for s in &b.stmts {
            self.nl();
            self.stmt(s);
        }
        self.indent -= 1;
        self.nl();
        self.w("}");
    }

    // ---- Expressions ----

    /// Prints `e` where an expression of at least level `min` is needed. A
    /// match is an operation at level 1 (§6.9), so it needs a group where
    /// more than that is needed.
    fn expr(&mut self, e: &Expr, min: u8) {
        let lvl = level(e);
        if lvl < min
            || self.explicit && min > 0 && lvl < 10
            || matches!(e.kind, ExprKind::Case { .. }) && min > 1
        {
            self.w("{ ");
            self.expr_raw(e);
            self.w(" }");
        } else {
            self.expr_raw(e);
        }
    }

    fn expr_raw(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Quote(_) => panic!("quote has no normal-surface spelling"),
            ExprKind::Int(v) => self.w(&v.to_string()),
            ExprKind::Float(x) => self.w(&float(*x)),
            ExprKind::String(s) => self.w(&string_literal(s)),
            ExprKind::Name { module, name } => {
                if let Some(m) = module {
                    self.w(&format!("{}.", m.name));
                }
                self.w(&name.name);
            }
            ExprKind::Tuple(items) => {
                // `()` is the empty data (syntax three §7.1 row 7), which
                // lowers to the empty tuple.
                self.w("(");
                self.list(items, |p, e| p.expr(e, 0));
                self.w(")");
            }
            ExprKind::Record { base, fields } => {
                self.w("(");
                if let Some(base) = base {
                    self.w("..");
                    self.expr(base, 0);
                }
                for (i, (label, value)) in fields.iter().enumerate() {
                    if i > 0 || base.is_some() {
                        self.w(", ");
                    }
                    let name = label.name.strip_prefix('_').unwrap_or(&label.name);
                    if name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                        && label
                            .name
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                    {
                        self.w(&label.name);
                    } else {
                        self.w(&string_literal(&label.name));
                    }
                    self.w(": ");
                    self.expr(value, 0);
                }
                self.w(")");
            }
            ExprKind::List { items, rest } => {
                self.w("[");
                self.list(items, |p, e| p.expr(e, 0));
                if let Some(rest) = rest {
                    self.w(", ..");
                    self.expr(rest, 0);
                }
                self.w("]");
            }
            ExprKind::BitArray(segs) => {
                self.w("<<");
                self.list(segs, |p, s| {
                    p.expr(&s.value, 0);
                    p.format(&s.format);
                });
                self.w(">>");
            }
            ExprKind::Block(b) => self.block(b),
            ExprKind::Fn(f) => self.function(f),
            ExprKind::Case { subject, clauses } => {
                self.expr(subject, 1);
                self.w(" |> ");
                self.clauses(clauses);
            }
            ExprKind::Clauses(clauses) => self.clauses(clauses),
            ExprKind::Call { callee, args } => {
                self.expr(callee, 10);
                self.args(args);
            }
            ExprKind::Field { target, name } => {
                self.target(target);
                self.w(&format!(".{}", name.name));
            }
            ExprKind::TupleIndex { target, index, .. } => {
                self.target(target);
                self.w(&format!(".{index}"));
            }
            ExprKind::RecordUpdate {
                constructor,
                base,
                fields,
            } => {
                self.expr(constructor, 10);
                self.w("(..");
                self.expr(base, 0);
                for (label, value) in fields {
                    self.w(&format!(", {}: ", label.name));
                    self.expr(value, 0);
                }
                self.w(")");
            }
            ExprKind::Unary { op, expr } => {
                self.w(match op {
                    UnOp::Neg => "-",
                    UnOp::NegFloat => "-.",
                });
                self.expr(expr, 10);
            }
            ExprKind::Binary { op, lhs, rhs } => {
                self.expr(lhs, op.level());
                self.w(&format!(" {} ", op.text()));
                self.expr(rhs, op.level() + 1);
            }
        }
    }

    /// The expression before `.name` or `.k`. An int right before `.` would
    /// be read as a float, so a space comes between them.
    fn target(&mut self, e: &Expr) {
        self.expr(e, 10);
        if matches!(e.kind, ExprKind::Int(_)) {
            self.w(" ");
        }
    }

    fn format(&mut self, format: &Option<Format>) {
        if let Some(f) = format {
            self.w(&format!(":{}", f.name.name));
            if let Some(arg) = &f.arg {
                self.w("(");
                self.expr(arg, 0);
                self.w(")");
            }
        }
    }

    fn function(&mut self, f: &Function) {
        self.w("(");
        self.list(&f.params, |p, param| match param {
            Param::Pattern(pat) => p.pattern(pat, false),
            Param::Env { name, ty } => {
                p.w(&format!("{} ! ", name.name));
                p.ty(ty);
            }
        });
        self.w(")");
        if let Some(ret) = &f.ret {
            self.w(" -> ");
            self.arrow_tail(&ret.value, ret.err.as_ref(), ret.env.as_ref(), true);
        }
        self.w(" ");
        self.block(&f.body);
    }

    /// A clause block, lone or after `|>`.
    fn clauses(&mut self, clauses: &[Clause]) {
        self.clause_lines(clauses);
    }

    fn clause_lines(&mut self, clauses: &[Clause]) {
        if clauses.is_empty() {
            return self.w("{}");
        }
        let group_first = first_clause_needs_group(clauses);
        self.w("{");
        self.indent += 1;
        for (n, c) in clauses.iter().enumerate() {
            self.nl();
            for (i, p) in c.patterns.iter().enumerate() {
                if i > 0 {
                    self.w(" | ");
                }
                let guard = c.guard.is_some();
                self.fn_next = i + 1 < c.patterns.len() || guard;
                self.pattern(p, true);
                self.fn_next = false;
            }
            if let Some(g) = &c.guard {
                self.w(" ? ");
                self.expr(g, 0);
            }
            self.w(" -> ");
            // §6.1: a first clause expression that starts with a function
            // with no `->` part would be read as the `->` part and the body
            // of a function.
            if n == 0 && group_first && leading_fn(&c.body).is_some_and(|f| f.ret.is_none()) {
                self.w("{ ");
                self.expr(&c.body, 0);
                self.w(" }");
            } else {
                self.expr(&c.body, 0);
            }
        }
        self.indent -= 1;
        self.nl();
        self.w("}");
    }

    // ---- Patterns ----

    /// `clause` is set for the top of a `case` clause pattern, where `->`
    /// comes after it.
    fn pattern(&mut self, p: &Pattern, clause: bool) {
        match &p.kind {
            PatternKind::Quote { .. } => panic!("quote pattern has no normal-surface spelling"),
            PatternKind::Var(n) | PatternKind::Discard(n) => self.w(n),
            PatternKind::Typed { binder, ty } => {
                self.w(&format!("{}: ", binder.name));
                if clause && self.ends_with_tuple(ty) {
                    self.w("{ ");
                    self.ty(ty);
                    self.w(" }");
                } else if clause && self.fn_next {
                    // §7.1: a function type before `?` or `|` is put in `{ }`.
                    self.ty_in(ty, 1, false);
                } else {
                    self.ty(ty);
                }
            }
            PatternKind::Named { name, pattern } => {
                self.w(&format!("{} = ", name.name));
                self.pattern(pattern, clause);
            }
            PatternKind::Int { negative, value } => {
                self.w(if *negative { "-" } else { "" });
                self.w(&value.to_string());
            }
            PatternKind::Float { negative, value } => {
                self.w(if *negative { "-." } else { "" });
                self.w(&float(*value));
            }
            PatternKind::String(s) => self.w(&string_literal(s)),
            PatternKind::Prefix {
                prefix,
                suffix,
                rest,
            } => {
                if !prefix.is_empty() || suffix.is_empty() {
                    self.w(&format!("{} <> ", string_literal(prefix)));
                }
                self.w(&rest.name);
                if !suffix.is_empty() {
                    self.w(&format!(" <> {}", string_literal(suffix)));
                }
            }
            PatternKind::Constructor {
                module,
                name,
                args,
                spread,
            } => {
                if let Some(m) = module {
                    self.w(&format!("{}.", m.name));
                }
                self.w(&name.name);
                if !args.is_empty() || *spread {
                    self.w("(");
                    self.list(args, |p, a| p.pattern(a, false));
                    if *spread {
                        self.w(if args.is_empty() { ".." } else { ", .." });
                    }
                    self.w(")");
                }
            }
            PatternKind::Tuple(items) => {
                assert!(!items.is_empty(), "tuple patterns must have a part");
                self.w("(");
                self.list(items, |p, a| p.pattern(a, false));
                self.w(")");
            }
            PatternKind::List { items, rest } => {
                self.w("[");
                self.list(items, |p, a| p.pattern(a, false));
                if let Some(rest) = rest {
                    self.w(if items.is_empty() { ".." } else { ", .." });
                    if let Some(b) = rest {
                        self.w(&b.name);
                    }
                }
                self.w("]");
            }
            PatternKind::BitArray(segs) => {
                self.w("<<");
                self.list(segs, |p, s| {
                    p.pattern(&s.value, false);
                    p.format(&s.format);
                });
                self.w(">>");
            }
        }
    }

    // ---- Types ----

    fn ty(&mut self, t: &TypeExpr) {
        self.ty_in(t, 1, true);
    }

    /// Whether a member of a union, intersection, or difference whose members
    /// need strength `min` is put in `{ }` for its strength.
    fn braced(&self, t: &TypeExpr, min: u8) -> bool {
        let prec = type_prec(t);
        prec < min || self.explicit && min > 1 && prec < 4
    }

    /// Whether the printed type ends with a function type, which would take a
    /// `|`, `&`, `-`, `?`, or `!` after it (§3).
    fn ends_with_fn(&self, t: &TypeExpr) -> bool {
        match &t.kind {
            TypeKind::Fn { .. } => true,
            TypeKind::Union(v) | TypeKind::Inter(v) | TypeKind::Diff(v) => {
                let last = v.last().unwrap();
                !self.braced(last, type_prec(t) + 1) && self.ends_with_fn(last)
            }
            _ => false,
        }
    }

    /// Whether the printed type ends with a tuple or record type: both are
    /// written in `( )`, which becomes the parameters of a function type when
    /// `->` comes after it (§7.1).
    fn ends_with_tuple(&self, t: &TypeExpr) -> bool {
        match &t.kind {
            TypeKind::Tuple(_) | TypeKind::Record { .. } => true,
            TypeKind::Union(v) | TypeKind::Inter(v) | TypeKind::Diff(v) => {
                let last = v.last().unwrap();
                !self.braced(last, type_prec(t) + 1) && self.ends_with_tuple(last)
            }
            TypeKind::Fn { ret, err, env, .. } => {
                self.ends_with_tuple(env.as_deref().or(err.as_deref()).unwrap_or(ret))
            }
            _ => false,
        }
    }

    /// Prints `t` where a type of strength `min` is needed. `tail` is set when
    /// nothing that a function type could take comes after it.
    fn ty_in(&mut self, t: &TypeExpr, min: u8, tail: bool) {
        let brace = self.braced(t, min) || !tail && self.ends_with_fn(t);
        if brace {
            self.w("{ ");
        }
        self.ty_raw(t, tail || brace);
        if brace {
            self.w(" }");
        }
    }

    fn ty_raw(&mut self, t: &TypeExpr, tail: bool) {
        let (members, sep) = match &t.kind {
            TypeKind::Named { module, name, args } => {
                if let Some(m) = module {
                    self.w(&format!("{}.", m.name));
                }
                self.w(&name.name);
                if !args.is_empty() {
                    self.w("(");
                    self.list(args, |p, a| p.ty(a));
                    self.w(")");
                }
                return;
            }
            TypeKind::ValueOf { module, name } => {
                if let Some(m) = module {
                    self.w(&format!("{}.", m.name));
                }
                return self.w(&name.name);
            }
            TypeKind::Call { module, name, args } => {
                if let Some(m) = module {
                    self.w(&format!("{}.", m.name));
                }
                self.w(&name.name);
                self.w("(");
                self.list(args, |p, t| p.ty(t));
                return self.w(")");
            }
            TypeKind::Lit(v) => return self.w(&v.text()),
            TypeKind::Hole => return self.w("_"),
            TypeKind::Tuple(items) => {
                assert!(!items.is_empty(), "tuple types must have a part");
                self.w("(");
                self.list(items, |p, a| p.ty(a));
                return self.w(")");
            }
            TypeKind::Record {
                base,
                fields,
                removed,
            } => {
                self.w("(");
                if let Some(base) = base {
                    self.w("..");
                    self.ty(base);
                }
                for (i, (label, ty)) in fields.iter().enumerate() {
                    if i > 0 || base.is_some() {
                        self.w(", ");
                    }
                    self.w(&record_label(&label.name, label.parameter));
                    self.w(": ");
                    self.ty(ty);
                }
                for (i, label) in removed.iter().enumerate() {
                    if i > 0 || base.is_some() || !fields.is_empty() {
                        self.w(", ");
                    }
                    self.w("-");
                    self.w(&record_label(&label.name, label.parameter));
                }
                return self.w(")");
            }
            TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => {
                self.w("(");
                self.list(params, |p, param| {
                    if let Some(l) = &param.label {
                        p.w(&format!("{}: ", l.name));
                    }
                    p.ty(&param.ty);
                });
                self.w(") -> ");
                return self.arrow_tail(ret, err.as_deref(), env.as_deref(), tail);
            }
            TypeKind::Union(v) => (v, " | "),
            TypeKind::Inter(v) => (v, " & "),
            TypeKind::Diff(v) => (v, " - "),
        };
        let min = type_prec(t) + 1;
        for (i, m) in members.iter().enumerate() {
            if i > 0 {
                self.w(sep);
            }
            self.ty_in(m, min, tail && i + 1 == members.len());
        }
    }

    fn arrow_tail(
        &mut self,
        ret: &TypeExpr,
        err: Option<&TypeExpr>,
        env: Option<&TypeExpr>,
        tail: bool,
    ) {
        self.ty_in(ret, 1, tail && err.is_none() && env.is_none());
        if let Some(err) = err {
            self.w(" ^ ");
            self.ty_in(err, 1, tail && env.is_none());
        }
        if let Some(env) = env {
            self.w(" ! ");
            self.ty_in(env, 1, tail);
        }
    }
}
