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
//! their line, and other comments on their own lines.

use crate::ast::*;
use crate::lexer::{Comment, lex};
use crate::print::{has_open_fn, level, type_prec};
use crate::span::Span;
use crate::token::{Token, TokenKind as T};
use crate::{Diagnostic, parse_module};

/// The width that the layout tries to keep to.
pub const WIDTH: usize = 80;

/// The columns that a tab counts for when lines are measured.
pub const TAB_WIDTH: usize = 4;

/// Formats a source file. A file with syntax errors is not formatted; its
/// errors are given back.
pub fn format(source: &str) -> Result<String, Vec<Diagnostic>> {
    let (module, diags) = parse_module(source);
    if !diags.is_empty() {
        return Err(diags);
    }
    let lexed = lex(source);
    let mut f = Formatter {
        src: source,
        toks: &lexed.tokens,
        close: &lexed.close,
        comments: &lexed.comments,
        next: 0,
        hard: 0,
        one_line: false,
    };
    let doc = f.module(&module);
    debug_assert_eq!(f.next, f.comments.len(), "every comment is printed");
    Ok(render(&doc, WIDTH))
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
            Doc::Tab(_) => rem -= 1,
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
                prev = self.comments[self.next - 1].span.end;
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
            Import(u32, &'m [ImportItem]),
            Export(u32, &'m [ExportItem]),
            Item(&'m Item),
        }
        let mut things = Vec::new();
        let mut spans = Vec::new();
        let mut i = 0;
        for (kw, is_import) in [(T::Import, true), (T::Export, false)] {
            if self.toks[i].kind != kw {
                continue;
            }
            let start = self.toks[i].span.start;
            let paren = self.toks[i + 1].span.start;
            let close = self.closer(paren);
            spans.push((start, close + 1));
            things.push(if is_import {
                Thing::Import(paren, &m.imports)
            } else {
                Thing::Export(paren, &m.exports)
            });
            i = self.tok_index(close + 1);
        }
        for item in &m.items {
            spans.push((item.span.start, item.span.end));
            things.push(Thing::Item(item));
        }
        let end = self.src.len() as u32;
        let mut lines = self.lines(None, end, &spans, Blank::Top, |f, i| match &things[i] {
            Thing::Import(paren, items) => f.import(*paren, items),
            Thing::Export(paren, items) => f.export(*paren, items),
            Thing::Item(item) => f.item(item),
        });
        lines.push(Doc::Hard);
        cat(lines)
    }

    fn import(&mut self, paren: u32, items: &[ImportItem]) -> Doc {
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
        cat(vec![text("import "), list])
    }

    fn export(&mut self, paren: u32, items: &[ExportItem]) -> Doc {
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
        cat(vec![text("export "), list])
    }

    fn item(&mut self, item: &Item) -> Doc {
        let mut out = Vec::new();
        for a in &item.attributes {
            out.push(self.leading(a.span.start));
            out.push(text(format!("@{}", a.name.name)));
            if !a.args.is_empty() {
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
            };
            if let Some(t) = self.trailing(a.span.end, next) {
                out.push(t);
            }
            out.push(self.hard());
        }
        match &item.kind {
            ItemKind::TypeDef(def) => out.push(self.type_def(def, item.span.end)),
            ItemKind::Binding(b) => out.push(self.binding(b)),
            ItemKind::Declaration { name, ty } => {
                out.push(self.leading(name.span.start));
                out.push(text(format!("{}: ", name.name)));
                out.push(self.ty(ty));
            }
        }
        cat(out)
    }

    fn type_def(&mut self, def: &TypeDef, end: u32) -> Doc {
        let mut out = vec![
            self.leading(def.name.span.start),
            text(def.name.name.as_str()),
        ];
        let mut after = def.name.span.end;
        if !def.params.is_empty() {
            let open = self.tok_after(after).span.start;
            let close = self.closer(open);
            let spans: Vec<_> = def
                .params
                .iter()
                .map(|n| (n.span.start, n.span.end))
                .collect();
            out.push(self.seq("(", ")", open, close, &spans, false, |_, i| {
                text(def.params[i].name.as_str())
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

    fn binding(&mut self, b: &Binding) -> Doc {
        cat(vec![
            self.pattern(&b.pattern, false),
            text(" = "),
            self.expr(&b.value, 0),
        ])
    }

    fn stmt(&mut self, s: &Stmt) -> Doc {
        match s {
            Stmt::Bind(b) => self.binding(b),
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
        let lead = self.leading(e.span.start);
        let body = if level(e) < min {
            cat(vec![text("{ "), self.expr_raw(e), text(" }")])
        } else {
            self.expr_raw(e)
        };
        cat(vec![lead, body])
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
            ExprKind::Case { subject, clauses } => self.case(subject, clauses, span),
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
                cat(vec![self.target(target), text(format!(".{}", name.name))])
            }
            ExprKind::TupleIndex {
                target, index_span, ..
            } => cat(vec![
                self.target(target),
                text(format!(".{}", self.slice(*index_span))),
            ]),
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
            if op == BinOp::Pipe {
                tail.push(Doc::Line);
                tail.push(text("|> "));
            } else if glued {
                tail.push(text(format!(" {} ", op.text())));
            } else {
                tail.push(text(format!(" {}", op.text())));
                tail.push(Doc::Line);
            }
            match &rhs.kind {
                ExprKind::Block(b) if i + 1 == last => {
                    tail.push(self.leading(rhs.span.start));
                    tail.push(self.block(b, true));
                }
                _ if i + 1 < last => tail.push(Doc::Full(Box::new(self.expr(rhs, lvl + 1)))),
                _ => tail.push(self.expr(rhs, lvl + 1)),
            }
        }
        group(cat(vec![head, nest(cat(tail))]))
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

    /// `case e { clauses }`, with one clause per line.
    fn case(&mut self, subject: &Expr, clauses: &[Clause], span: Span) -> Doc {
        let subject_doc = if has_open_fn(subject) {
            cat(vec![text("{ "), self.expr(subject, 0), text(" }")])
        } else {
            self.expr(subject, 0)
        };
        let open = self.tok_after(subject.span.end).span.start;
        let spans: Vec<_> = clauses.iter().map(|c| (c.span.start, c.span.end)).collect();
        let body = self.braced_lines(open, span.end - 1, &spans, |f, i| f.clause(&clauses[i]));
        cat(vec![text("case "), subject_doc, text(" "), body])
    }

    /// A clause. Its patterns are not broken inside; when they do not fit
    /// on one line, each choice after the first begins a line with `|`,
    /// which goes on from the line before (§1.7).
    fn clause(&mut self, c: &Clause) -> Doc {
        let saved = std::mem::replace(&mut self.one_line, true);
        let first = self.pattern(&c.patterns[0], true);
        let mut more = Vec::new();
        for p in &c.patterns[1..] {
            more.push(Doc::Line);
            more.push(text("| "));
            more.push(self.pattern(p, true));
        }
        self.one_line = saved;
        let mut out = vec![group(cat(vec![first, nest(cat(more))]))];
        if let Some(g) = &c.guard {
            out.push(text(" if "));
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
        let lead = self.leading(p.span.start);
        let span = p.span;
        let body = match &p.kind {
            PatternKind::Var(n) | PatternKind::Discard(n) => text(n.as_str()),
            PatternKind::Typed { binder, ty } => {
                let t = if clause && ends_with_tuple(ty) {
                    cat(vec![text("{ "), self.ty(ty), text(" }")])
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
        };
        cat(vec![lead, body])
    }

    // ---- Types (§3) ----

    fn ty(&mut self, t: &TypeExpr) -> Doc {
        self.ty_in(t, 1, true)
    }

    /// `t` where a type of strength `min` is needed. `tail` is set when
    /// nothing that a function type could take comes after it.
    fn ty_in(&mut self, t: &TypeExpr, min: u8, tail: bool) -> Doc {
        let lead = self.leading(t.span.start);
        let brace = type_braced(t, min) || !tail && ends_with_fn(t);
        let body = self.ty_raw(t, tail || brace);
        if brace {
            cat(vec![lead, text("{ "), body, text(" }")])
        } else {
            cat(vec![lead, body])
        }
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
            TypeKind::Var(n) => return text(n.as_str()),
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
