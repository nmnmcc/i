//! A canonical printer from the AST back to i source. It puts in `{ }` where
//! the meaning needs them, so printing and then parsing gives the same tree.
//! The explicit mode also puts every operator inside another one in `{ }`.

use crate::ast::*;

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
        ExprKind::Block(_) | ExprKind::Fn(_) | ExprKind::Case { .. } => false,
        ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::String(_) | ExprKind::Name { .. } => true,
        ExprKind::Tuple(items) => items.iter().all(is_simple),
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

/// An expression with a function without `->` outside all brackets, which
/// would end too early after `case` (§1.7).
pub(crate) fn has_open_fn(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Fn(f) => f.ret.is_none(),
        ExprKind::Binary { lhs, rhs, .. } => has_open_fn(lhs) || has_open_fn(rhs),
        ExprKind::Unary { expr: e, .. }
        | ExprKind::Call { callee: e, .. }
        | ExprKind::Field { target: e, .. }
        | ExprKind::TupleIndex { target: e, .. } => has_open_fn(e),
        _ => false,
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

pub fn string_literal(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{c}' => out.push_str("\\f"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:04X}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl Printer {
    fn new(explicit: bool) -> Printer {
        Printer {
            out: String::new(),
            indent: 0,
            explicit,
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
        if !m.imports.is_empty() {
            self.w("import (");
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
        if !m.exports.is_empty() {
            self.w("export (");
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
            self.w(&format!("@{}", a.name.name));
            if !a.args.is_empty() {
                self.w("(");
                self.list(&a.args, |p, (s, _)| p.w(&string_literal(s)));
                self.w(")");
            }
            self.w("\n");
        }
        match &item.kind {
            ItemKind::TypeDef(def) => self.type_def(def),
            ItemKind::Binding(b) => self.binding(b),
            ItemKind::Declaration { name, ty } => {
                self.w(&format!("{}: ", name.name));
                self.ty(ty);
            }
        }
    }

    fn type_def(&mut self, def: &TypeDef) {
        self.w(&def.name.name);
        if !def.params.is_empty() {
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
        self.pattern(&b.pattern, false);
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

    /// Prints `e` where an expression of at least level `min` is needed.
    fn expr(&mut self, e: &Expr, min: u8) {
        let lvl = level(e);
        if lvl < min || self.explicit && min > 0 && lvl < 10 {
            self.w("{ ");
            self.expr_raw(e);
            self.w(" }");
        } else {
            self.expr_raw(e);
        }
    }

    fn expr_raw(&mut self, e: &Expr) {
        match &e.kind {
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
                self.w("(");
                self.list(items, |p, e| p.expr(e, 0));
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
            ExprKind::Case { subject, clauses } => self.case(subject, clauses),
            ExprKind::Call { callee, args } => {
                self.expr(callee, 10);
                self.w("(");
                self.list(args, |p, a| {
                    if let Some(l) = &a.label {
                        p.w(&format!("{}: ", l.name));
                    }
                    p.expr(&a.value, 0);
                });
                self.w(")");
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

    fn case(&mut self, subject: &Expr, clauses: &[Clause]) {
        self.w("case ");
        if has_open_fn(subject) {
            self.w("{ ");
            self.expr(subject, 0);
            self.w(" }");
        } else {
            self.expr(subject, 0);
        }
        if clauses.is_empty() {
            return self.w(" {}");
        }
        self.w(" {");
        self.indent += 1;
        for c in clauses {
            self.nl();
            for (i, p) in c.patterns.iter().enumerate() {
                if i > 0 {
                    self.w(" | ");
                }
                self.pattern(p, true);
            }
            if let Some(g) = &c.guard {
                self.w(" if ");
                self.expr(g, 0);
            }
            self.w(" -> ");
            self.expr(&c.body, 0);
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
            PatternKind::Var(n) | PatternKind::Discard(n) => self.w(n),
            PatternKind::Typed { binder, ty } => {
                self.w(&format!("{}: ", binder.name));
                if clause && self.ends_with_tuple(ty) {
                    self.w("{ ");
                    self.ty(ty);
                    self.w(" }");
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
            PatternKind::Prefix { prefix, rest } => {
                self.w(&format!("{} <> {}", string_literal(prefix), rest.name));
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

    /// Whether the printed type ends with a tuple type, which becomes a
    /// function type when `->` comes after it (§7.1).
    fn ends_with_tuple(&self, t: &TypeExpr) -> bool {
        match &t.kind {
            TypeKind::Tuple(_) => true,
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
            TypeKind::Var(n) => return self.w(n),
            TypeKind::Hole => return self.w("_"),
            TypeKind::Tuple(items) => {
                self.w("(");
                self.list(items, |p, a| p.ty(a));
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
            self.w(" ? ");
            self.ty_in(err, 1, tail && env.is_none());
        }
        if let Some(env) = env {
            self.w(" ! ");
            self.ty_in(env, 1, tail);
        }
    }
}
