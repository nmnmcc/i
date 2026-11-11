//! Span-based first-surface migration. Source between edits stays untouched.
use ilang_syntax::{
    ast::*, lexer::lex, parser::parse_module_legacy, span::Span, token::TokenKind as T,
};
use serde_json::{Value, json};
use std::io::{self, BufRead};

struct Migrator<'a> {
    src: &'a str,
    edits: Vec<(usize, usize, &'static str)>,
    counts: [usize; 4],
    empty: Vec<String>,
    record_shadowed: bool,
}
impl<'a> Migrator<'a> {
    fn edit(&mut self, span: Span, text: &'static str) {
        self.edits
            .push((span.start as usize, span.end as usize, text));
    }
    fn mark_empty(&mut self, span: Span, kind: &str) {
        self.empty.push(format!("{kind} at byte {}", span.start));
    }
    fn record_delimiters(&mut self, span: Span) {
        let start = span.start as usize;
        let end = span.end as usize;
        if self.src.as_bytes()[start] == b'{' {
            self.edit(Span::new(span.start, span.start + 1), "(");
            self.edit(Span::new(span.end - 1, span.end), ")");
        }
        assert!(end > start, "record span must not be empty");
    }
    fn error(&mut self, value: &TypeExpr, err: &TypeExpr) {
        let start = value.span.end as usize;
        let end = err.span.start as usize;
        if start <= end
            && let Some(tok) = lex(&self.src[start..end])
                .tokens
                .into_iter()
                .find(|t| t.kind == T::Question)
        {
            self.edit(
                Span::new(start as u32 + tok.span.start, start as u32 + tok.span.end),
                "^",
            );
            self.counts[2] += 1;
        }
    }
    fn ty(&mut self, t: &TypeExpr) {
        match &t.kind {
            TypeKind::Record { base, fields, .. } => {
                self.record_delimiters(t.span);
                if let Some(base) = base {
                    self.ty(base);
                }
                for (_, field) in fields {
                    self.ty(field);
                }
            }
            TypeKind::Tuple(xs) => {
                if xs.is_empty() {
                    self.mark_empty(t.span, "empty tuple type");
                }
                for x in xs {
                    self.ty(x);
                }
            }
            TypeKind::Named { name, args, .. }
                if name.name == "Record" && self.src.as_bytes()[t.span.start as usize] == b'{' =>
            {
                if self.record_shadowed {
                    return;
                }
                let source = &self.src[t.span.start as usize..t.span.end as usize];
                let dots = lex(source)
                    .tokens
                    .into_iter()
                    .find(|tok| tok.kind == T::DotDot)
                    .expect("record type has dots");
                self.edit(Span::new(t.span.start, t.span.start + 1), "");
                self.edit(
                    Span::new(t.span.start + dots.span.start, t.span.start + dots.span.end),
                    "Record",
                );
                self.edit(Span::new(t.span.end - 1, t.span.end), "");
                for x in args {
                    self.ty(x);
                }
            }
            TypeKind::Named { args, .. }
            | TypeKind::Call { args, .. }
            | TypeKind::Union(args)
            | TypeKind::Inter(args)
            | TypeKind::Diff(args) => {
                for x in args {
                    self.ty(x);
                }
            }
            TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => {
                for p in params {
                    self.ty(&p.ty);
                }
                self.ty(ret);
                if let Some(e) = err {
                    self.error(ret, e);
                    self.ty(e);
                }
                if let Some(e) = env {
                    self.ty(e);
                }
            }
            _ => {}
        }
    }
    fn pat(&mut self, p: &Pattern) {
        match &p.kind {
            PatternKind::Typed { ty, .. } => self.ty(ty),
            PatternKind::Named { pattern, .. } => self.pat(pattern),
            PatternKind::Constructor { args, .. } => {
                for p in args {
                    self.pat(p)
                }
            }
            PatternKind::Tuple(xs) => {
                if xs.is_empty() {
                    self.mark_empty(p.span, "empty tuple pattern");
                }
                for x in xs {
                    self.pat(x)
                }
            }
            PatternKind::List { items, .. } => {
                for x in items {
                    self.pat(x)
                }
            }
            PatternKind::BitArray(xs) => {
                for x in xs {
                    self.pat(&x.value);
                    if let Some(f) = &x.format
                        && let Some(a) = &f.arg
                    {
                        self.expr(a)
                    }
                }
            }
            _ => {}
        }
    }
    fn block(&mut self, b: &Block) {
        for s in &b.stmts {
            match s {
                Stmt::Bind(b) => self.binding(b),
                Stmt::Env { ty, value, .. } => {
                    self.ty(ty);
                    self.expr(value)
                }
                Stmt::Expr(e) => self.expr(e),
            }
        }
    }
    fn binding(&mut self, b: &Binding) {
        self.pat(&b.pattern);
        self.expr(&b.value)
    }
    fn clauses(&mut self, cs: &[Clause]) {
        for c in cs {
            for p in &c.patterns {
                self.pat(p)
            }
            if let Some(e) = &c.guard {
                self.expr(e)
            }
            self.expr(&c.body)
        }
    }
    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Record { base, fields } => {
                self.record_delimiters(e.span);
                if let Some(base) = base {
                    self.expr(base);
                }
                for (_, field) in fields {
                    self.expr(field);
                }
            }
            ExprKind::Tuple(xs) => {
                if xs.is_empty() {
                    self.mark_empty(e.span, "empty tuple expression");
                }
                for x in xs {
                    self.expr(x)
                }
            }
            ExprKind::List { items, rest } => {
                for x in items {
                    self.expr(x)
                }
                if let Some(x) = rest {
                    self.expr(x)
                }
            }
            ExprKind::BitArray(xs) => {
                for x in xs {
                    self.expr(&x.value);
                    if let Some(f) = &x.format
                        && let Some(a) = &f.arg
                    {
                        self.expr(a)
                    }
                }
            }
            ExprKind::Block(b) => self.block(b),
            ExprKind::Fn(f) => {
                for p in &f.params {
                    match p {
                        Param::Pattern(p) => self.pat(p),
                        Param::Env { ty, .. } => self.ty(ty),
                    }
                }
                if let Some(r) = &f.ret {
                    self.ty(&r.value);
                    if let Some(err) = &r.err {
                        self.error(&r.value, err);
                        self.ty(err)
                    }
                    if let Some(env) = &r.env {
                        self.ty(env)
                    }
                }
                self.block(&f.body)
            }
            ExprKind::Case { subject, clauses } => {
                self.expr(subject);
                self.clauses(clauses);
            }
            ExprKind::Clauses(cs) => self.clauses(cs),
            ExprKind::Call { callee, args } => {
                self.expr(callee);
                for a in args {
                    self.expr(&a.value)
                }
            }
            ExprKind::Field { target, .. } | ExprKind::TupleIndex { target, .. } => {
                self.expr(target)
            }
            ExprKind::RecordUpdate {
                constructor,
                base,
                fields,
            } => {
                self.expr(constructor);
                self.expr(base);
                for (_, e) in fields {
                    self.expr(e)
                }
            }
            ExprKind::Unary { expr, .. } => self.expr(expr),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            _ => {}
        }
    }
}
fn migrate(src: &str) -> Value {
    let (legacy, diags) = parse_module_legacy(src);
    let m = if diags.is_empty() {
        legacy
    } else {
        let (current, current_diags) = ilang_syntax::parse_module(src);
        if !current_diags.is_empty() {
            return json!({"diagnostics":diags.iter().map(|d|format!("{}: {}",d.code.as_str(),d.message)).collect::<Vec<_>>()});
        }
        current
    };
    let mut g = Migrator {
        src,
        edits: vec![],
        counts: [0; 4],
        empty: vec![],
        // Keep the unambiguous legacy form if a local Record could capture it.
        record_shadowed: lex(src)
            .tokens
            .iter()
            .any(|t| matches!(&t.kind, T::UpName(n) if n == "Record")),
    };
    let mut mains = vec![];
    for item in &m.items {
        for a in &item.attributes {
            if a.module.is_none() && a.name.name == "main" {
                let name = if let ItemKind::Binding(b) = &item.kind {
                    if let PatternKind::Var(n) = &b.pattern.kind {
                        n.as_str()
                    } else {
                        "<invalid>"
                    }
                } else {
                    "<invalid>"
                };
                mains.push(name.to_string());
                g.edit(a.span, "");
                g.counts[3] += 1;
            }
        }
        match &item.kind {
            ItemKind::Binding(b) => g.binding(b),
            ItemKind::Declaration { ty, .. } => g.ty(ty),
            ItemKind::TypeDef(t) => {
                for x in &t.head {
                    g.ty(x)
                }
                match &t.body {
                    TypeBody::Alias(x) => g.ty(x),
                    TypeBody::Custom(cs) => {
                        for c in cs {
                            for f in &c.fields {
                                g.ty(&f.ty)
                            }
                        }
                    }
                }
            }
            ItemKind::MacroItem { args, .. } => {
                for a in args {
                    g.expr(&a.value)
                }
            }
        }
    }
    g.edits.sort_by_key(|&(start, end, _)| (start, end));
    let mut out = src.to_string();
    for (start, end, repl) in g.edits.iter().rev() {
        out.replace_range(*start..*end, repl)
    }
    if g.empty.is_empty() {
        let (after, after_diags) = ilang_syntax::parse_module(&out);
        if !after_diags.is_empty() {
            return json!({"diagnostics": after_diags.iter().map(|d| format!("migrated source: {}: {}", d.code.as_str(), d.message)).collect::<Vec<_>>()});
        }
        let mut before = m.clone();
        for item in &mut before.items {
            item.attributes
                .retain(|a| a.module.is_some() || a.name.name != "main");
        }
        if ilang_syntax::shape::canon(&before) != ilang_syntax::shape::canon(&after) {
            return json!({"diagnostics": ["migrated source changes the syntax tree"]});
        }
    }
    let names: Vec<_> = lex(&out)
        .tokens
        .into_iter()
        .filter_map(|t| {
            if let T::Name(n) = t.kind {
                Some(json!({"name":n,"start":t.span.start,"end":t.span.end}))
            } else {
                None
            }
        })
        .collect();
    json!({"source":out,"mains":mains,"names":names,"counts":g.counts,"empty":g.empty,"diagnostics":[]})
}
fn main() {
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let src: String = serde_json::from_str(&line).unwrap();
        println!("{}", migrate(&src));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_surface_keeps_records() {
        let source = "result = (name: 1)";
        let result = migrate(source);
        assert_eq!(result["diagnostics"], json!([]));
        assert_eq!(result["empty"], json!([]));
        assert_eq!(result["source"], source);
    }

    #[test]
    fn shadowed_record_keeps_uncapturable_legacy_type() {
        let source = "Record = Int\nT = {..}";
        let result = migrate(source);
        assert_eq!(result["source"], source);
        assert_eq!(result["diagnostics"], json!([]));
    }

    #[test]
    fn preserves_comments_and_tree_and_is_idempotent() {
        for source in [
            "result = 1 + x |> { 0 -> 1 _ -> 2 }",
            "result = x // a comment\n |> { 0 -> 1 _ -> 2 }",
            "result = (x) -> Int ? String { x }",
            "result = x |> f",
            "T = {..}",
            "result = { name: 1 }",
        ] {
            let result = migrate(source);
            assert_eq!(result["diagnostics"], json!([]), "{result}");
            let after = result["source"].as_str().unwrap();
            let (before_tree, _) = parse_module_legacy(source);
            let (after_tree, errors) = ilang_syntax::parse_module(after);
            assert!(errors.is_empty(), "{after}: {errors:?}");
            assert_eq!(
                ilang_syntax::shape::canon(&before_tree),
                ilang_syntax::shape::canon(&after_tree),
                "{after}"
            );
            assert_eq!(migrate(after)["source"], after);
            if source.contains("// a comment") {
                assert!(after.contains("// a comment"));
            }
        }
    }
}
