//! The printer and formatter of syntax three. One layout engine serves both:
//! [`print_module`] writes a tree with no source, [`print_with_comments`]
//! puts the comments of a source back at the items, statements, branches and
//! data items they stood before or after, and [`format`] is the formatter.
//!
//! The text is laid out as in "A prettier printer" (Wadler): a group is on
//! one line when it fits in [`WIDTH`] columns. Groups `{ }` are put in where
//! the grammar needs them, so `parse(print(t)) == t`.
//!
//! Nested calls (owner rule): the call depth of an expression is the number
//! of nested function calls on the deepest path through call arguments (a
//! call of an `UpName` constructor and data literals do not count, field
//! access and pipes do not add depth, a function literal argument starts
//! again for its body). A call whose depth is greater than 2 is never on one
//! line: its arguments go one per line, indented, with the `)` on its own
//! line, as when a call does not fit the width; the rule applies again to each
//! argument. A block whose one statement is broken so is over several lines.

use super::ast::*;
use crate::Diagnostic;
use crate::lexer::{Comment, lex3};
use crate::print::string_literal;
use crate::span::Span;
use crate::token::TokenKind as T;

pub const WIDTH: usize = 80;
pub const TAB_WIDTH: usize = 4;

/// Prints a tree. Documentation comments come from the tree.
pub fn print_module(module: &Module) -> String {
    let mut p = Printer {
        src: None,
        comments: &[],
        next: 0,
    };
    let doc = p.module(module);
    render(&doc)
}

/// Prints a tree whose spans point into `source` (the text it was parsed
/// from, or, for a converted tree, the text of the older syntax), with the
/// comments of `source`.
pub fn print_with_comments(module: &Module, source: &str, comments: &[Comment]) -> String {
    let mut p = Printer {
        src: Some(source),
        comments,
        next: 0,
    };
    let doc = p.module(module);
    render(&doc)
}

/// Formats a syntax-three source file. Syntax errors leave it untouched.
pub fn format(source: &str) -> Result<String, Vec<Diagnostic>> {
    let (module, diags) = super::parse_module(source);
    if !diags.is_empty() {
        return Err(diags);
    }
    let lexed = lex3(source);
    Ok(print_with_comments(&module, source, &lexed.comments))
}

/// Prints one expression on as few lines as it needs.
pub fn print_expr(e: &Expr) -> String {
    let mut p = Printer {
        src: None,
        comments: &[],
        next: 0,
    };
    let doc = p.expr(e, LOWEST);
    render(&doc).trim_end().to_string()
}

/// Prints one type.
pub fn print_type(t: &Type) -> String {
    let mut p = Printer {
        src: None,
        comments: &[],
        next: 0,
    };
    let doc = p.ty(t, 0);
    render(&doc).trim_end().to_string()
}

/// Prints one pattern.
pub fn print_pattern(pattern: &Pattern) -> String {
    let mut p = Printer {
        src: None,
        comments: &[],
        next: 0,
    };
    let doc = p.pattern(pattern);
    render(&doc).trim_end().to_string()
}

// ---- Documents ----

#[derive(Clone, Debug)]
enum Doc {
    Text(String),
    /// A space on one line, or a line end.
    Line,
    /// Nothing on one line, or a line end.
    Soft,
    /// Always a line end; the groups around it are never on one line.
    Hard,
    /// Text only when the group around it is over several lines.
    IfBreak(&'static str),
    Nest(Box<Doc>),
    Group(Box<Doc>),
    Cat(Vec<Doc>),
}

fn t(s: impl Into<String>) -> Doc {
    Doc::Text(s.into())
}

fn cat(parts: Vec<Doc>) -> Doc {
    Doc::Cat(parts)
}

fn nest(d: Doc) -> Doc {
    Doc::Nest(Box::new(d))
}

fn group(d: Doc) -> Doc {
    Doc::Group(Box::new(d))
}

fn text_width(s: &str) -> usize {
    s.chars().count()
}

fn fits<'a>(
    mut rem: isize,
    rest: &[(usize, bool, &'a Doc)],
    first: (usize, bool, &'a Doc),
) -> bool {
    let mut stack: Vec<(usize, bool, &'a Doc)> = vec![first];
    let mut below = rest.len();
    loop {
        if rem < 0 {
            return false;
        }
        let (indent, flat, doc) = match stack.pop() {
            Some(x) => x,
            None => {
                if below == 0 {
                    return true;
                }
                below -= 1;
                rest[below]
            }
        };
        match doc {
            Doc::Text(s) => rem -= text_width(s) as isize,
            Doc::Line => {
                if flat {
                    rem -= 1;
                } else {
                    return true;
                }
            }
            Doc::Soft => {
                if !flat {
                    return true;
                }
            }
            Doc::Hard => return !flat,
            Doc::IfBreak(s) => {
                if !flat {
                    rem -= s.len() as isize;
                }
            }
            Doc::Nest(d) => stack.push((indent + 1, flat, d)),
            Doc::Group(d) => stack.push((indent, flat, d)),
            Doc::Cat(parts) => {
                for part in parts.iter().rev() {
                    stack.push((indent, flat, part));
                }
            }
        }
    }
}

fn render(doc: &Doc) -> String {
    render_in(doc, WIDTH)
}

fn render_in(doc: &Doc, width: usize) -> String {
    let mut out = String::new();
    let mut col = 0usize;
    // Indentation is written when the first text of a line is.
    let mut pending: Option<usize> = None;
    let mut stack: Vec<(usize, bool, &Doc)> = vec![(0, false, doc)];
    while let Some((indent, flat, doc)) = stack.pop() {
        match doc {
            Doc::Text(s) => {
                if let Some(n) = pending.take() {
                    for _ in 0..n {
                        out.push('\t');
                    }
                    col = n * TAB_WIDTH;
                }
                out.push_str(s);
                col += text_width(s);
            }
            Doc::Line | Doc::Soft if flat => {
                if matches!(doc, Doc::Line) {
                    out.push(' ');
                    col += 1;
                }
            }
            Doc::Line | Doc::Soft | Doc::Hard => {
                while out.ends_with(' ') {
                    out.pop();
                }
                out.push('\n');
                pending = Some(indent);
                col = indent * TAB_WIDTH;
            }
            Doc::IfBreak(s) => {
                if !flat {
                    out.push_str(s);
                    col += s.len();
                }
            }
            Doc::Nest(d) => stack.push((indent + 1, flat, d)),
            Doc::Group(d) => {
                let now = match pending {
                    Some(n) => n * TAB_WIDTH,
                    None => col,
                };
                let flat = flat || fits(width as isize - now as isize, &stack, (indent, true, d));
                stack.push((indent, flat, d));
            }
            Doc::Cat(parts) => {
                for part in parts.iter().rev() {
                    stack.push((indent, flat, part));
                }
            }
        }
    }
    while out.ends_with(['\n', ' ']) {
        out.pop();
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

// ---- The printer ----

/// The level of `<-`, the weakest.
pub(super) const LOWEST: i8 = -1;
pub(super) const PIPE: i8 = 0;
pub(super) const POSTFIX: i8 = 10;

pub(super) fn level(e: &Expr) -> i8 {
    match &e.kind {
        ExprKind::Provide { .. } => LOWEST,
        ExprKind::Pipe { .. } => PIPE,
        ExprKind::Binary { op, .. } => op.level() as i8,
        ExprKind::Unary { .. } => 9,
        _ => POSTFIX,
    }
}

/// The level of a type: 0 a union or a function type, 1 an intersection,
/// 2 a difference, 3 the rest.
fn type_level(ty: &Type) -> u8 {
    match &ty.kind {
        TypeKind::Union(_) | TypeKind::Fn(_) => 0,
        TypeKind::Inter(_) => 1,
        TypeKind::Diff(_) => 2,
        _ => 3,
    }
}

/// The greatest call depth that the formatter lays out on one line.
pub const MAX_CALL_DEPTH: usize = 2;

/// The call depth of an expression: the number of nested function calls on
/// the deepest path through call arguments. A call of an `UpName`
/// constructor and data do not count; field access, pipes and operators do
/// not add depth; a function literal or a block starts again at 0.
pub fn call_depth(e: &Expr) -> usize {
    let items = |items: &[DItem]| {
        items
            .iter()
            .map(|i| match i {
                DItem::Pos(e) | DItem::Label(_, e) | DItem::Spread(e) => call_depth(e),
            })
            .max()
            .unwrap_or(0)
    };
    match &e.kind {
        ExprKind::Call { callee, args } => {
            let constructor = matches!(&callee.kind, ExprKind::Name { name, .. }
                if name.name.starts_with(|c: char| c.is_ascii_uppercase()));
            items(args) + usize::from(!constructor)
        }
        ExprKind::Data(xs) => items(xs),
        ExprKind::List { items: xs, rest } => xs
            .iter()
            .chain(rest.as_deref())
            .map(call_depth)
            .max()
            .unwrap_or(0),
        ExprKind::Field { target, .. } | ExprKind::Index { target, .. } => call_depth(target),
        ExprKind::Unary { expr, .. } => call_depth(expr),
        ExprKind::Binary { lhs, rhs, .. } | ExprKind::Pipe { lhs, rhs } => {
            call_depth(lhs).max(call_depth(rhs))
        }
        ExprKind::Provide { target, services } => call_depth(target).max(call_depth(services)),
        _ => 0,
    }
}

fn float(x: f64) -> String {
    let s = format!("{x:?}");
    match s.split_once('e') {
        Some((m, e)) if !m.contains('.') => format!("{m}.0e{e}"),
        _ => s,
    }
}

fn qualified(module: &Option<Ident>, name: &Ident) -> String {
    match module {
        Some(m) => format!("{}.{}", m.name, name.name),
        None => name.name.clone(),
    }
}

struct Printer<'a> {
    src: Option<&'a str>,
    comments: &'a [Comment],
    next: usize,
}

/// A part of a bracketed list, with the comment that ends its line.
struct Part {
    doc: Doc,
    trailing: Option<Doc>,
}

impl Printer<'_> {
    // ---- Comments and source ----

    fn peek(&self) -> Option<&Comment> {
        self.comments.get(self.next)
    }

    /// Whether a comment waits before `pos`.
    fn comment_before(&self, pos: u32) -> bool {
        pos != 0 && self.peek().is_some_and(|c| c.span.start < pos)
    }

    /// Whether the source has an empty line before `pos`.
    fn blank_before(&self, pos: u32) -> bool {
        let Some(src) = self.src else { return false };
        let mut lines = 0;
        for b in src.as_bytes()[..(pos as usize).min(src.len())].iter().rev() {
            match b {
                b'\n' => lines += 1,
                b' ' | b'\t' | b'\r' => {}
                _ => break,
            }
        }
        lines >= 2
    }

    /// The comments before `pos`, each on its own line.
    fn leading(&mut self, pos: u32) -> Vec<Doc> {
        let mut out = Vec::new();
        if pos == 0 {
            return out;
        }
        while let Some(c) = self.peek() {
            if c.span.start >= pos {
                break;
            }
            if !out.is_empty() && self.blank_before(c.span.start) {
                out.push(Doc::Hard);
            }
            out.push(t(c.text.trim_end()));
            out.push(Doc::Hard);
            self.next += 1;
        }
        out
    }

    /// The comment on the line of `end`, after it.
    fn trailing(&mut self, end: u32) -> Option<Doc> {
        let src = self.src?;
        if end == 0 {
            return None;
        }
        let c = self.peek()?;
        if c.span.start < end || src[end as usize..c.span.start as usize].contains('\n') {
            return None;
        }
        let text = format!(" {}", c.text.trim_end());
        self.next += 1;
        Some(t(text))
    }

    /// The text of a literal as it is written in the source, when the span
    /// holds exactly that literal.
    fn written(&self, span: Sp, same: impl Fn(&T) -> bool) -> Option<String> {
        let src = self.src?;
        let slice = src.get(span.0.start as usize..span.0.end as usize)?;
        if slice.is_empty() || slice.contains('\n') {
            return None;
        }
        let lexed = lex3(slice);
        (lexed.diagnostics.is_empty() && lexed.tokens.len() == 2 && same(&lexed.tokens[0].kind))
            .then(|| slice.to_string())
    }

    // ---- Brackets ----

    /// `open part, part, … close`: on one line, or one part per line.
    fn delimited(&mut self, open: &str, close: &str, parts: Vec<Part>) -> Doc {
        if parts.is_empty() {
            return t(format!("{open}{close}"));
        }
        let mut inner = vec![Doc::Soft];
        let n = parts.len();
        let mut last_trailing = None;
        for (i, part) in parts.into_iter().enumerate() {
            inner.push(part.doc);
            if i + 1 < n {
                inner.push(t(","));
                match part.trailing {
                    Some(c) => {
                        inner.push(c);
                        inner.push(Doc::Hard);
                    }
                    None => inner.push(Doc::Line),
                }
            } else {
                last_trailing = part.trailing;
            }
        }
        let mut out = vec![t(open), nest(cat(inner)), Doc::IfBreak(",")];
        match last_trailing {
            Some(c) => {
                out.push(c);
                out.push(Doc::Hard);
            }
            None => out.push(Doc::Soft),
        }
        out.push(t(close));
        group(cat(out))
    }

    fn plain(docs: Vec<Doc>) -> Vec<Part> {
        docs.into_iter()
            .map(|doc| Part {
                doc,
                trailing: None,
            })
            .collect()
    }

    // ---- Module ----

    fn module(&mut self, m: &Module) -> Doc {
        let mut out = Vec::new();
        if self.src.is_none() {
            for d in &m.doc {
                out.push(t(format!("////{d}")));
                out.push(Doc::Hard);
            }
        }
        let mut first = true;
        let mut sep = |out: &mut Vec<Doc>, blank: bool| {
            if !first {
                out.push(Doc::Hard);
                if blank {
                    out.push(Doc::Hard);
                }
            }
            first = false;
        };
        if !m.imports.is_empty() {
            let pos = m.imports[0].span.0.start;
            let lead = self.leading(pos);
            sep(&mut out, false);
            out.extend(lead);
            let mut lines = Vec::new();
            for i in &m.imports {
                lines.push(Doc::Hard);
                lines.extend(self.leading(i.span.0.start));
                let mut s = String::new();
                if let Some(a) = &i.alias {
                    s.push_str(&format!("{} = ", a.name));
                }
                if !i.names.is_empty() {
                    let names: Vec<&str> = i.names.iter().map(|n| n.name.as_str()).collect();
                    s.push_str(&format!("({}) = ", names.join(", ")));
                }
                let path: Vec<&str> = i.path.iter().map(|n| n.name.as_str()).collect();
                s.push_str(&path.join("/"));
                lines.push(t(s));
                lines.extend(self.trailing(i.span.0.end));
            }
            out.push(cat(vec![t("("), nest(cat(lines)), Doc::Hard, t(")")]));
        }
        if !m.exports.is_empty() {
            let pos = m.exports[0].span.0.start;
            let lead = self.leading(pos);
            sep(&mut out, true);
            out.extend(lead);
            let mut lines = Vec::new();
            for e in &m.exports {
                lines.push(Doc::Hard);
                lines.extend(self.leading(e.span.0.start));
                lines.push(t(if e.constructors {
                    format!("{}(..)", e.name.name)
                } else {
                    e.name.name.clone()
                }));
                lines.extend(self.trailing(e.span.0.end));
            }
            out.push(cat(vec![t("("), nest(cat(lines)), Doc::Hard, t(")")]));
        }
        for (i, item) in m.items.iter().enumerate() {
            // An item with no span is one that a converter made beside the
            // next item that has one: the run is spaced as that item is.
            let anchored = item.span.0.end != 0;
            let after_run = i > 0 && m.items[i - 1].span.0.end == 0;
            let start = match m.items[i..].iter().find(|x| x.span.0.end != 0) {
                Some(next) => next.span.0.start,
                None => 0,
            };
            let gap = match self.peek() {
                Some(c) if start != 0 && c.span.start < start => c.span.start,
                _ => start,
            };
            let blank = !after_run && (self.src.is_none() || self.blank_before(gap));
            let lead = if anchored {
                self.leading(start)
            } else {
                Vec::new()
            };
            sep(&mut out, blank);
            out.extend(lead);
            out.push(self.item(item));
            if anchored {
                out.extend(self.trailing(item.span.0.end));
            }
        }
        let rest = self.leading(u32::MAX);
        if !rest.is_empty() {
            sep(&mut out, false);
            out.extend(rest);
        }
        cat(out)
    }

    fn item(&mut self, item: &Item) -> Doc {
        let mut out = Vec::new();
        if self.src.is_none() {
            for d in &item.doc {
                out.push(t(format!("///{d}")));
                out.push(Doc::Hard);
            }
        }
        for a in &item.attributes {
            out.extend(self.leading(a.span.0.start));
            let mut parts = vec![t(format!("@{}", qualified(&a.module, &a.name)))];
            if let Some(args) = &a.args {
                parts.push(self.data(args, false));
            }
            out.push(cat(parts));
            out.push(Doc::Hard);
        }
        match &item.kind {
            // An item with no span is one that a converter made beside
            // another; the comments stay with that other item.
            ItemKind::TypeDef(def) if item.span.0.end == 0 => out.push(self.type_def(def)),
            ItemKind::TypeDef(def) => {
                out.extend(self.leading(def.name.span.0.start));
                out.push(self.type_def(def));
            }
            ItemKind::Bind(b) => {
                out.extend(self.leading(b.span.0.start));
                out.push(self.binding(b));
            }
            ItemKind::Decl { name, ty } => {
                out.extend(self.leading(name.span.0.start));
                out.push(t(format!("{}: ", name.name)));
                out.push(self.ty(ty, 0));
            }
            ItemKind::Macro { module, name, args } => {
                let at = module.as_ref().unwrap_or(name).span.0.start;
                out.extend(self.leading(at));
                out.push(t(qualified(module, name)));
                out.push(self.data(args, true));
            }
        }
        cat(out)
    }

    fn type_def(&mut self, def: &TypeDef) -> Doc {
        let mut out = vec![t(def.name.name.clone())];
        if !def.head.is_empty() {
            let parts = def.head.iter().map(|h| self.ty(h, 0)).collect();
            out.push(self.delimited("(", ")", Self::plain(parts)));
        }
        out.push(t(" = "));
        match &def.rhs {
            TypeRhs::Opaque => out.push(t("Opaque")),
            TypeRhs::Data(items) => out.push(self.data_type(items)),
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
                let doc = self.ty(ty, 0);
                if grouped {
                    out.push(cat(vec![t("{ "), doc, t(" }")]));
                } else {
                    out.push(doc);
                }
            }
        }
        cat(out)
    }

    // ---- Statements ----

    fn binding(&mut self, b: &Binding) -> Doc {
        let pattern = self.pattern(&b.pattern);
        match &b.value.kind {
            ExprKind::Chain(fs) if fs.len() > 1 => {
                let branches = self.branches(fs);
                group(cat(vec![pattern, t(" ="), nest(cat(vec![Doc::Line, branches]))]))
            }
            _ => cat(vec![pattern, t(" = "), self.expr(&b.value, LOWEST)]),
        }
    }

    fn stmt_span(s: &Stmt) -> Span {
        match s {
            Stmt::Bind(b) => b.span.0,
            Stmt::Expr(e) => e.span.0,
        }
    }

    fn stmt(&mut self, s: &Stmt) -> Doc {
        match s {
            Stmt::Bind(b) => self.binding(b),
            Stmt::Expr(e) => self.expr(e, LOWEST),
        }
    }

    fn block(&mut self, b: &Block) -> Doc {
        let end = b.span.0.end;
        if let [Stmt::Expr(e)] = b.stmts.as_slice()
            && !self.comment_before(end)
        {
            let inner = self.expr(e, LOWEST);
            return group(cat(vec![
                t("{"),
                nest(cat(vec![Doc::Line, inner])),
                Doc::Line,
                t("}"),
            ]));
        }
        let mut lines = Vec::new();
        for (i, s) in b.stmts.iter().enumerate() {
            let span = Self::stmt_span(s);
            let gap = match self.peek() {
                Some(c) if span.start != 0 && c.span.start < span.start => c.span.start,
                _ => span.start,
            };
            lines.push(Doc::Hard);
            if i > 0 && self.blank_before(gap) {
                lines.push(Doc::Hard);
            }
            lines.extend(self.leading(span.start));
            lines.push(self.stmt(s));
            lines.extend(self.trailing(span.end));
        }
        if end != 0 {
            let rest = self.leading(end.saturating_sub(1));
            if !rest.is_empty() {
                lines.push(Doc::Hard);
                let mut rest = rest;
                rest.pop();
                lines.extend(rest);
            }
        }
        cat(vec![t("{"), nest(cat(lines)), Doc::Hard, t("}")])
    }

    // ---- Expressions ----

    /// `e` where the grammar reads an expression of at least level `min`.
    fn expr(&mut self, e: &Expr, min: i8) -> Doc {
        let doc = self.expr_raw(e);
        if level(e) < min {
            group(cat(vec![
                t("{"),
                nest(cat(vec![Doc::Line, doc])),
                Doc::Line,
                t("}"),
            ]))
        } else {
            doc
        }
    }

    /// The target of a call, `.f` or `.k`.
    fn target(&mut self, e: &Expr) -> Doc {
        if matches!(e.kind, ExprKind::Int(_) | ExprKind::Float(_)) {
            return cat(vec![t("{ "), self.expr_raw(e), t(" }")]);
        }
        self.expr(e, POSTFIX)
    }

    fn data_item(&mut self, item: &DItem) -> Part {
        let (start, end, doc_of): (u32, u32, _) = match item {
            DItem::Pos(e) => (e.span.0.start, e.span.0.end, None),
            DItem::Label(n, e) => (n.span.0.start, e.span.0.end, Some(format!("{} = ", n.name))),
            DItem::Spread(e) => (e.span.0.start, e.span.0.end, Some("..".to_string())),
        };
        let mut parts = self.leading(start);
        let value = match item {
            DItem::Pos(e) | DItem::Label(_, e) | DItem::Spread(e) => e,
        };
        if let Some(prefix) = doc_of {
            parts.push(t(prefix));
        }
        match (&value.kind, item) {
            (ExprKind::Chain(fs), DItem::Label(..)) if fs.len() > 1 => {
                let branches = self.branches(fs);
                parts.push(group(nest(branches)));
            }
            _ => parts.push(self.expr(value, LOWEST)),
        }
        Part {
            doc: cat(parts),
            trailing: self.trailing(end),
        }
    }

    /// Data, or the data of a call. When `hug` is set and the last item is a
    /// function or a block, it begins on the line of the `(`.
    fn data(&mut self, items: &[DItem], hug: bool) -> Doc {
        let hugged = hug
            && match items.last() {
                Some(DItem::Pos(e)) => match &e.kind {
                    ExprKind::Chain(fs) => fs.len() == 1,
                    ExprKind::Block(b) => b.stmts.len() > 1,
                    _ => false,
                },
                _ => false,
            }
            && !self.comment_before(match items.last() {
                Some(DItem::Pos(e)) => e.span.0.start,
                _ => 0,
            })
            && items[..items.len() - 1].iter().all(|i| match i {
                DItem::Pos(e) | DItem::Label(_, e) | DItem::Spread(e) => {
                    !matches!(e.kind, ExprKind::Chain(_) | ExprKind::Block(_))
                }
            });
        let parts: Vec<Part> = items.iter().map(|i| self.data_item(i)).collect();
        if hugged && parts.iter().all(|p| p.trailing.is_none()) {
            let mut out = vec![t("(")];
            for (i, part) in parts.into_iter().enumerate() {
                if i > 0 {
                    out.push(t(", "));
                }
                out.push(part.doc);
            }
            out.push(t(")"));
            return cat(out);
        }
        self.delimited("(", ")", parts)
    }

    fn branches(&mut self, fs: &[Function]) -> Doc {
        let mut out = Vec::new();
        for (i, f) in fs.iter().enumerate() {
            let lead = self.leading(f.span.0.start);
            if i > 0 {
                out.push(Doc::Line);
            }
            out.extend(lead);
            if i > 0 {
                out.push(t("| "));
            }
            out.push(self.function(f));
        }
        cat(out)
    }

    fn function(&mut self, f: &Function) -> Doc {
        let mut out = Vec::new();
        if !f.generics.is_empty() {
            let names: Vec<&str> = f.generics.iter().map(|g| g.name.as_str()).collect();
            out.push(t(format!("<{}>", names.join(", "))));
        }
        let mut inner = Vec::new();
        for (i, p) in f.params.iter().enumerate() {
            if i > 0 {
                inner.push(t(","));
                inner.push(Doc::Line);
            }
            if let Some(k) = &p.key {
                inner.push(t(format!("{} = ", k.name)));
            }
            for (j, c) in p.choices.iter().enumerate() {
                if j > 0 {
                    inner.push(t(" | "));
                }
                inner.push(self.pattern(c));
            }
        }
        if let Some(g) = &f.guard {
            if !f.params.is_empty() {
                inner.push(Doc::Line);
            }
            inner.push(t("? "));
            inner.push(self.expr(g, LOWEST));
        }
        if inner.is_empty() {
            out.push(t("()"));
        } else {
            inner.insert(0, Doc::Soft);
            out.push(group(cat(vec![t("("), nest(cat(inner)), Doc::Soft, t(")")])));
        }
        if let Some(services) = &f.services {
            out.push(t(" <- "));
            let parts = services
                .iter()
                .map(|s| {
                    let head = format!("{}{}: ", if s.rest { ".." } else { "" }, s.name.name);
                    cat(vec![t(head), self.ty(&s.ty, 0)])
                })
                .collect();
            out.push(self.delimited("(", ")", Self::plain(parts)));
        }
        if let Some(ret) = &f.ret {
            out.push(t(" -> "));
            out.push(self.ty(ret, 0));
        }
        out.push(t(" "));
        out.push(self.block(&f.body));
        cat(out)
    }

    fn string(&self, e: &Expr, s: &str) -> String {
        self.written(e.span, |k| matches!(k, T::Str(x) if x == s))
            .unwrap_or_else(|| string_literal(s))
    }

    fn expr_raw(&mut self, e: &Expr) -> Doc {
        match &e.kind {
            ExprKind::Quote(d) => t(crate::lisp::show(&d.0)),
            ExprKind::Int(n) => t(self
                .written(e.span, |k| matches!(k, T::Int(x) if x == n))
                .unwrap_or_else(|| n.to_string())),
            ExprKind::Float(x) => t(self
                .written(e.span, |k| matches!(k, T::Float(y) if y == x))
                .unwrap_or_else(|| float(*x))),
            ExprKind::String(s) => t(self.string(e, s)),
            ExprKind::Interp(parts) => {
                let mut s = String::from("\"");
                for part in parts {
                    match part {
                        InterpPart::Text(text) => {
                            let lit = string_literal(text);
                            s.push_str(&lit[1..lit.len() - 1]);
                        }
                        InterpPart::Expr(inner) => {
                            s.push_str("\\(");
                            let mut p = Printer {
                                src: None,
                                comments: &[],
                                next: 0,
                            };
                            let doc = p.expr(inner, LOWEST);
                            s.push_str(render_in(&doc, usize::MAX / 4).trim_end());
                            s.push(')');
                        }
                    }
                }
                s.push('"');
                t(s)
            }
            ExprKind::Name { module, name } => t(qualified(module, name)),
            ExprKind::Data(items) => self.data(items, false),
            ExprKind::List { items, rest } => {
                let mut parts = Vec::new();
                for item in items {
                    let mut docs = self.leading(item.span.0.start);
                    docs.push(self.expr(item, LOWEST));
                    parts.push(Part {
                        doc: cat(docs),
                        trailing: self.trailing(item.span.0.end),
                    });
                }
                if let Some(rest) = rest {
                    let mut docs = self.leading(rest.span.0.start);
                    docs.push(t(".."));
                    docs.push(self.expr(rest, LOWEST));
                    parts.push(Part {
                        doc: cat(docs),
                        trailing: self.trailing(rest.span.0.end),
                    });
                }
                self.delimited("[", "]", parts)
            }
            ExprKind::BitArray(segments) => {
                let parts = segments
                    .iter()
                    .map(|s| {
                        let value = self.expr(&s.value, LOWEST);
                        cat(vec![value, self.format(&s.format)])
                    })
                    .collect();
                self.delimited("<<", ">>", Self::plain(parts))
            }
            ExprKind::Block(b) => self.block(b),
            ExprKind::Chain(fs) => {
                let mut fs_iter = fs.iter();
                let first = fs_iter.next().expect("a chain has a branch");
                let lead = self.leading(first.span.0.start);
                let mut out = lead;
                out.push(self.function(first));
                if fs.len() == 1 {
                    return cat(out);
                }
                let mut more = Vec::new();
                for f in fs_iter {
                    more.push(Doc::Line);
                    more.extend(self.leading(f.span.0.start));
                    more.push(t("| "));
                    more.push(self.function(f));
                }
                out.push(nest(cat(more)));
                group(cat(out))
            }
            ExprKind::Call { callee, args } => self.call(e, callee, args),
            ExprKind::Provide { target, services } => {
                let target = self.expr(target, LOWEST);
                cat(vec![target, t(" <- "), self.expr(services, POSTFIX)])
            }
            ExprKind::Field { target, name } => {
                cat(vec![self.target(target), t(format!(".{}", name.name))])
            }
            ExprKind::Index { target, index } => {
                cat(vec![self.target(target), t(format!(".{index}"))])
            }
            ExprKind::Unary { op, expr } => {
                let op = match op {
                    UnOp::Neg => "-",
                    UnOp::NegFloat => "-.",
                };
                cat(vec![t(op), self.expr(expr, POSTFIX)])
            }
            ExprKind::Binary { op, .. } => {
                // The left spine of one level is one group.
                let lvl = op.level();
                let mut spine = Vec::new();
                let mut cur = e;
                while let ExprKind::Binary { op, lhs, rhs } = &cur.kind {
                    if op.level() != lvl {
                        break;
                    }
                    spine.push((*op, &**rhs));
                    cur = &**lhs;
                }
                let mut out = vec![self.expr(cur, lvl as i8)];
                let mut rest = Vec::new();
                for (op, rhs) in spine.into_iter().rev() {
                    out.push(t(format!(" {}", op.text())));
                    rest.push(Doc::Line);
                    rest.push(self.expr(rhs, lvl as i8 + 1));
                    out.push(nest(cat(std::mem::take(&mut rest))));
                }
                group(cat(out))
            }
            ExprKind::Pipe { .. } => {
                let mut spine = Vec::new();
                let mut cur = e;
                while let ExprKind::Pipe { lhs, rhs } = &cur.kind {
                    spine.push(&**rhs);
                    cur = &**lhs;
                }
                let mut out = vec![self.expr(cur, 1)];
                for rhs in spine.into_iter().rev() {
                    match &rhs.kind {
                        ExprKind::Chain(fs) if fs.len() > 1 => {
                            let branches = self.branches(fs);
                            out.push(t(" |>"));
                            out.push(nest(cat(vec![Doc::Line, branches])));
                        }
                        ExprKind::Chain(_) => {
                            out.push(t(" |> "));
                            out.push(self.expr_raw(rhs));
                        }
                        _ => {
                            let rhs = self.expr(rhs, 1);
                            out.push(nest(cat(vec![Doc::Line, t("|> "), rhs])));
                        }
                    }
                }
                group(cat(out))
            }
        }
    }

    /// A call. A call whose call depth ([`call_depth`]) is greater than
    /// [`MAX_CALL_DEPTH`] is never on one line: its arguments go one per line
    /// (the layout of a call that does not fit the width).
    fn call(&mut self, e: &Expr, callee: &Expr, args: &[DItem]) -> Doc {
        let callee = self.target(callee);
        if call_depth(e) <= MAX_CALL_DEPTH || args.is_empty() {
            return cat(vec![callee, self.data(args, true)]);
        }
        let parts: Vec<Part> = args.iter().map(|i| self.data_item(i)).collect();
        let Doc::Group(inner) = self.delimited("(", ")", parts) else {
            unreachable!("a delimited list with parts is a group")
        };
        let Doc::Cat(mut out) = *inner else {
            unreachable!("a delimited list is a sequence")
        };
        // `( nest(Soft part , …) …`: the first line end is always taken.
        let Doc::Nest(body) = &mut out[1] else {
            unreachable!("the parts of a delimited list are nested")
        };
        let Doc::Cat(body) = &mut **body else {
            unreachable!("the parts are a sequence")
        };
        body[0] = Doc::Hard;
        cat(vec![callee, group(cat(out))])
    }

    fn format(&mut self, format: &Option<Format>) -> Doc {
        let Some(f) = format else {
            return cat(Vec::new());
        };
        let mut out = vec![t(format!(":{}", f.name.name))];
        if let Some(arg) = &f.arg {
            out.push(t("("));
            out.push(self.expr(arg, LOWEST));
            out.push(t(")"));
        }
        cat(out)
    }

    // ---- Patterns ----

    fn binder(b: &Binder) -> String {
        b.name.clone()
    }

    fn data_pattern(&mut self, items: &[DPItem]) -> Doc {
        let parts = items
            .iter()
            .map(|i| match i {
                DPItem::Pos(p) => self.pattern(p),
                DPItem::Key(k, p) => cat(vec![t(format!("{} = ", k.name)), self.pattern(p)]),
                DPItem::Rest(None) => t(".."),
                DPItem::Rest(Some(b)) => t(format!("..{}", b.name)),
            })
            .collect();
        self.delimited("(", ")", Self::plain(parts))
    }

    fn pattern(&mut self, p: &Pattern) -> Doc {
        match &p.kind {
            PatternKind::Quote(d) => t(crate::lisp::show(&d.0)),
            PatternKind::Var(s) | PatternKind::Discard(s) => t(s.clone()),
            PatternKind::Typed { binder, ty } => {
                cat(vec![t(format!("{}: ", binder.name)), self.ty(ty, 0)])
            }
            PatternKind::Int { negative, value } => {
                let text = if *negative {
                    None
                } else {
                    self.written(p.span, |k| matches!(k, T::Int(x) if x == value))
                };
                t(text.unwrap_or_else(|| {
                    format!("{}{value}", if *negative { "-" } else { "" })
                }))
            }
            PatternKind::Float { negative, value } => t(format!(
                "{}{}",
                if *negative { "-." } else { "" },
                float(*value)
            )),
            PatternKind::String(s) => t(self
                .written(p.span, |k| matches!(k, T::Str(x) if x == s))
                .unwrap_or_else(|| string_literal(s))),
            PatternKind::Prefix {
                prefix,
                suffix,
                rest,
            } => {
                let mut s = if !prefix.is_empty() || suffix.is_empty() {
                    format!("{} <> {}", string_literal(prefix), Self::binder(rest))
                } else {
                    Self::binder(rest)
                };
                if !suffix.is_empty() {
                    s.push_str(&format!(" <> {}", string_literal(suffix)));
                }
                t(s)
            }
            PatternKind::Ctor { module, name, args } => {
                let mut out = vec![t(qualified(module, name))];
                if let Some(args) = args {
                    out.push(self.data_pattern(args));
                }
                cat(out)
            }
            PatternKind::Data(items) => self.data_pattern(items),
            PatternKind::List { items, rest } => {
                let mut parts: Vec<Doc> = items.iter().map(|i| self.pattern(i)).collect();
                match rest {
                    Some(Some(b)) => parts.push(t(format!("..{}", b.name))),
                    Some(None) => parts.push(t("..")),
                    None => {}
                }
                self.delimited("[", "]", Self::plain(parts))
            }
            PatternKind::BitArray(segments) => {
                let parts = segments
                    .iter()
                    .map(|s| {
                        let value = self.pattern(&s.value);
                        cat(vec![value, self.format(&s.format)])
                    })
                    .collect();
                self.delimited("<<", ">>", Self::plain(parts))
            }
            PatternKind::And(parts) => {
                let mut out = Vec::new();
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 {
                        out.push(t(" = "));
                    }
                    out.push(self.pattern(part));
                }
                cat(out)
            }
        }
    }

    // ---- Types ----

    fn data_type(&mut self, items: &[TItem]) -> Doc {
        let parts = items
            .iter()
            .map(|i| match i {
                TItem::Pos(ty) => self.ty(ty, 0),
                TItem::Label(n, ty) => cat(vec![t(format!("{} = ", n.name)), self.ty(ty, 0)]),
                TItem::Spread(ty) => cat(vec![t(".."), self.ty(ty, 3)]),
            })
            .collect();
        self.delimited("(", ")", Self::plain(parts))
    }

    /// `ty` where the grammar reads a type of at least level `min`.
    fn ty(&mut self, ty: &Type, min: u8) -> Doc {
        let doc = self.ty_raw(ty);
        if type_level(ty) < min {
            cat(vec![t("{ "), doc, t(" }")])
        } else {
            doc
        }
    }

    fn type_list(&mut self, parts: &[Type], op: &str, min: u8, breaks: bool) -> Doc {
        let mut out = Vec::new();
        let mut more = Vec::new();
        for (i, part) in parts.iter().enumerate() {
            // A function type takes everything to its right, so only the
            // last member of a union may be one without a group.
            let min = match part.kind {
                TypeKind::Fn(_) if op == "|" && i + 1 == parts.len() => 0,
                TypeKind::Fn(_) => 3,
                _ => min,
            };
            let doc = self.ty(part, min);
            if i == 0 {
                out.push(doc);
            } else if breaks {
                more.push(Doc::Line);
                more.push(t(format!("{op} ")));
                more.push(doc);
            } else {
                more.push(t(format!(" {op} ")));
                more.push(doc);
            }
        }
        out.push(nest(cat(more)));
        group(cat(out))
    }

    fn ty_raw(&mut self, ty: &Type) -> Doc {
        match &ty.kind {
            TypeKind::Lit(LitValue::Int(n)) => t(n.to_string()),
            TypeKind::Lit(LitValue::Str(s)) => t(string_literal(s)),
            TypeKind::Named { module, name, args } => {
                let mut out = vec![t(qualified(module, name))];
                if !args.is_empty() {
                    let parts = args.iter().map(|a| self.ty(a, 0)).collect();
                    out.push(self.delimited("(", ")", Self::plain(parts)));
                }
                cat(out)
            }
            TypeKind::ValueOf { module, name } => t(qualified(module, name)),
            TypeKind::Call { module, name, args } => {
                let parts = args.iter().map(|a| self.ty(a, 0)).collect();
                cat(vec![
                    t(qualified(module, name)),
                    self.delimited("(", ")", Self::plain(parts)),
                ])
            }
            TypeKind::Hole => t("_"),
            TypeKind::Data(items) => self.data_type(items),
            TypeKind::Fn(f) => {
                let mut out = Vec::new();
                if !f.generics.is_empty() {
                    let names: Vec<&str> = f.generics.iter().map(|g| g.name.as_str()).collect();
                    out.push(t(format!("<{}>", names.join(", "))));
                }
                match &f.param.kind {
                    TypeKind::Data(items) => out.push(self.data_type(items)),
                    _ => {
                        out.push(t("{ "));
                        out.push(self.ty(&f.param, 0));
                        out.push(t(" }"));
                    }
                }
                if let Some(s) = &f.services {
                    out.push(t(" <- "));
                    out.push(self.ty_raw(s));
                }
                out.push(t(" -> "));
                out.push(self.ty(&f.ret, 0));
                cat(out)
            }
            TypeKind::Union(parts) => self.type_list(parts, "|", 1, true),
            TypeKind::Inter(parts) => self.type_list(parts, "&", 2, false),
            TypeKind::Diff(parts) => self.type_list(parts, "-", 3, false),
        }
    }
}
