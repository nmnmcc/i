//! Conversion of a file between the two surfaces (`i fmt --to`), on the
//! tree of syntax three: the file is read, written in the other surface,
//! read back and compared, and the comments that stand alone before an item
//! or a statement are moved to the same item or statement of the output.

use super::ast::*;
use super::walk::{self, Visitor};
use crate::lexer::{Lexed, lex3, lex3_lisp};
use crate::span::Span;
use crate::{Code, Diagnostic};

/// A converted file, and the spans of the comments that have no place in it.
pub type Converted = Result<(String, Vec<Span>), Vec<Diagnostic>>;

/// The tree without groups: a block of one expression statement, in the
/// place of an expression, is that expression. The two surfaces differ only
/// in where they need groups.
pub fn ungrouped(module: &Module) -> Module {
    struct Ungroup;
    impl Visitor for Ungroup {
        fn expr(&mut self, e: &mut Expr, enter: bool) {
            if enter {
                return;
            }
            if let ExprKind::Block(b) = &mut e.kind
                && let [Stmt::Expr(_)] = b.stmts.as_slice()
                && let Some(Stmt::Expr(inner)) = b.stmts.pop()
            {
                *e = inner;
            }
        }
    }
    let mut module = module.clone();
    walk::module(&mut module, &mut Ungroup);
    module
}

fn unrepresentable() -> Vec<Diagnostic> {
    vec![Diagnostic::new(
        Code::Unexpected,
        Span::default(),
        "the module cannot be represented in the other surface",
    )]
}

/// The places a comment can be moved to, in source order: the header
/// groups, the items, and the statements of every block.
fn anchors(module: &Module, lexed: &Lexed) -> Vec<Span> {
    struct Statements(Vec<Span>);
    impl Visitor for Statements {
        fn block(&mut self, b: &mut Block, enter: bool) {
            if enter {
                self.0.extend(b.stmts.iter().map(|s| match s {
                    Stmt::Bind(b) => b.span.0,
                    Stmt::Expr(e) => e.span.0,
                }));
            }
        }
    }
    // A header group begins at the bracket before its first part: `(` in
    // the normal surface, `(,` in the Lisp surface.
    let group = |spans: Vec<Span>| {
        let cover = spans.into_iter().reduce(Span::to)?;
        let first = lexed.tokens.partition_point(|t| t.span.end <= cover.start);
        let open = lexed.tokens[first.saturating_sub(2)..first]
            .iter()
            .rev()
            .find(|t| t.kind == crate::token::TokenKind::LParen)
            .map(|t| t.span);
        Some(open.map_or(cover, |o| o.to(cover)))
    };
    let mut out = Statements(Vec::new());
    out.0
        .extend(group(module.imports.iter().map(|i| i.span.0).collect()));
    out.0
        .extend(group(module.exports.iter().map(|e| e.span.0).collect()));
    let mut module = module.clone();
    for item in &mut module.items {
        // An item with attributes begins at its first attribute.
        out.0.push(item.span.0);
        walk::item(item, &mut out);
    }
    out.0.sort_by_key(|s| (s.start, std::cmp::Reverse(s.end)));
    out.0
}

/// Moves the comments of `source` to `output`. `from` and `to` are the same
/// places of the two texts, in the same order. A comment is moved when it
/// stands alone on its line and no token is between it and a place.
fn transfer(
    source: &str,
    lexed: &Lexed,
    output: String,
    from: &[Span],
    to: &[Span],
) -> (String, Vec<Span>) {
    let mut insertions: Vec<(usize, &str)> = Vec::new();
    let mut lost = Vec::new();
    let paired = from.len() == to.len();
    // Comments, tokens and places are in source order: one pass.
    let (mut token, mut place) = (0, 0);
    for comment in &lexed.comments {
        if comment.text.starts_with("///") {
            continue;
        }
        let line_start = source[..comment.span.start as usize]
            .rfind(['\r', '\n'])
            .map_or(0, |x| x + 1);
        let standalone = source[line_start..comment.span.start as usize]
            .trim()
            .is_empty();
        while lexed
            .tokens
            .get(token)
            .is_some_and(|t| t.span.start < comment.span.end || t.span.start == t.span.end)
        {
            token += 1;
        }
        while from.get(place).is_some_and(|s| s.start < comment.span.end) {
            place += 1;
        }
        let next = lexed.tokens.get(token).map(|t| t.span.start);
        match from.get(place) {
            Some(s) if paired && standalone && next == Some(s.start) => {
                insertions.push((to[place].start as usize, comment.text.as_str()))
            }
            _ => lost.push(comment.span),
        }
    }
    insertions.sort_by_key(|(at, _)| *at);
    let mut result = String::with_capacity(output.len());
    let mut cursor = 0;
    for (at, comment) in insertions {
        result.push_str(&output[cursor..at]);
        result.push('\n');
        result.push_str(comment);
        result.push('\n');
        cursor = at;
    }
    result.push_str(&output[cursor..]);
    (result, lost)
}

/// A normal-surface file as a Lisp-surface file, one datum per line.
pub fn normal_to_lisp(source: &str) -> Converted {
    let (module, errors) = super::parse_module(source);
    if !errors.is_empty() {
        return Err(errors);
    }
    let output = super::lisp::to_lisp(&module);
    let (back, errors) = super::lisp::from_lisp(&output);
    if !errors.is_empty() {
        return Err(errors);
    }
    // A group that the other surface does not write is not a place.
    let (module, back) = (ungrouped(&module), ungrouped(&back));
    if module != back {
        return Err(unrepresentable());
    }
    let lexed = lex3(source);
    let from = anchors(&module, &lexed);
    let to = anchors(&back, &lex3_lisp(&output));
    Ok(transfer(source, &lexed, output, &from, &to))
}

/// A Lisp-surface file as a normal-surface file.
pub fn lisp_to_normal(source: &str) -> Converted {
    let (module, errors) = super::lisp::from_lisp(source);
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut grouped = module.clone();
    super::group::regroup(&mut grouped);
    let output = super::print_module(&grouped);
    let (back, errors) = super::parse_module(&output);
    let (module, back) = (ungrouped(&module), ungrouped(&back));
    if !errors.is_empty() || module != back {
        // The text of a quote is Lisp; the normal surface has no place for it.
        return Err(unrepresentable());
    }
    let lexed = lex3_lisp(source);
    let from = anchors(&module, &lexed);
    let to = anchors(&back, &lex3(&output));
    Ok(transfer(source, &lexed, output, &from, &to))
}
