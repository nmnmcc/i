//! Source-preserving layout of the Lisp surface, by Lisp convention.
//!
//! Indentation follows cljfmt (and Emacs `calculate-lisp-indent`, whose rules
//! cljfmt reproduces): closing brackets are gathered at the end of the last
//! line of a form, a call whose first argument stays on the head line aligns
//! its other arguments under that argument, a call with nothing after the head
//! indents its arguments one space (under the head), and the body of a
//! block-like form (`:block`/`defun` style) is indented two spaces. Where
//! to break is chosen as zprint does: a form that fits stays on one line,
//! otherwise the "hang" layout is tried before the "flow" layout and the one
//! that overflows less (then uses fewer lines) is kept.
//!
//! Comments belong to gaps between source datums, not to regenerated nodes.
use super::{Datum, DatumKind as K};
use crate::Diagnostic;
use crate::format::WIDTH;
use crate::lexer::{Comment, lex_lisp};
use crate::token::Token;
use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn format(source: &str) -> Result<String, Vec<Diagnostic>> {
    let document = super::parse(source)?;
    let lexed = lex_lisp(source);
    let mut builder = Builder {
        source,
        comments: &lexed.comments,
        tokens: &lexed.tokens,
        next: 0,
        nodes: Vec::new(),
    };
    let mut top = Vec::new();
    let mut end = 0;
    for datum in document.iter() {
        builder.gap(end, datum.span.start, true, &mut top);
        let id = builder.datum(datum);
        top.push(Item::Datum(id));
        end = datum.span.end;
    }
    builder.gap(end, source.len() as u32, true, &mut top);
    assert_eq!(
        builder.next,
        lexed.comments.len(),
        "every Lisp comment is printed"
    );
    let mut printer = Printer {
        nodes: &builder.nodes,
        memo: HashMap::new(),
    };
    let mut out = String::new();
    let mut col = None::<usize>;
    for item in &top {
        match item {
            Item::Blank => {
                if col.take().is_some() {
                    out.push('\n');
                }
                out.push('\n');
            }
            Item::Comment { text, trailing } => {
                match col.take() {
                    Some(c) if *trailing && c + 1 + width(text) <= WIDTH => out.push(' '),
                    Some(_) => out.push('\n'),
                    None => {}
                }
                out.push_str(text);
                out.push('\n');
            }
            Item::Datum(id) => {
                if col.take().is_some() {
                    out.push('\n');
                }
                let layout = printer.layout(*id, 0, 0);
                out.push_str(&layout.text);
                col = Some(layout.last_col);
            }
        }
    }
    if col.is_some() {
        out.push('\n');
    }
    Ok(out)
}

fn width(s: &str) -> usize {
    s.chars().count()
}

enum Item {
    Datum(usize),
    Comment { text: String, trailing: bool },
    /// One kept blank line between top-level items.
    Blank,
}

enum Part {
    Text(&'static str),
    Comment(String),
}

enum Node {
    Atom(String),
    /// Quote, unquote or splice prefix tokens with any comments among them.
    Prefix(Vec<Part>, usize),
    Seq {
        open: &'static str,
        close: &'static str,
        items: Vec<Item>,
    },
}

struct Builder<'a> {
    source: &'a str,
    comments: &'a [Comment],
    tokens: &'a [Token],
    next: usize,
    /// Each node with its one-line spelling, if it has no comment.
    nodes: Vec<(Node, Option<String>)>,
}

impl Builder<'_> {
    fn comment_text(comment: &Comment) -> String {
        if comment.text.starts_with("///") {
            comment.text.clone()
        } else {
            comment.text.trim_end().to_owned()
        }
    }

    /// The comments in exactly this source gap. A comment is trailing when no
    /// line ends between it and the token before it.
    fn gap(&mut self, from: u32, to: u32, top: bool, items: &mut Vec<Item>) {
        let mut end = from;
        while let Some(comment) = self.comments.get(self.next).filter(|c| c.span.start < to) {
            assert!(comment.span.start >= end, "comments follow datum order");
            let before = &self.source[end as usize..comment.span.start as usize];
            if top && end != 0 && before.matches('\n').count() >= 2 {
                items.push(Item::Blank);
            }
            items.push(Item::Comment {
                text: Self::comment_text(comment),
                trailing: end != 0 && !before.contains(['\n', '\r']),
            });
            end = comment.span.end;
            self.next += 1;
        }
        let whitespace = &self.source[end as usize..to as usize];
        if top && end != 0 && to as usize != self.source.len() && whitespace.matches('\n').count() >= 2 {
            items.push(Item::Blank);
        }
    }

    fn prefix_gap(&mut self, from: u32, to: u32, parts: &mut Vec<Part>) {
        while let Some(comment) = self.comments.get(self.next).filter(|c| c.span.start < to) {
            assert!(comment.span.start >= from, "comments follow datum order");
            parts.push(Part::Comment(Self::comment_text(comment)));
            self.next += 1;
        }
    }

    fn push(&mut self, node: Node) -> usize {
        let flat = match &node {
            Node::Atom(s) => Some(s.clone()),
            Node::Prefix(parts, child) => {
                let mut s = String::new();
                for part in parts {
                    match part {
                        Part::Text(t) => s.push_str(t),
                        Part::Comment(_) => return self.store(node, None),
                    }
                }
                self.nodes[*child].1.as_ref().map(|c| s + c)
            }
            Node::Seq { open, close, items } => {
                let mut parts = Vec::new();
                for item in items {
                    match item {
                        Item::Datum(id) => parts.push(self.nodes[*id].1.clone()),
                        _ => parts.push(None),
                    }
                }
                parts
                    .into_iter()
                    .collect::<Option<Vec<_>>>()
                    .map(|p| format!("{open}{}{close}", p.join(" ")))
            }
        };
        self.store(node, flat)
    }

    fn store(&mut self, node: Node, flat: Option<String>) -> usize {
        self.nodes.push((node, flat));
        self.nodes.len() - 1
    }

    fn datum(&mut self, datum: &Datum) -> usize {
        let (open, close, xs) = match &datum.kind {
            K::Quote(d) | K::Unquote(d) | K::Splice(d) => {
                let mut parts = vec![Part::Text(if matches!(datum.kind, K::Quote(_)) {
                    "`"
                } else {
                    "~"
                })];
                let mut end = datum.span.start + 1;
                if matches!(datum.kind, K::Splice(_)) {
                    let dot = &self.tokens[self
                        .tokens
                        .partition_point(|t| t.span.start <= datum.span.start)];
                    self.prefix_gap(end, dot.span.start, &mut parts);
                    parts.push(Part::Text(".."));
                    end = dot.span.end;
                }
                self.prefix_gap(end, d.span.start, &mut parts);
                let child = self.datum(d);
                return self.push(Node::Prefix(parts, child));
            }
            K::Round(xs) => ("(", ")", xs),
            K::Square(xs) => ("[", "]", xs),
            K::Curly(xs) => ("{", "}", xs),
            K::Bits(xs) => ("<<", ">>", xs),
            _ => {
                let text = &self.source[datum.span.start as usize..datum.span.end as usize];
                return self.push(Node::Atom(text.to_owned()));
            }
        };
        let mut items = Vec::new();
        let mut end = datum.span.start + open.len() as u32;
        for child in xs {
            self.gap(end, child.span.start, false, &mut items);
            let id = self.datum(child);
            items.push(Item::Datum(id));
            end = child.span.end;
        }
        self.gap(end, datum.span.end - close.len() as u32, false, &mut items);
        self.push(Node::Seq { open, close, items })
    }
}

struct Layout {
    /// Starts at the given column; later lines carry their own indentation.
    text: String,
    last_col: usize,
    lines: usize,
    overflow: usize,
}

/// Where the parts of a broken form go: the first `inline` datums share the
/// opening line; datum `i` otherwise starts a line at `indent[min(i, len-1)]`.
struct Shape {
    inline: usize,
    indent: Vec<usize>,
}

/// The number of distinguished parts kept on the head line of a block-like
/// form; its other parts are its body. `->` has two (parameters and result)
/// as a function and one (the pattern or parameter types) as a clause or
/// function type.
fn body_form(head: &str, parts: usize) -> Option<usize> {
    match head {
        "=" | "|" | "@" | "." | ":" | "!" => Some(1),
        "->" => Some(if parts == 4 { 2 } else { 1 }),
        _ => None,
    }
}

struct Printer<'a> {
    nodes: &'a [(Node, Option<String>)],
    memo: HashMap<(usize, usize, usize), Rc<Layout>>,
}

impl Printer<'_> {
    /// Lay out node `id` starting at column `col`, followed on its last line
    /// by `suffix` columns of closing brackets or inline parts.
    fn layout(&mut self, id: usize, col: usize, suffix: usize) -> Rc<Layout> {
        if let Some(l) = self.memo.get(&(id, col, suffix)) {
            return l.clone();
        }
        let nodes = self.nodes;
        let (node, flat) = &nodes[id];
        let layout = match (node, flat) {
            (_, Some(flat)) if col + width(flat) + suffix <= WIDTH || matches!(node, Node::Atom(_)) => {
                measure(flat.clone(), col, suffix)
            }
            (Node::Prefix(parts, child), _) => {
                let mut text = String::new();
                let mut cur = col;
                for part in parts {
                    match part {
                        Part::Text(t) => {
                            text.push_str(t);
                            cur += t.len();
                        }
                        Part::Comment(c) => {
                            text.push(' ');
                            text.push_str(c);
                            newline(&mut text, col);
                            cur = col;
                        }
                    }
                }
                text.push_str(&self.layout(*child, cur, suffix).text);
                measure(text, col, suffix)
            }
            (Node::Seq { open, close, items }, _) => {
                let datums: Vec<usize> = items
                    .iter()
                    .filter_map(|i| match i {
                        Item::Datum(id) => Some(*id),
                        _ => None,
                    })
                    .collect();
                let inner = col + open.len();
                let head = match (*open, datums.first().map(|d| &nodes[*d].0)) {
                    ("(", Some(Node::Atom(head))) => Some(head),
                    _ => None,
                };
                let shapes = match head {
                    None => vec![Shape {
                        inline: 1,
                        indent: vec![inner],
                    }],
                    Some(head) => match body_form(head, datums.len()) {
                        Some(k) => {
                            let own_lines = Shape {
                                inline: 1,
                                indent: [vec![inner], vec![col + 4; k], vec![col + 2]].concat(),
                            };
                            // An indivisible distinguished part that does not
                            // fit on the head line goes on its own line.
                            let mut end = inner + width(head);
                            let fits = datums[1..datums.len().min(k + 1)].iter().all(|d| {
                                end += 1 + nodes[*d].1.as_ref().map_or(0, |s| width(s));
                                !matches!(nodes[*d].0, Node::Atom(_)) || end <= WIDTH
                            });
                            if fits {
                                vec![
                                    Shape {
                                        inline: 1 + k,
                                        indent: vec![inner, col + 2],
                                    },
                                    own_lines,
                                ]
                            } else {
                                vec![own_lines]
                            }
                        }
                        None => vec![
                            Shape {
                                inline: 2,
                                indent: vec![inner, inner + width(head) + 1],
                            },
                            Shape {
                                inline: 1,
                                indent: vec![inner],
                            },
                        ],
                    },
                };
                shapes
                    .iter()
                    .map(|shape| self.arrange(items, open, close, col, suffix, shape))
                    .min_by_key(|l| (l.overflow, l.lines))
                    .unwrap()
            }
            (Node::Atom(_), _) => unreachable!("an atom has a spelling"),
        };
        let layout = Rc::new(layout);
        self.memo.insert((id, col, suffix), layout.clone());
        layout
    }

    fn arrange(
        &mut self,
        items: &[Item],
        open: &str,
        close: &str,
        col: usize,
        suffix: usize,
        shape: &Shape,
    ) -> Layout {
        let indent = |i: usize| shape.indent[i.min(shape.indent.len() - 1)];
        let mut text = open.to_owned();
        let mut cur = col + open.len();
        // Still on the opening line with nothing after the bracket yet.
        let mut first = true;
        let mut open_line = true;
        let mut datum = 0;
        for (i, item) in items.iter().enumerate() {
            match item {
                Item::Datum(id) => {
                    let inline = open_line && datum < shape.inline;
                    if !inline {
                        open_line = false;
                        newline(&mut text, indent(datum));
                        cur = indent(datum);
                    } else if !first {
                        text.push(' ');
                        cur += 1;
                    }
                    let after = if i + 1 == items.len() {
                        suffix + close.len()
                    } else {
                        match &items[i + 1] {
                            Item::Datum(next) if inline && datum + 1 < shape.inline => {
                                1 + self.nodes[*next].1.as_ref().map_or(0, |s| width(s))
                            }
                            _ => 0,
                        }
                    };
                    let layout = self.layout(*id, cur, after);
                    text.push_str(&layout.text);
                    cur = layout.last_col;
                    datum += 1;
                }
                Item::Comment { text: comment, trailing } => {
                    if *trailing && (first || cur + 1 + width(comment) <= WIDTH) {
                        if !first {
                            text.push(' ');
                        }
                    } else {
                        newline(&mut text, indent(datum));
                    }
                    text.push_str(comment);
                    open_line = false;
                }
                Item::Blank => unreachable!("blank lines are kept only at the top level"),
            }
            first = false;
        }
        // A line comment cannot be followed by the closing bracket.
        if let Some(Item::Comment { .. }) = items.last() {
            newline(&mut text, indent(datum));
        }
        text.push_str(close);
        measure(text, col, suffix)
    }
}

fn newline(text: &mut String, indent: usize) {
    text.push('\n');
    text.extend(std::iter::repeat_n(' ', indent));
}

fn measure(text: String, col: usize, suffix: usize) -> Layout {
    let mut lines = 0;
    let mut overflow = 0;
    let mut last_col = 0;
    for (i, line) in text.split('\n').enumerate() {
        last_col = width(line) + if i == 0 { col } else { 0 };
        overflow += last_col.saturating_sub(WIDTH);
        lines += 1;
    }
    overflow += (last_col + suffix).saturating_sub(WIDTH.max(last_col));
    Layout {
        text,
        last_col,
        lines,
        overflow,
    }
}
