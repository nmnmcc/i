//! The formatter of `i fmt`: one canonical layout for i source.
//!
//! It is a pretty printer in the style of Wadler and Leijen ("A prettier
//! printer"): the tree is turned into a document of text, groups, nesting, and
//! line ends, and each group is laid out on one line when it fits in
//! [`WIDTH`] columns, and over several lines when not. Then, as gofmt does,
//! runs of lines of the same shape are aligned with spaces: trailing
//! comments, the `->` of one-line `case` clauses, and the parts after
//! `label:` when they are one per line. Indentation is one tab per level.
//!
//! The layout keeps the meaning (§1.7): a line is only ever ended where the
//! line end is a break that the construct wants (between statements, clauses,
//! items, and the parts of brackets, where a break is a `,`), or where it is
//! not a break at all (after an operator, `(`, `[`, `{`, `<<`, `->`, `=`, or
//! before `|>`). Literals are printed as they are written in the source.
//! Comments are put back where they were, in the same order, and without the
//! spaces at their ends (but documentation): trailing comments at the end of
//! their line, a comment in the middle of an expression at the end of the
//! line it is in, with what comes after it one level in, and other comments
//! on their own lines.

use crate::ast::*;
use crate::lexer::{Comment, lex};
use crate::migrate::{self, Env, Plan};
use crate::print::{is_match, level, type_prec};
use crate::span::Span;
use crate::token::{Token, TokenKind as T};
use crate::{Code, Diagnostic, old_syntax_hint, parse_module, parse_module_old};
use std::collections::HashSet;

/// The width that the layout tries to keep to.
pub const WIDTH: usize = 80;

/// The columns that a tab counts for when lines are measured.
pub const TAB_WIDTH: usize = 4;

/// Formats a source file. A file with syntax errors is not formatted; its
/// errors are given back. Each node keeps the syntax it is written in.
///
/// Until stream A3, the parser reads the old and the new syntax. A file that
/// the old syntax reads in another way, with no error, is not formatted: a
/// lowercase name before a `(` on its line in a type is a `TypeCall` in the
/// new syntax, but may end an old item before a binding that starts with
/// `(`.
pub fn format(source: &str) -> Result<String, Vec<Diagnostic>> {
    format_migration(source, false, "module", true).map(|(out, _)| out)
}

pub fn format_from_old(source: &str) -> Result<String, Vec<Diagnostic>> {
    format_migration(source, true, "module", true).map(|(out, _)| out)
}

/// Formats with the same pure plan used by the byte splicer and the tree
/// transform. Only --from-old needs the write check on this path.
pub fn format_migration(
    source: &str,
    from_old: bool,
    module_name: &str,
    choices_split: bool,
) -> Result<(String, Plan), Vec<Diagnostic>> {
    let module = parse_for_format(source, from_old)?;
    let mut env = Env::new(&module);
    env.module = module_name.into();
    env.choices_split = choices_split;
    let plan = migrate::plan(&module, &env);
    if let Some(refused) = &plan.refused {
        return Err(vec![migrate::refused_diagnostic(refused)]);
    }
    let out = format_planned(source, &module, &plan);
    if from_old {
        migrate::check_written(&module, &plan, &out)?;
    }
    Ok((out, plan))
}

fn parse_for_format(source: &str, from_old: bool) -> Result<Module, Vec<Diagnostic>> {
    let (module, mut diags) = if from_old {
        parse_module_old(source)
    } else {
        parse_module(source)
    };
    if !diags.is_empty() {
        if let Some(hint) = old_syntax_hint(source) {
            for d in &mut diags {
                d.message.push_str("; ");
                d.message.push_str(hint);
            }
        }
        return Err(diags);
    }
    if !from_old && let Some(span) = lower_type_call(&module) {
        let (old, old_diags) = parse_module_old(source);
        if old_diags.is_empty() && crate::shape::shape(&old) != crate::shape::shape(&module) {
            let msg = "this file reads differently in the old syntax; run `i fmt --from-old`";
            return Err(vec![Diagnostic::new(Code::DualAmbiguous, span, msg)]);
        }
    }
    Ok(module)
}

pub(crate) fn format_planned(source: &str, module: &Module, plan: &Plan) -> String {
    let lexed = lex(source);
    let mut f = Formatter {
        src: source,
        toks: &lexed.tokens,
        close: &lexed.close,
        comments: &lexed.comments,
        next: 0,
        hard: 0,
        one_line: false,
        fn_next: false,
        plan,
        expanding: HashSet::new(),
        printing_function: HashSet::new(),
    };
    let doc = f.module(module);
    debug_assert_eq!(f.next, f.comments.len(), "every comment is printed");
    render(&doc, WIDTH)
}

/// The span of the first `TypeCall` in `module` with a lowercase name and
/// no module name.
fn lower_type_call(module: &Module) -> Option<Span> {
    let mut found = None;
    for item in &module.items {
        match &item.kind {
            ItemKind::TypeDef(def) => {
                def.head.iter().for_each(|t| find_ty(t, &mut found));
                match &def.body {
                    TypeBody::Alias(t) => find_ty(t, &mut found),
                    TypeBody::Custom(cs) => cs
                        .iter()
                        .flat_map(|c| &c.fields)
                        .for_each(|f| find_ty(&f.ty, &mut found)),
                }
            }
            ItemKind::Binding(b) => {
                find_pat(&b.pattern, &mut found);
                find_expr(&b.value, &mut found);
            }
            ItemKind::Declaration { ty, .. } => find_ty(ty, &mut found),
            ItemKind::MacroItem { args, .. } => {
                args.iter().for_each(|a| find_expr(&a.value, &mut found))
            }
        }
        for a in &item.attributes {
            if let Some((args, _)) = &a.call {
                args.iter().for_each(|a| find_expr(&a.value, &mut found));
            }
        }
    }
    found
}

fn find_ty(t: &TypeExpr, found: &mut Option<Span>) {
    match &t.kind {
        TypeKind::Call { module, args, .. } => {
            if module.is_none() && found.is_none() {
                *found = Some(t.span);
            }
            for a in args {
                if let TypeArg::Type(t) = a {
                    find_ty(t, found);
                }
            }
        }
        TypeKind::Named { args, .. } => args.iter().for_each(|a| find_ty(a, found)),
        TypeKind::Tuple(ts) | TypeKind::Union(ts) | TypeKind::Inter(ts) | TypeKind::Diff(ts) => {
            ts.iter().for_each(|a| find_ty(a, found))
        }
        TypeKind::Fn {
            params,
            ret,
            err,
            env,
        } => {
            params.iter().for_each(|p| find_ty(&p.ty, found));
            find_ty(ret, found);
            err.iter().chain(env).for_each(|a| find_ty(a, found));
        }
        TypeKind::Var(_) | TypeKind::ValueOf { .. } | TypeKind::Hole => {}
    }
}

fn find_pat(p: &Pattern, found: &mut Option<Span>) {
    match &p.kind {
        PatternKind::Typed { ty, .. } => find_ty(ty, found),
        PatternKind::Named { pattern, .. } => find_pat(pattern, found),
        PatternKind::Constructor { args: ps, .. } | PatternKind::Tuple(ps) => {
            ps.iter().for_each(|a| find_pat(a, found))
        }
        PatternKind::List { items, .. } => items.iter().for_each(|a| find_pat(a, found)),
        PatternKind::BitArray(segs) => {
            for s in segs {
                find_pat(&s.value, found);
                if let Some(arg) = s.format.as_ref().and_then(|f| f.arg.as_deref()) {
                    find_expr(arg, found);
                }
            }
        }
        _ => {}
    }
}

fn find_block(b: &Block, found: &mut Option<Span>) {
    for s in &b.stmts {
        match s {
            Stmt::Bind(b) => {
                find_pat(&b.pattern, found);
                find_expr(&b.value, found);
            }
            Stmt::Env { ty, value, .. } => {
                find_ty(ty, found);
                find_expr(value, found);
            }
            Stmt::Expr(e) => find_expr(e, found),
        }
    }
}

fn find_expr(e: &Expr, found: &mut Option<Span>) {
    match &e.kind {
        ExprKind::Fn(f) => {
            for p in &f.params {
                match p {
                    Param::Pattern(p) => find_pat(p, found),
                    Param::Env { ty, .. } => find_ty(ty, found),
                }
            }
            if let Some(r) = &f.ret {
                find_ty(&r.value, found);
                r.err.iter().chain(&r.env).for_each(|t| find_ty(t, found));
            }
            find_block(&f.body, found);
        }
        ExprKind::Block(b) => find_block(b, found),
        ExprKind::Case {
            subject, clauses, ..
        } => {
            find_expr(subject, found);
            find_clauses(clauses, found);
        }
        ExprKind::Clauses(clauses) => find_clauses(clauses, found),
        ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::String(_) | ExprKind::Name { .. } => {}
        ExprKind::Tuple(items) => items.iter().for_each(|x| find_expr(x, found)),
        ExprKind::List { items, rest } => items
            .iter()
            .chain(rest.as_deref())
            .for_each(|x| find_expr(x, found)),
        ExprKind::BitArray(segs) => {
            for s in segs {
                find_expr(&s.value, found);
                if let Some(arg) = s.format.as_ref().and_then(|f| f.arg.as_deref()) {
                    find_expr(arg, found);
                }
            }
        }
        ExprKind::Call { callee, args } => {
            find_expr(callee, found);
            args.iter().for_each(|a| find_expr(&a.value, found));
        }
        ExprKind::Field { target, .. } | ExprKind::TupleIndex { target, .. } => {
            find_expr(target, found)
        }
        ExprKind::RecordUpdate {
            constructor,
            base,
            fields,
        } => {
            find_expr(constructor, found);
            find_expr(base, found);
            fields.iter().for_each(|(_, x)| find_expr(x, found));
        }
        ExprKind::Unary { expr, .. } => find_expr(expr, found),
        ExprKind::Binary { lhs, rhs, .. } => {
            find_expr(lhs, found);
            find_expr(rhs, found);
        }
    }
}

fn find_clauses(clauses: &[Clause], found: &mut Option<Span>) {
    for c in clauses {
        c.patterns.iter().for_each(|p| find_pat(p, found));
        c.guard.iter().for_each(|g| find_expr(g, found));
        find_expr(&c.body, found);
    }
}

// ---- Documents ----

/// What a column of aligned lines lines up (gofmt's elastic tabstops).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Align {
    /// The part after `label:` of a field or an argument.
    Label,
    /// The `->` of a `case` clause.
    Arrow,
    /// A trailing comment.
    Comment,
}

#[derive(Clone, Debug)]
enum Doc {
    Text(String),
    /// Text only when the group is on one line, such as the `,` between
    /// parts.
    Flat(&'static str),
    /// A space on one line, or a line end.
    Line,
    /// Nothing on one line, or a line end.
    Soft,
    /// A space, unless the line is empty so far or already ends with a space:
    /// before a comment that goes on the line of the text before it.
    Gap,
    /// Always a line end; the group around it is never on one line.
    Hard,
    /// The group around it is never on one line.
    Force,
    /// A space on one line; in a group over several lines, a space where
    /// lines of the same shape are aligned.
    Tab(Align),
    /// One more level of indentation for the lines in it.
    Nest(Box<Doc>),
    /// A part that is not last in its line, such as an argument before the
    /// last one: when it is measured, a block that hugs inside it is
    /// measured in full, because more comes after it.
    Full(Box<Doc>),
    /// A group, whether it is forced over several lines, and its kind.
    Group(Box<Doc>, bool, Kind),
    Cat(Vec<Doc>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Plain,
    /// A block of one expression: when it is measured, all of it is measured,
    /// also the blocks that hug inside it.
    Block,
    /// A block at the end of what it is in, which may begin at the end of a
    /// line: when the line before it is measured, it counts only up to its
    /// `{`, and it is laid out on its own even inside a group on one line.
    Hug,
}

fn text(s: impl Into<String>) -> Doc {
    Doc::Text(s.into())
}

fn cat(parts: Vec<Doc>) -> Doc {
    Doc::Cat(parts)
}

fn nest(d: Doc) -> Doc {
    Doc::Nest(Box::new(d))
}

/// Whether `d` has a `Hard` or a `Force` outside its inner groups.
fn forces(d: &Doc) -> bool {
    match d {
        Doc::Hard | Doc::Force => true,
        Doc::Nest(d) | Doc::Full(d) => forces(d),
        Doc::Cat(v) => v.iter().any(forces),
        _ => false,
    }
}

fn group(d: Doc) -> Doc {
    let forced = forces(&d);
    Doc::Group(Box::new(d), forced, Kind::Plain)
}

fn group_of(kind: Kind, d: Doc) -> Doc {
    let forced = forces(&d);
    Doc::Group(Box::new(d), forced, kind)
}

// ---- Layout ----

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Flat,
    Break,
}

fn width(s: &str) -> usize {
    s.chars().count()
}

/// One line of output: its indentation, and its text cut at the alignment
/// marks.
#[derive(Default)]
struct OutLine {
    indent: usize,
    parts: Vec<String>,
    marks: Vec<Align>,
    /// The line goes on from a string literal of several lines, so it is
    /// printed as it is, without indentation.
    raw: bool,
    /// A string literal of several lines starts on this line.
    open: bool,
}

impl OutLine {
    fn is_empty(&self) -> bool {
        !self.raw && self.parts.iter().all(|p| p.is_empty())
    }
}

struct Out {
    lines: Vec<OutLine>,
}

impl Out {
    fn line(&mut self) -> &mut OutLine {
        self.lines.last_mut().unwrap()
    }

    fn push(&mut self, s: &str) {
        let mut pieces = s.split('\n');
        let first = pieces.next().unwrap_or("");
        self.line().parts.last_mut().unwrap().push_str(first);
        for piece in pieces {
            self.line().open = true;
            self.lines.push(OutLine {
                parts: vec![piece.to_string()],
                raw: true,
                ..OutLine::default()
            });
        }
    }

    fn newline(&mut self, indent: usize) {
        self.lines.push(OutLine {
            indent,
            parts: vec![String::new()],
            ..OutLine::default()
        });
    }
}

/// Whether the rest of the line fits in `rem` columns, with `next` laid out on
/// one line, and then the commands of `rest` in their modes. A block that
/// hugs counts only up to its first line end, but not inside a block, where
/// `full` is set.
fn fits(next: &Doc, rest: &[(usize, Mode, &Doc)], mut rem: isize, full: bool) -> bool {
    let mut stack: Vec<(Mode, &Doc, bool)> = vec![(Mode::Flat, next, full)];
    let mut rest_index = rest.len();
    loop {
        if rem < 0 {
            return false;
        }
        let (mode, d, full) = match stack.pop() {
            Some(x) => x,
            None => {
                if rest_index == 0 {
                    return true;
                }
                rest_index -= 1;
                let (_, mode, d) = rest[rest_index];
                (mode, d, false)
            }
        };
        match d {
            Doc::Text(s) => match s.split_once('\n') {
                Some((first, _)) => return rem >= width(first) as isize,
                None => rem -= width(s) as isize,
            },
            Doc::Flat(s) => {
                if mode == Mode::Flat {
                    rem -= width(s) as isize;
                }
            }
            Doc::Line => match mode {
                Mode::Flat => rem -= 1,
                Mode::Break => return true,
            },
            Doc::Soft => {
                if mode == Mode::Break {
                    return true;
                }
            }
            Doc::Hard => return true,
            Doc::Force => {}
            Doc::Tab(_) | Doc::Gap => rem -= 1,
            Doc::Nest(d) => stack.push((mode, d, full)),
            Doc::Full(d) => stack.push((mode, d, true)),
            Doc::Cat(v) => stack.extend(v.iter().rev().map(|d| (mode, d, full))),
            Doc::Group(d, forced, kind) => {
                let mode = if *forced || *kind == Kind::Hug && !full {
                    Mode::Break
                } else {
                    mode
                };
                stack.push((mode, d, full || *kind != Kind::Plain));
            }
        }
    }
}

fn render(doc: &Doc, max: usize) -> String {
    let mut out = Out { lines: Vec::new() };
    out.newline(0);
    let mut col = 0usize;
    let mut cmds: Vec<(usize, Mode, &Doc)> = vec![(0, Mode::Break, doc)];
    while let Some((indent, mode, d)) = cmds.pop() {
        match d {
            Doc::Text(s) => {
                out.push(s);
                col = match s.rsplit_once('\n') {
                    Some((_, last)) => width(last),
                    None => col + width(s),
                };
            }
            Doc::Flat(s) => {
                if mode == Mode::Flat {
                    out.push(s);
                    col += width(s);
                }
            }
            Doc::Line | Doc::Soft | Doc::Hard => {
                if mode == Mode::Break || matches!(d, Doc::Hard) {
                    out.newline(indent);
                    col = indent * TAB_WIDTH;
                } else if matches!(d, Doc::Line) {
                    out.push(" ");
                    col += 1;
                }
            }
            Doc::Force => {}
            Doc::Gap => {
                let last = out.line().parts.last().unwrap();
                if !last.is_empty() && !last.ends_with([' ', '\t']) {
                    out.push(" ");
                    col += 1;
                }
            }
            Doc::Tab(kind) => {
                if mode == Mode::Break {
                    let line = out.line();
                    line.marks.push(*kind);
                    line.parts.push(String::new());
                } else {
                    out.push(" ");
                }
                col += 1;
            }
            // In a group on one line, only the lines of forced groups inside it
            // end; they are indented from the line where the group began.
            Doc::Nest(d) => cmds.push((indent + usize::from(mode == Mode::Break), mode, d)),
            Doc::Full(d) => cmds.push((indent, mode, d)),
            Doc::Cat(v) => cmds.extend(v.iter().rev().map(|d| (indent, mode, d))),
            Doc::Group(d, forced, kind) => {
                let mode = if *forced {
                    Mode::Break
                } else if mode == Mode::Flat && *kind != Kind::Hug
                    || fits(d, &cmds, max as isize - col as isize, *kind != Kind::Plain)
                {
                    Mode::Flat
                } else {
                    Mode::Break
                };
                cmds.push((indent, mode, d));
            }
        }
    }
    align(&mut out.lines);
    let mut text = String::new();
    for line in &out.lines {
        if line.is_empty() {
            text.push('\n');
            continue;
        }
        if !line.raw {
            text.extend(std::iter::repeat_n('\t', line.indent));
        }
        text.push_str(&line.parts.concat());
        text.push('\n');
    }
    let kept = text.trim_end_matches('\n').len();
    text.truncate(kept);
    if !text.is_empty() {
        text.push('\n');
    }
    text
}

/// Aligns the marks of each kind over runs of lines that have one, as the
/// elastic tabstops of gofmt do. A run is consecutive lines with the same
/// indentation; a blank line or a line without the mark ends it. Labels and
/// arrows only count on lines that hold their whole part, so a part that goes
/// on in the lines after it is not aligned.
fn align(lines: &mut [OutLine]) {
    for kind in [Align::Label, Align::Arrow, Align::Comment] {
        let marked: Vec<Option<usize>> = (0..lines.len())
            .map(|i| {
                let line = &lines[i];
                let j = line.marks.iter().position(|&m| m == kind)?;
                if line.raw || line.open {
                    return None;
                }
                let goes_on = lines
                    .get(i + 1)
                    .is_some_and(|n| !n.raw && !n.is_empty() && n.indent > line.indent);
                (kind == Align::Comment || !goes_on).then_some(j)
            })
            .collect();
        let mut i = 0;
        while i < lines.len() {
            let Some(_) = marked[i] else {
                i += 1;
                continue;
            };
            let mut end = i;
            while end + 1 < lines.len()
                && marked[end + 1].is_some()
                && lines[end + 1].indent == lines[i].indent
            {
                end += 1;
            }
            let column = (i..=end)
                .map(|k| prefix(&lines[k], marked[k].unwrap()))
                .max()
                .unwrap();
            for k in i..=end {
                let j = marked[k].unwrap();
                merge(&mut lines[k], j, column);
            }
            i = end + 1;
        }
        for line in lines.iter_mut() {
            while let Some(j) = line.marks.iter().position(|&m| m == kind) {
                let column = prefix(line, j);
                merge(line, j, column);
            }
        }
    }
}

fn prefix(line: &OutLine, mark: usize) -> usize {
    line.parts[..=mark].iter().map(|p| width(p)).sum::<usize>() + mark
}

/// Replaces mark `j` with the spaces that put the text after it at `column`
/// plus one.
fn merge(line: &mut OutLine, j: usize, column: usize) {
    let pad = column + 1 - prefix(line, j);
    let next = line.parts.remove(j + 1);
    line.marks.remove(j);
    let part = &mut line.parts[j];
    part.extend(std::iter::repeat_n(' ', pad));
    part.push_str(&next);
}

// ---- From the tree to a document ----

fn spans_of(items: &[Expr]) -> Vec<(u32, u32)> {
    items.iter().map(|e| (e.span.start, e.span.end)).collect()
}

struct Formatter<'a> {
    src: &'a str,
    toks: &'a [Token],
    close: &'a [Option<u32>],
    comments: &'a [Comment],
    /// The next comment to print.
    next: usize,
    /// How many `Hard` and `Force` were made, to know whether a part has one.
    hard: usize,
    /// Set in the `->` part of a function, which stays on the line of its
    /// `)` and `{`: its brackets are not broken.
    one_line: bool,
    /// Set while a clause pattern is printed that `?` or `|` comes after,
    /// which a function type at its end would take (§7.1).
    fn_next: bool,
    plan: &'a Plan,
    expanding: HashSet<Span>,
    printing_function: HashSet<Span>,
}

/// How blank lines between the lines of a list of statements, clauses,
/// constructors, or items are kept.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Blank {
    /// At most one blank line where the source has one or more.
    Keep,
    /// The top level: one blank line after every item, and at most one where
    /// the source has one or more.
    Top,
}

/// The binding strength that the members of a type need (§3).
fn type_braced(t: &TypeExpr, min: u8) -> bool {
    type_prec(t) < min
}

/// Whether the printed type ends with a function type, which would take a
/// `|`, `&`, `-`, `?`, or `!` after it (§3).
fn ends_with_fn(t: &TypeExpr) -> bool {
    match &t.kind {
        TypeKind::Fn { .. } => true,
        TypeKind::Union(v) | TypeKind::Inter(v) | TypeKind::Diff(v) => {
            let last = v.last().unwrap();
            !type_braced(last, type_prec(t) + 1) && ends_with_fn(last)
        }
        _ => false,
    }
}

/// Whether the printed type ends with a tuple type, which becomes a function
/// type when `->` comes after it (§7.1).
fn ends_with_tuple(t: &TypeExpr) -> bool {
    match &t.kind {
        TypeKind::Tuple(_) => true,
        TypeKind::Union(v) | TypeKind::Inter(v) | TypeKind::Diff(v) => {
            let last = v.last().unwrap();
            !type_braced(last, type_prec(t) + 1) && ends_with_tuple(last)
        }
        TypeKind::Fn { ret, err, env, .. } => {
            ends_with_tuple(env.as_deref().or(err.as_deref()).unwrap_or(ret))
        }
        _ => false,
    }
}

impl<'a> Formatter<'a> {
    // ---- Source and tokens ----

    fn slice(&self, span: Span) -> &'a str {
        &self.src[span.start as usize..span.end as usize]
    }

    /// The index of the first token that starts at `pos` or after it.
    fn tok_index(&self, pos: u32) -> usize {
        self.toks.partition_point(|t| t.span.start < pos)
    }

    /// The first token at `pos` or after it that is not a `,`.
    fn tok_after(&self, pos: u32) -> &'a Token {
        let mut i = self.tok_index(pos);
        while self.toks[i].kind == T::Comma {
            i += 1;
        }
        &self.toks[i]
    }

    /// The start of the bracket that closes the bracket at `open`.
    fn closer(&self, open: u32) -> u32 {
        let i = self.tok_index(open);
        let c = self.close[i].expect("a parsed bracket is closed");
        self.toks[c as usize].span.start
    }

    fn line_ends(&self, from: u32, to: u32) -> usize {
        if from >= to {
            return 0;
        }
        let gap = &self.src[from as usize..to as usize];
        gap.matches('\n').count() + gap.matches('\r').count() - gap.matches("\r\n").count()
    }

    fn blank_between(&self, from: u32, to: u32) -> bool {
        self.line_ends(from, to) >= 2
    }

    /// Whether a comment at `start` is at the end of the line of a part that
    /// ends at `end`: only spaces and `,` are between them.
    fn trails(&self, end: u32, start: u32) -> bool {
        start < end
            || self.src[end as usize..start as usize]
                .chars()
                .all(|c| matches!(c, ' ' | '\t' | ','))
    }

    // ---- Comments ----

    fn peek(&self) -> Option<&'a Comment> {
        self.comments.get(self.next)
    }

    /// The next comment, without the spaces at its end. The text of a `///`
    /// or `////` comment is documentation, which is kept as it is: in
    /// Markdown, spaces at the end of a line end the line.
    fn take(&mut self) -> Doc {
        let c = &self.comments[self.next];
        self.next += 1;
        if c.text.starts_with("///") {
            text(c.text.as_str())
        } else {
            text(c.text.trim_end())
        }
    }

    /// `label:` and the space after it, where lines of labels are aligned.
    fn label(&self, name: &str) -> Doc {
        let space = if self.one_line {
            text(" ")
        } else {
            Doc::Tab(Align::Label)
        };
        cat(vec![text(format!("{name}:")), space])
    }

    fn hard(&mut self) -> Doc {
        self.hard += 1;
        Doc::Hard
    }

    fn force(&mut self) -> Doc {
        self.hard += 1;
        Doc::Force
    }

    /// The comments before `pos` that were not printed yet, each on its own
    /// line, for the middle of an expression, a pattern, or a type.
    fn leading(&mut self, pos: u32) -> Doc {
        let mut out = Vec::new();
        while self.peek().is_some_and(|c| c.span.start < pos) {
            out.push(self.take());
            out.push(self.hard());
        }
        cat(out)
    }

    /// The comments before `pos` that were not printed yet, in the middle of
    /// an expression, a pattern, or a type: the first goes on the line of the
    /// text before it, after a space, and the others each on its own line. A
    /// line end must come after them.
    fn mid_comments(&mut self, pos: u32) -> Option<Doc> {
        if !self.peek().is_some_and(|c| c.span.start < pos) {
            return None;
        }
        let mut out = vec![Doc::Gap, self.take()];
        while self.peek().is_some_and(|c| c.span.start < pos) {
            out.push(self.hard());
            out.push(self.take());
        }
        Some(cat(out))
    }

    /// The part at `pos` after the comments before it, if there are any: it
    /// goes on in the next line, one level in.
    fn continued(&mut self, pos: u32, part: impl FnOnce(&mut Self) -> Doc) -> Doc {
        match self.mid_comments(pos) {
            Some(comments) => {
                let end = self.hard();
                let part = part(self);
                cat(vec![comments, nest(cat(vec![end, part]))])
            }
            None => part(self),
        }
    }

    /// The comment at the end of the line of a part that ends at `end`, if
    /// there is one before `limit`.
    fn trailing(&mut self, end: u32, limit: u32) -> Option<Doc> {
        let c = self.peek()?;
        if c.span.start >= limit || !self.trails(end, c.span.start) {
            return None;
        }
        let comment = self.take();
        Some(cat(vec![Doc::Tab(Align::Comment), comment, self.force()]))
    }

    // ---- Lists of lines and bracketed parts ----

    /// The lines of a block, a `case`, a type, or the file: each part on its
    /// own line, with the comments before and after it. `open` is the end of
    /// the opening bracket, or `None` at the top level, and `close` the start
    /// of the closing bracket. Each line begins with a line end, except the
    /// first line at the top level.
    fn lines(
        &mut self,
        open: Option<u32>,
        close: u32,
        spans: &[(u32, u32)],
        blank: Blank,
        mut part: impl FnMut(&mut Self, usize) -> Doc,
    ) -> Vec<Doc> {
        let mut out = Vec::new();
        let mut prev = open.unwrap_or(0);
        let mut first = open.is_none();
        let mut after_item = false;
        // A comment on the line of the opening bracket stays there.
        if open.is_some() {
            let limit = spans.first().map_or(close, |s| s.0);
            if let Some(c) = self.peek()
                && c.span.start < limit
                && self.line_ends(prev, c.span.start) == 0
            {
                prev = c.span.end;
                out.push(text(" "));
                out.push(self.take());
            }
        }
        let sep = |f: &mut Self,
                   out: &mut Vec<Doc>,
                   prev: u32,
                   start: u32,
                   first: bool,
                   after_item: bool| {
            if first {
                return;
            }
            out.push(f.hard());
            let blank_line = match blank {
                Blank::Keep => f.blank_between(prev, start),
                Blank::Top => after_item || f.blank_between(prev, start),
            };
            if blank_line {
                out.push(f.hard());
            }
        };
        for (i, &(start, end)) in spans.iter().enumerate() {
            while let Some(c) = self.peek().filter(|c| c.span.start < start) {
                sep(self, &mut out, prev, c.span.start, first, after_item);
                prev = c.span.end;
                first = false;
                after_item = false;
                out.push(self.take());
            }
            sep(self, &mut out, prev, start, first, after_item);
            first = false;
            out.push(part(self, i));
            prev = end;
            if let Some(t) = self.trailing(end, close) {
                // A comment from inside the part is put after it, but the
                // blank lines after the part are counted from its end.
                prev = prev.max(self.comments[self.next - 1].span.end);
                out.push(t);
            }
            after_item = true;
        }
        while let Some(c) = self.peek().filter(|c| c.span.start < close) {
            sep(self, &mut out, prev, c.span.start, first, after_item);
            prev = c.span.end;
            first = false;
            after_item = false;
            out.push(self.take());
        }
        out
    }

    /// `{` and the lines of a block, a `case`, or a type, and `}`, with the
    /// lines one level in.
    fn braced_lines(
        &mut self,
        open: u32,
        close: u32,
        spans: &[(u32, u32)],
        part: impl FnMut(&mut Self, usize) -> Doc,
    ) -> Doc {
        if spans.is_empty() && self.peek().is_none_or(|c| c.span.start >= close) {
            return text("{}");
        }
        let lines = self.lines(Some(open + 1), close, spans, Blank::Keep, part);
        let end = self.hard();
        group(cat(vec![text("{"), nest(cat(lines)), end, text("}")]))
    }

    /// Bracketed parts: on one line with `, ` between them when they fit, or
    /// one per line with no `,` (a break is a `,`, §1.7). `open` is the start
    /// of the opening bracket and `close` the start of the closing one.
    #[allow(clippy::too_many_arguments)]
    fn seq(
        &mut self,
        left: &str,
        right: &str,
        open: u32,
        close: u32,
        spans: &[(u32, u32)],
        always_break: bool,
        mut part: impl FnMut(&mut Self, usize) -> Doc,
    ) -> Doc {
        if self.one_line && self.peek().is_none_or(|c| c.span.start >= close) {
            let mut out = vec![text(left)];
            for i in 0..spans.len() {
                if i > 0 {
                    out.push(text(", "));
                }
                out.push(part(self, i));
            }
            out.push(text(right));
            return cat(out);
        }
        let mut inner = Vec::new();
        if always_break && !spans.is_empty() {
            inner.push(self.force());
        }
        let open_end = open + left.len() as u32;
        let limit = spans.first().map_or(close, |s| s.0);
        if let Some(c) = self.peek()
            && c.span.start < limit
            && self.line_ends(open_end, c.span.start) == 0
        {
            inner.push(text(" "));
            inner.push(self.take());
            inner.push(self.force());
        }
        for (i, &(start, end)) in spans.iter().enumerate() {
            if i == 0 {
                inner.push(Doc::Soft);
            } else {
                inner.push(Doc::Flat(","));
                inner.push(Doc::Line);
            }
            while self.peek().is_some_and(|c| c.span.start < start) {
                inner.push(self.take());
                inner.push(self.hard());
            }
            let item = part(self, i);
            inner.push(if i + 1 < spans.len() {
                Doc::Full(Box::new(item))
            } else {
                item
            });
            if let Some(t) = self.trailing(end, close) {
                inner.push(t);
            }
        }
        while self.peek().is_some_and(|c| c.span.start < close) {
            inner.push(self.hard());
            inner.push(self.take());
        }
        if inner.is_empty() {
            return text(format!("{left}{right}"));
        }
        group(cat(vec![
            text(left),
            nest(cat(inner)),
            Doc::Soft,
            text(right),
        ]))
    }

    // ---- Module (§2.2 to §2.5) ----

    fn module(&mut self, m: &Module) -> Doc {
        enum Thing<'m> {
            Import(u32, HeaderStyle, &'m [ImportItem]),
            Export(u32, HeaderStyle, &'m [ExportItem]),
            Item(&'m Item),
        }
        let mut things = Vec::new();
        let mut spans = Vec::new();
        let header = [
            (m.imports_span, m.imports_style, true),
            (m.exports_span, m.exports_style, false),
        ];
        for (span, style, is_import) in header {
            let Some(span) = span else { continue };
            if (is_import && self.plan.drop_imports) || (!is_import && self.plan.drop_exports) {
                continue;
            }
            let mut at = self.tok_index(span.start);
            if style == HeaderStyle::Keyword {
                at += 1;
            }
            let paren = self.toks[at].span.start;
            spans.push((span.start, span.end));
            things.push(if is_import {
                Thing::Import(paren, HeaderStyle::Group, &m.imports)
            } else {
                Thing::Export(paren, HeaderStyle::Group, &m.exports)
            });
        }
        for item in &m.items {
            spans.push((item.span.start, item.span.end));
            things.push(Thing::Item(item));
        }
        let end = self.src.len() as u32;
        let mut lines = self.lines(None, end, &spans, Blank::Top, |f, i| match &things[i] {
            Thing::Import(paren, style, items) => f.import(*paren, *style, items),
            Thing::Export(paren, style, items) => f.export(*paren, *style, items),
            Thing::Item(item) => f.item(item),
        });
        lines.push(Doc::Hard);
        cat(lines)
    }

    fn import(&mut self, paren: u32, style: HeaderStyle, items: &[ImportItem]) -> Doc {
        let close = self.closer(paren);
        let spans: Vec<_> = items.iter().map(|i| (i.span.start, i.span.end)).collect();
        let list = self.seq("(", ")", paren, close, &spans, true, |f, i| {
            let item = &items[i];
            let mut out = Vec::new();
            if let Some(alias) = &item.alias {
                out.push(text(format!("{} = ", alias.name)));
            }
            if !item.names.is_empty() {
                let mut at = f.tok_index(item.span.start);
                if item.alias.is_some() {
                    at += 2;
                }
                let open = f.toks[at].span.start;
                let close = f.closer(open);
                let spans: Vec<_> = item
                    .names
                    .iter()
                    .map(|n| (n.span.start, n.span.end))
                    .collect();
                out.push(f.seq("(", ")", open, close, &spans, false, |_, i| {
                    text(item.names[i].name.as_str())
                }));
                out.push(text(" = "));
            }
            let path: Vec<&str> = item.path.iter().map(|p| p.name.as_str()).collect();
            out.push(text(path.join("/")));
            cat(out)
        });
        match style {
            HeaderStyle::Keyword => cat(vec![text("import "), list]),
            HeaderStyle::Group => list,
        }
    }

    fn export(&mut self, paren: u32, style: HeaderStyle, items: &[ExportItem]) -> Doc {
        let close = self.closer(paren);
        let spans: Vec<_> = items.iter().map(|i| (i.span.start, i.span.end)).collect();
        let list = self.seq("(", ")", paren, close, &spans, true, |_, i| {
            let e = &items[i];
            text(format!(
                "{}{}",
                e.name.name,
                if e.constructors { "(..)" } else { "" }
            ))
        });
        match style {
            HeaderStyle::Keyword => cat(vec![text("export "), list]),
            HeaderStyle::Group => list,
        }
    }

    fn item(&mut self, item: &Item) -> Doc {
        let mut out = Vec::new();
        for a in &item.attributes {
            out.push(self.leading(a.span.start));
            match &a.module {
                Some(m) => out.push(text(format!("@{}.{}", m.name, a.name.name))),
                None => out.push(text(format!("@{}", a.name.name))),
            }
            if let Some((args, span)) = &a.call {
                out.push(self.args(span.start, span.end - 1, args));
            } else if !a.args.is_empty() {
                let open = self.tok_after(a.name.span.end).span.start;
                let close = self.closer(open);
                let spans: Vec<_> = a.args.iter().map(|(_, s)| (s.start, s.end)).collect();
                let args = self.seq("(", ")", open, close, &spans, false, |f, i| {
                    text(f.slice(a.args[i].1))
                });
                out.push(args);
            }
            let next = match &item.kind {
                ItemKind::TypeDef(def) => def.name.span.start,
                ItemKind::Binding(b) => b.span.start,
                ItemKind::Declaration { name, .. } => name.span.start,
                ItemKind::MacroItem { module, name, .. } => {
                    module.as_ref().unwrap_or(name).span.start
                }
            };
            if let Some(t) = self.trailing(a.span.end, next) {
                out.push(t);
            }
            out.push(self.hard());
        }
        match &item.kind {
            ItemKind::TypeDef(def) => out.push(self.type_def(def, item.span.end)),
            ItemKind::Binding(b) => {
                out.push(self.leading(b.span.start));
                out.push(self.binding(b, true));
            }
            ItemKind::Declaration {
                name,
                type_params,
                ty,
            } => {
                out.push(self.leading(name.span.start));
                out.push(text(name.name.as_str()));
                out.push(if let Some(params) = self.plan.params.get(&name.span) {
                    text(format!("({})", params.join(", ")))
                } else {
                    self.type_params(name, type_params)
                });
                out.push(text(": "));
                out.push(self.ty(ty));
            }
            ItemKind::MacroItem { module, name, args } => {
                let head = module.as_ref().unwrap_or(name);
                out.push(self.leading(head.span.start));
                match module {
                    Some(m) => out.push(text(format!("{}.{}", m.name, name.name))),
                    None => out.push(text(name.name.as_str())),
                }
                let open = self.tok_after(name.span.end).span.start;
                let close = self.closer(open);
                out.push(self.args(open, close, args));
            }
        }
        cat(out)
    }

    /// `(A, B)` after the name `name`, or nothing.
    fn type_params(&mut self, name: &Ident, params: &[Ident]) -> Doc {
        if params.is_empty() {
            return cat(vec![]);
        }
        let open = self.tok_after(name.span.end).span.start;
        let close = self.closer(open);
        let spans: Vec<_> = params.iter().map(|n| (n.span.start, n.span.end)).collect();
        self.seq("(", ")", open, close, &spans, false, |_, i| {
            text(params[i].name.as_str())
        })
    }

    /// The arguments of a call, from the `(` at `open` to the `)` at `close`.
    fn args(&mut self, open: u32, close: u32, args: &[Arg]) -> Doc {
        let spans: Vec<_> = args
            .iter()
            .map(|a| {
                let start = a
                    .label
                    .as_ref()
                    .map_or(a.value.span.start, |l| l.span.start);
                (start, a.value.span.end)
            })
            .collect();
        self.seq("(", ")", open, close, &spans, false, |f, i| {
            let arg = &args[i];
            match &arg.label {
                Some(l) => cat(vec![f.label(&l.name), f.expr(&arg.value, 0)]),
                None => f.expr(&arg.value, 0),
            }
        })
    }

    fn type_def(&mut self, def: &TypeDef, end: u32) -> Doc {
        let mut out = vec![
            self.leading(def.name.span.start),
            text(def.name.name.as_str()),
        ];
        let mut after = def.name.span.end;
        if !def.head.is_empty() {
            let open = self.tok_after(after).span.start;
            let close = self.closer(open);
            out.push(self.types(open, close, &def.head));
            after = close + 1;
        } else if !def.params.is_empty() {
            let open = self.tok_after(after).span.start;
            let close = self.closer(open);
            let spans: Vec<_> = def
                .params
                .iter()
                .map(|n| (n.span.start, n.span.end))
                .collect();
            out.push(self.seq("(", ")", open, close, &spans, false, |f, i| {
                text(
                    f.plan
                        .renames
                        .get(&def.params[i].span)
                        .unwrap_or(&def.params[i].name)
                        .clone(),
                )
            }));
            after = close + 1;
        }
        match &def.body {
            TypeBody::Alias(t) => {
                out.push(text(" = "));
                out.push(self.ty(t));
            }
            TypeBody::Custom(cs) => {
                let open = self.tok_after(after).span.start;
                let close = end - 1;
                let spans: Vec<_> = cs.iter().map(|c| (c.span.start, c.span.end)).collect();
                out.push(text(" "));
                out.push(self.braced_lines(open, close, &spans, |f, i| f.constructor(&cs[i])));
            }
        }
        cat(out)
    }

    fn constructor(&mut self, c: &Constructor) -> Doc {
        let name = text(c.name.name.as_str());
        if c.fields.is_empty() {
            return name;
        }
        let open = self.tok_after(c.name.span.end).span.start;
        let close = c.span.end - 1;
        let spans: Vec<_> = c
            .fields
            .iter()
            .map(|f| (f.span.start, f.span.end))
            .collect();
        let fields = self.seq("(", ")", open, close, &spans, false, |f, i| {
            let field = &c.fields[i];
            match &field.label {
                Some(l) => cat(vec![f.label(&l.name), f.ty(&field.ty)]),
                None => f.ty(&field.ty),
            }
        });
        cat(vec![name, fields])
    }

    fn binding(&mut self, b: &Binding, top: bool) -> Doc {
        if self.plan.splits.contains(&b.span) {
            let (PatternKind::Tuple(ps), ExprKind::Tuple(es)) = (&b.pattern.kind, &b.value.kind)
            else {
                unreachable!()
            };
            let mut docs = Vec::new();
            for (p, value) in ps.iter().zip(es) {
                if !docs.is_empty() {
                    docs.push(Doc::Hard);
                    if top {
                        docs.push(Doc::Hard);
                    }
                }
                let params: Vec<_> = self.plan.params[&b.span]
                    .iter()
                    .filter(|n| {
                        self.plan.renames.iter().any(|(span, value_name)| {
                            *n == value_name
                                && ((p.span.start <= span.start && span.end <= p.span.end)
                                    || (value.span.start <= span.start
                                        && span.end <= value.span.end))
                        })
                    })
                    .cloned()
                    .collect();
                docs.push(self.binding_part(p, value, &params));
            }
            return cat(docs);
        }
        if let Some(params) = self.plan.params.get(&b.span) {
            return self.binding_part(&b.pattern, &b.value, params);
        }

        let head = match (&b.pattern.kind, b.type_params.is_empty()) {
            (PatternKind::Var(n), false) => {
                let name = Ident {
                    name: n.clone(),
                    span: b.pattern.span,
                };
                cat(vec![
                    self.leading(name.span.start),
                    text(n.as_str()),
                    self.type_params(&name, &b.type_params),
                ])
            }
            (PatternKind::Typed { binder, ty }, false) => {
                let name = Ident {
                    name: binder.name.clone(),
                    span: binder.span,
                };
                cat(vec![
                    self.leading(name.span.start),
                    text(binder.name.as_str()),
                    self.type_params(&name, &b.type_params),
                    text(": "),
                    self.ty(ty),
                ])
            }
            _ => self.pattern(&b.pattern, false),
        };
        cat(vec![head, text(" = "), self.expr(&b.value, 0)])
    }

    fn binding_part(&mut self, pattern: &Pattern, value: &Expr, params: &[String]) -> Doc {
        let (name, ty) = match &pattern.kind {
            PatternKind::Var(n) => (n.as_str(), None),
            PatternKind::Typed { binder, ty } => (binder.name.as_str(), Some(ty)),
            _ => unreachable!(),
        };
        let mut docs = vec![self.leading(pattern.span.start), text(name)];
        if !params.is_empty() {
            docs.push(text(format!("({})", params.join(", "))));
        }
        if let Some(ty) = ty {
            docs.push(text(": "));
            docs.push(self.ty(ty));
        }
        docs.push(text(" = "));
        docs.push(self.expr(value, 0));
        cat(docs)
    }

    fn stmt(&mut self, s: &Stmt) -> Doc {
        match s {
            Stmt::Bind(b) => self.binding(b, false),
            Stmt::Env { ty, value, span } => cat(vec![
                self.leading(span.start),
                text("! "),
                self.ty(ty),
                text(" = "),
                self.expr(value, 0),
            ]),
            Stmt::Expr(e) => self.expr(e, 0),
        }
    }

    fn stmt_span(s: &Stmt) -> (u32, u32) {
        let span = match s {
            Stmt::Bind(b) => b.span,
            Stmt::Env { span, .. } => *span,
            Stmt::Expr(e) => e.span,
        };
        (span.start, span.end)
    }

    /// A block with one expression stays on one line when it fits and has no
    /// line that must end in it. Other blocks have one statement per line.
    /// `hug` is set for a block at the end of what it is in: the body of a
    /// function, or the last operand of an operator, so that the line before
    /// it may end with `{`.
    fn block(&mut self, b: &Block, hug_it: bool) -> Doc {
        let close = b.span.end - 1;
        if let [Stmt::Expr(e)] = &b.stmts[..]
            && self.peek().is_none_or(|c| c.span.start >= close)
        {
            let before = self.hard;
            let body = self.expr(e, 0);
            if self.hard == before {
                let kind = if hug_it { Kind::Hug } else { Kind::Block };
                return group_of(
                    kind,
                    cat(vec![
                        text("{"),
                        nest(cat(vec![Doc::Line, body])),
                        Doc::Line,
                        text("}"),
                    ]),
                );
            }
            let (a, z) = (self.hard(), self.hard());
            return group(cat(vec![text("{"), nest(cat(vec![a, body])), z, text("}")]));
        }
        let spans: Vec<_> = b.stmts.iter().map(Self::stmt_span).collect();
        self.braced_lines(b.span.start, close, &spans, |f, i| f.stmt(&b.stmts[i]))
    }

    // ---- Expressions (§6) ----

    /// `e` where an expression of at least level `min` is needed (§6.9).
    fn expr(&mut self, e: &Expr, min: u8) -> Doc {
        if let Some(name) = self.plan.replacements.get(&e.span)
            && !self.printing_function.contains(&e.span)
        {
            return self.continued(e.span.start, |_| text(name.clone()));
        }
        if let Some(extract) = self.plan.extractions.get(&e.span)
            && !self.expanding.contains(&e.span)
        {
            self.expanding.insert(e.span);
            self.printing_function.insert(extract.function.span);
            let value = self.expr(&extract.function, 0);
            self.printing_function.remove(&extract.function.span);
            let rest = self.expr(e, 0);
            self.expanding.remove(&e.span);
            return cat(vec![
                text("{"),
                nest(cat(vec![
                    Doc::Hard,
                    text(format!("{} = ", extract.name)),
                    value,
                    Doc::Hard,
                    rest,
                ])),
                Doc::Hard,
                text("}"),
            ]);
        }

        self.continued(e.span.start, |f| {
            // A match is an operation at level 6 (§6.9).
            if level(e) < min || is_match(e) && min > 6 || self.plan.groups.contains(&e.span) {
                let before = f.hard;
                let body = f.expr_raw(e);
                if f.hard == before {
                    group_of(
                        Kind::Block,
                        cat(vec![
                            text("{"),
                            nest(cat(vec![Doc::Line, body])),
                            Doc::Line,
                            text("}"),
                        ]),
                    )
                } else {
                    let (a, z) = (f.hard(), f.hard());
                    group(cat(vec![text("{"), nest(cat(vec![a, body])), z, text("}")]))
                }
            } else {
                f.expr_raw(e)
            }
        })
    }

    fn expr_raw(&mut self, e: &Expr) -> Doc {
        let span = e.span;
        match &e.kind {
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::String(_) => text(self.slice(span)),
            ExprKind::Name { module, name } => match module {
                Some(m) => text(format!("{}.{}", m.name, name.name)),
                None => text(name.name.as_str()),
            },
            ExprKind::Tuple(items) => {
                let spans = spans_of(items);
                self.seq("(", ")", span.start, span.end - 1, &spans, false, |f, i| {
                    f.expr(&items[i], 0)
                })
            }
            ExprKind::List { items, rest } => {
                let mut spans = spans_of(items);
                if let Some(r) = rest {
                    spans.push((r.span.start, r.span.end));
                }
                self.seq(
                    "[",
                    "]",
                    span.start,
                    span.end - 1,
                    &spans,
                    false,
                    |f, i| match items.get(i) {
                        Some(e) => f.expr(e, 0),
                        None => cat(vec![text(".."), f.expr(rest.as_deref().unwrap(), 0)]),
                    },
                )
            }
            ExprKind::BitArray(segs) => {
                let spans: Vec<_> = segs.iter().map(|s| (s.span.start, s.span.end)).collect();
                self.seq(
                    "<<",
                    ">>",
                    span.start,
                    span.end - 2,
                    &spans,
                    false,
                    |f, i| {
                        let value = f.expr(&segs[i].value, 0);
                        cat(vec![value, f.format(&segs[i].format)])
                    },
                )
            }
            ExprKind::Block(b) => self.block(b, false),
            ExprKind::Fn(func) => self.function(func, span.start),
            ExprKind::Case {
                subject,
                clauses,
                syntax: Syntax::Old,
            } => {
                let subject_doc = self.expr(subject, 0);
                let open = self.tok_after(subject.span.end).span.start;
                let body = self.clause_block(open, span.end - 1, clauses, Syntax::New);
                cat(vec![subject_doc, text(" |> "), body])
            }
            ExprKind::Case {
                subject,
                clauses,
                syntax: Syntax::New,
            } => {
                // §6.9: `|>` is stronger than the operators of levels 1 to 5.
                let subject_doc = self.expr(subject, 6);
                let open = self.tok_after(subject.span.end);
                let open = self.tok_after(open.span.end).span.start;
                let body = self.clause_block(open, span.end - 1, clauses, Syntax::New);
                cat(vec![subject_doc, text(" |> "), body])
            }
            ExprKind::Clauses(clauses) => {
                self.clause_block(span.start, span.end - 1, clauses, Syntax::New)
            }
            ExprKind::Call { callee, args } => {
                let head = self.expr(callee, 10);
                let open = self.tok_after(callee.span.end).span.start;
                // A sole argument in brackets of its own hugs the call:
                // `Some((a, b))` breaks inside the tuple.
                if let [Arg { label: None, value }] = &args[..]
                    && matches!(value.kind, ExprKind::Tuple(_) | ExprKind::List { .. })
                    && self.peek().is_none_or(|c| c.span.start >= span.end)
                {
                    return cat(vec![head, text("("), self.expr(value, 0), text(")")]);
                }
                let spans: Vec<_> = args
                    .iter()
                    .map(|a| {
                        let start = a
                            .label
                            .as_ref()
                            .map_or(a.value.span.start, |l| l.span.start);
                        (start, a.value.span.end)
                    })
                    .collect();
                let list = self.seq("(", ")", open, span.end - 1, &spans, false, |f, i| {
                    let arg = &args[i];
                    match &arg.label {
                        Some(l) => cat(vec![f.label(&l.name), f.expr(&arg.value, 0)]),
                        None => f.expr(&arg.value, 0),
                    }
                });
                cat(vec![head, list])
            }
            ExprKind::Field { target, name } => {
                let target = self.target(target);
                let field = self.dotted(name.span.start, format!(".{}", name.name));
                cat(vec![target, field])
            }
            ExprKind::TupleIndex {
                target, index_span, ..
            } => {
                let target = self.target(target);
                let index = format!(".{}", self.slice(*index_span));
                let index = self.dotted(index_span.start, index);
                cat(vec![target, index])
            }
            ExprKind::RecordUpdate {
                constructor,
                base,
                fields,
            } => {
                let head = self.expr(constructor, 10);
                let open = self.tok_after(constructor.span.end).span.start;
                let mut spans = vec![(base.span.start, base.span.end)];
                spans.extend(fields.iter().map(|(l, v)| (l.span.start, v.span.end)));
                let list = self.seq("(", ")", open, span.end - 1, &spans, false, |f, i| {
                    if i == 0 {
                        return cat(vec![text(".."), f.expr(base, 0)]);
                    }
                    let (label, value) = &fields[i - 1];
                    cat(vec![f.label(&label.name), f.expr(value, 0)])
                });
                cat(vec![head, list])
            }
            ExprKind::Unary { op, expr } => cat(vec![
                text(match op {
                    UnOp::Neg => "-",
                    UnOp::NegFloat => "-.",
                }),
                self.expr(expr, 10),
            ]),
            ExprKind::Binary { op, .. } => self.binary(e, op.level()),
        }
    }

    /// A chain of operators of one level, all on one line, or broken after
    /// each operator, so that no line begins with `-` or `-.` (§1.7). A
    /// pipeline breaks before each `|>` instead, which goes on from the line
    /// before.
    fn binary(&mut self, e: &Expr, lvl: u8) -> Doc {
        let mut rest = Vec::new();
        let mut first = e;
        while let ExprKind::Binary { op, lhs, rhs } = &first.kind
            && op.level() == lvl
        {
            rest.push((*op, &**rhs));
            first = lhs;
        }
        rest.reverse();
        // After a pipeline, which breaks before each `|>`, the operators stay
        // on its last line: `xs |> f |> g == ys`.
        let glued = matches!(
            &first.kind,
            ExprKind::Binary {
                op: BinOp::Pipe,
                ..
            }
        );
        let head = Doc::Full(Box::new(self.expr(first, lvl)));
        let mut tail = Vec::new();
        let last = rest.len();
        for (i, (op, rhs)) in rest.into_iter().enumerate() {
            // A comment before the operand stays at the end of the line: before
            // `|>`, which begins the next line, and after other operators.
            let comments = self.mid_comments(rhs.span.start);
            if op == BinOp::Pipe {
                if let Some(c) = comments {
                    tail.push(c);
                    tail.push(self.force());
                }
                tail.push(Doc::Line);
                tail.push(text("|> "));
            } else if let Some(c) = comments {
                tail.push(text(format!(" {}", op.text())));
                tail.push(c);
                tail.push(self.hard());
            } else if glued {
                tail.push(text(format!(" {} ", op.text())));
            } else {
                tail.push(text(format!(" {}", op.text())));
                tail.push(Doc::Line);
            }
            match &rhs.kind {
                ExprKind::Block(b) if i + 1 == last => tail.push(self.block(b, true)),
                _ if i + 1 < last => tail.push(Doc::Full(Box::new(self.expr(rhs, lvl + 1)))),
                _ => tail.push(self.expr(rhs, lvl + 1)),
            }
        }
        group(cat(vec![head, nest(cat(tail))]))
    }

    /// `.name` or `.k`, whose name is at `pos`. A comment before it stays on
    /// the line of the target, and the `.` goes on in the next line, which
    /// is not a break (§1.7).
    fn dotted(&mut self, pos: u32, part: String) -> Doc {
        match self.mid_comments(pos) {
            Some(comments) => {
                let end = self.hard();
                nest(cat(vec![comments, end, text(part)]))
            }
            None => text(part),
        }
    }

    /// The expression before `.name` or `.k`. An int right before `.` would
    /// be read as a float, so a space comes between them.
    fn target(&mut self, e: &Expr) -> Doc {
        let d = self.expr(e, 10);
        if matches!(e.kind, ExprKind::Int(_)) {
            cat(vec![d, text(" ")])
        } else {
            d
        }
    }

    fn format(&mut self, format: &Option<Format>) -> Doc {
        let Some(f) = format else {
            return cat(vec![]);
        };
        let mut out = vec![text(format!(":{}", f.name.name))];
        if let Some(arg) = &f.arg {
            out.push(text("("));
            out.push(self.expr(arg, 0));
            out.push(text(")"));
        }
        cat(out)
    }

    /// `(params) -> A ? E ! N { body }`; the `{` stays on the line of the
    /// `)` or of the type before it (§1.7).
    fn function(&mut self, f: &Function, open: u32) -> Doc {
        let close = self.closer(open);
        let spans: Vec<_> = f
            .params
            .iter()
            .map(|p| match p {
                Param::Pattern(p) => (p.span.start, p.span.end),
                Param::Env { name, ty } => (name.span.start, ty.span.end),
            })
            .collect();
        let params = self.seq("(", ")", open, close, &spans, false, |fm, i| {
            match &f.params[i] {
                Param::Pattern(p) => fm.pattern(p, false),
                Param::Env { name, ty } => cat(vec![text(format!("{} ! ", name.name)), fm.ty(ty)]),
            }
        });
        let mut out = vec![params];
        if let Some(ret) = &f.ret {
            out.push(text(" -> "));
            let saved = std::mem::replace(&mut self.one_line, true);
            out.push(self.arrow_tail(&ret.value, ret.err.as_ref(), ret.env.as_ref(), true));
            self.one_line = saved;
        }
        out.push(text(" "));
        out.push(self.block(&f.body, true));
        cat(out)
    }

    /// The clauses of a `case` or a clause block, from the `{` at `open` to
    /// the `}` at `close`, one clause per line.
    fn clause_block(&mut self, open: u32, close: u32, clauses: &[Clause], syntax: Syntax) -> Doc {
        let spans: Vec<_> = clauses.iter().map(|c| (c.span.start, c.span.end)).collect();
        let split =
            clauses.len() == 1 && self.plan.choices.iter().any(|span| span.end == close + 1);
        self.braced_lines(open, close, &spans, |f, i| {
            if split {
                let c = &clauses[i];
                let mut docs = Vec::new();
                for p in &c.patterns {
                    if !docs.is_empty() {
                        docs.push(Doc::Hard);
                    }
                    docs.push(f.pattern(p, true));
                    docs.push(Doc::Tab(Align::Arrow));
                    docs.push(text("-> "));
                    docs.push(f.expr(&c.body, 0));
                }
                cat(docs)
            } else {
                f.clause(&clauses[i], syntax)
            }
        })
    }

    /// A clause. Its patterns are not broken inside; when they do not fit
    /// on one line, each choice after the first begins a line with `|`,
    /// which goes on from the line before (§1.7).
    fn clause(&mut self, c: &Clause, syntax: Syntax) -> Doc {
        let saved = std::mem::replace(&mut self.one_line, true);
        let guard = syntax == Syntax::New && c.guard.is_some();
        self.fn_next = c.patterns.len() > 1 || guard;
        let first = self.pattern(&c.patterns[0], true);
        let mut more = Vec::new();
        for (i, p) in c.patterns.iter().enumerate().skip(1) {
            // A comment before a choice stays at the end of the line before
            // its `|`.
            if let Some(comments) = self.mid_comments(p.span.start) {
                more.push(comments);
                more.push(self.force());
            }
            more.push(Doc::Line);
            more.push(text("| "));
            self.fn_next = i + 1 < c.patterns.len() || guard;
            more.push(self.pattern(p, true));
        }
        self.fn_next = false;
        self.one_line = saved;
        let mut out = vec![group(cat(vec![first, nest(cat(more))]))];
        if let Some(g) = &c.guard {
            out.push(text(match syntax {
                Syntax::Old => " if ",
                Syntax::New => " ? ",
            }));
            out.push(self.expr(g, 0));
        }
        out.push(Doc::Tab(Align::Arrow));
        out.push(text("-> "));
        out.push(self.expr(&c.body, 0));
        cat(out)
    }

    // ---- Patterns (§7) ----

    /// The source of the literal token that ends at `end`.
    fn literal_before(&self, end: u32) -> &'a str {
        let i = self.tok_index(end) - 1;
        self.slice(self.toks[i].span)
    }

    /// `clause` is set for the top of a `case` clause pattern, where `->`
    /// comes after it.
    fn pattern(&mut self, p: &Pattern, clause: bool) -> Doc {
        self.continued(p.span.start, |f| f.pattern_raw(p, clause))
    }

    fn pattern_raw(&mut self, p: &Pattern, clause: bool) -> Doc {
        let span = p.span;
        match &p.kind {
            PatternKind::Var(n) | PatternKind::Discard(n) => text(n.as_str()),
            PatternKind::Typed { binder, ty } => {
                let t = if clause && ends_with_tuple(ty) {
                    cat(vec![text("{ "), self.ty(ty), text(" }")])
                } else if clause && self.fn_next {
                    // §7.1: a function type before `?` or `|` is put in `{ }`.
                    self.ty_in(ty, 1, false)
                } else {
                    self.ty(ty)
                };
                cat(vec![text(format!("{}: ", binder.name)), t])
            }
            PatternKind::Named { name, pattern } => cat(vec![
                text(format!("{} = ", name.name)),
                self.pattern(pattern, clause),
            ]),
            PatternKind::Int { negative, .. } => text(format!(
                "{}{}",
                if *negative { "-" } else { "" },
                self.literal_before(span.end)
            )),
            PatternKind::Float { negative, .. } => text(format!(
                "{}{}",
                if *negative { "-." } else { "" },
                self.literal_before(span.end)
            )),
            PatternKind::String(_) => text(self.slice(span)),
            PatternKind::Prefix { rest, .. } => {
                let prefix = self.slice(self.toks[self.tok_index(span.start)].span);
                text(format!("{prefix} <> {}", rest.name))
            }
            PatternKind::Constructor {
                module,
                name,
                args,
                spread,
            } => {
                let head = match module {
                    Some(m) => text(format!("{}.{}", m.name, name.name)),
                    None => text(name.name.as_str()),
                };
                if args.is_empty() && !*spread {
                    head
                } else {
                    let open = self.tok_after(name.span.end).span.start;
                    let close = span.end - 1;
                    let mut spans: Vec<_> =
                        args.iter().map(|a| (a.span.start, a.span.end)).collect();
                    if *spread {
                        let after = args.last().map_or(open + 1, |a| a.span.end);
                        let dots = self.tok_after(after).span;
                        spans.push((dots.start, dots.end));
                    }
                    let list = self.seq("(", ")", open, close, &spans, false, |f, i| {
                        match args.get(i) {
                            Some(a) => f.pattern(a, false),
                            None => text(".."),
                        }
                    });
                    cat(vec![head, list])
                }
            }
            PatternKind::Tuple(items) => {
                let spans: Vec<_> = items.iter().map(|a| (a.span.start, a.span.end)).collect();
                self.seq("(", ")", span.start, span.end - 1, &spans, false, |f, i| {
                    f.pattern(&items[i], false)
                })
            }
            PatternKind::List { items, rest } => {
                let mut spans: Vec<_> = items.iter().map(|a| (a.span.start, a.span.end)).collect();
                if rest.is_some() {
                    let after = items.last().map_or(span.start + 1, |a| a.span.end);
                    let dots = self.tok_after(after).span;
                    let end = match rest {
                        Some(Some(b)) => b.span.end,
                        _ => dots.end,
                    };
                    spans.push((dots.start, end));
                }
                self.seq(
                    "[",
                    "]",
                    span.start,
                    span.end - 1,
                    &spans,
                    false,
                    |f, i| match items.get(i) {
                        Some(a) => f.pattern(a, false),
                        None => match rest {
                            Some(Some(b)) => text(format!("..{}", b.name)),
                            _ => text(".."),
                        },
                    },
                )
            }
            PatternKind::BitArray(segs) => {
                let spans: Vec<_> = segs.iter().map(|s| (s.span.start, s.span.end)).collect();
                self.seq(
                    "<<",
                    ">>",
                    span.start,
                    span.end - 2,
                    &spans,
                    false,
                    |f, i| {
                        let value = f.pattern(&segs[i].value, false);
                        cat(vec![value, f.format(&segs[i].format)])
                    },
                )
            }
        }
    }

    // ---- Types (§3) ----

    fn ty(&mut self, t: &TypeExpr) -> Doc {
        self.ty_in(t, 1, true)
    }

    /// `t` where a type of strength `min` is needed. `tail` is set when
    /// nothing that a function type could take comes after it.
    fn ty_in(&mut self, t: &TypeExpr, min: u8, tail: bool) -> Doc {
        let brace = type_braced(t, min) || !tail && ends_with_fn(t);
        self.continued(t.span.start, |f| {
            let body = f.ty_raw(t, tail || brace);
            if brace {
                cat(vec![text("{ "), body, text(" }")])
            } else {
                body
            }
        })
    }

    fn types(&mut self, open: u32, close: u32, items: &[TypeExpr]) -> Doc {
        let spans: Vec<_> = items.iter().map(|a| (a.span.start, a.span.end)).collect();
        self.seq("(", ")", open, close, &spans, false, |f, i| f.ty(&items[i]))
    }

    fn ty_raw(&mut self, t: &TypeExpr, tail: bool) -> Doc {
        let (members, sep) = match &t.kind {
            TypeKind::Named { module, name, args } => {
                let head = match module {
                    Some(m) => text(format!("{}.{}", m.name, name.name)),
                    None => text(name.name.as_str()),
                };
                if args.is_empty() {
                    return head;
                }
                let open = self.tok_after(name.span.end).span.start;
                let list = self.types(open, t.span.end - 1, args);
                return cat(vec![head, list]);
            }
            TypeKind::Var(n) => return text(self.plan.renames.get(&t.span).unwrap_or(n).clone()),
            TypeKind::ValueOf { module, name } => {
                return match module {
                    Some(m) => text(format!("{}.{}", m.name, name.name)),
                    None => text(name.name.as_str()),
                };
            }
            TypeKind::Call { module, name, args } => {
                let head = match module {
                    Some(m) => text(format!("{}.{}", m.name, name.name)),
                    None => text(name.name.as_str()),
                };
                let open = self.tok_after(name.span.end).span.start;
                let spans: Vec<_> = args
                    .iter()
                    .map(|a| {
                        let span = match a {
                            TypeArg::Type(t) => t.span,
                            TypeArg::Int(_, s) | TypeArg::String(_, s) => *s,
                        };
                        (span.start, span.end)
                    })
                    .collect();
                let list =
                    self.seq(
                        "(",
                        ")",
                        open,
                        t.span.end - 1,
                        &spans,
                        false,
                        |f, i| match &args[i] {
                            TypeArg::Type(t) => f.ty(t),
                            TypeArg::Int(_, s) | TypeArg::String(_, s) => text(f.slice(*s)),
                        },
                    );
                return cat(vec![head, list]);
            }
            TypeKind::Hole => return text("_"),
            TypeKind::Tuple(items) => return self.types(t.span.start, t.span.end - 1, items),
            TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => {
                let close = self.closer(t.span.start);
                let spans: Vec<_> = params
                    .iter()
                    .map(|p| {
                        let start = p.label.as_ref().map_or(p.ty.span.start, |l| l.span.start);
                        (start, p.ty.span.end)
                    })
                    .collect();
                let list = self.seq("(", ")", t.span.start, close, &spans, false, |f, i| {
                    let p = &params[i];
                    match &p.label {
                        Some(l) => cat(vec![f.label(&l.name), f.ty(&p.ty)]),
                        None => f.ty(&p.ty),
                    }
                });
                let tail_doc = self.arrow_tail(ret, err.as_deref(), env.as_deref(), tail);
                return cat(vec![list, text(" -> "), tail_doc]);
            }
            TypeKind::Union(v) => (v, " | "),
            TypeKind::Inter(v) => (v, " & "),
            TypeKind::Diff(v) => (v, " - "),
        };
        let min = type_prec(t) + 1;
        let mut out = Vec::new();
        for (i, m) in members.iter().enumerate() {
            if i > 0 {
                out.push(text(sep));
            }
            out.push(self.ty_in(m, min, tail && i + 1 == members.len()));
        }
        cat(out)
    }

    fn arrow_tail(
        &mut self,
        ret: &TypeExpr,
        err: Option<&TypeExpr>,
        env: Option<&TypeExpr>,
        tail: bool,
    ) -> Doc {
        let mut out = vec![self.ty_in(ret, 1, tail && err.is_none() && env.is_none())];
        if let Some(err) = err {
            out.push(text(" ? "));
            out.push(self.ty_in(err, 1, tail && env.is_none()));
        }
        if let Some(env) = env {
            out.push(text(" ! "));
            out.push(self.ty_in(env, 1, tail));
        }
        cat(out)
    }
}
