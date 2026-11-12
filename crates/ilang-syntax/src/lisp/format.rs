//! Source-preserving Lisp documents, laid out by the normal surface's engine.
//!
//! As in Racket's pp-one-up/pp-general and Clojure's logical blocks, a
//! form's distinguished prefix is grouped separately from its body. Comments
//! belong to gaps between source datums, not to regenerated AST nodes.
use super::{Datum, DatumKind as K};
use crate::Diagnostic;
use crate::format::{Doc, TAB_WIDTH, WIDTH, cat, group, nest, render_with_indent_limit, text};
use crate::lexer::{Comment, lex_lisp};
use crate::token::Token;

pub(super) fn format(source: &str) -> Result<String, Vec<Diagnostic>> {
    let document = super::parse(source)?;
    let lexed = lex_lisp(source);
    let comments = lexed.comments;
    let mut formatter = Formatter {
        source,
        comments: &comments,
        tokens: &lexed.tokens,
        next: 0,
    };
    let mut parts = Vec::new();
    let mut end = 0;
    for (i, datum) in document.iter().enumerate() {
        parts.push(formatter.gap(
            end,
            datum.span.start,
            if i == 0 { cat(vec![]) } else { Doc::Hard },
            true,
        ));
        parts.push(formatter.datum(datum));
        end = datum.span.end;
    }
    parts.push(formatter.gap(end, source.len() as u32, cat(vec![]), true));
    assert_eq!(
        formatter.next,
        comments.len(),
        "every Lisp comment is printed"
    );
    Ok(render_with_indent_limit(
        &cat(parts),
        WIDTH,
        WIDTH / TAB_WIDTH - 1,
    ))
}

struct Formatter<'a> {
    source: &'a str,
    comments: &'a [Comment],
    tokens: &'a [Token],
    next: usize,
}

impl Formatter<'_> {
    /// Whitespace and comments in exactly this source gap. A line comment
    /// always ends the output line, including before a closing delimiter.
    fn gap(&mut self, from: u32, to: u32, ordinary: Doc, top: bool) -> Doc {
        let mut parts = Vec::new();
        let mut end = from;
        while let Some(comment) = self.comments.get(self.next).filter(|c| c.span.start < to) {
            assert!(comment.span.start >= end, "comments follow datum order");
            let before = &self.source[end as usize..comment.span.start as usize];
            let mut comment_parts = Vec::new();
            if parts.is_empty() && from != 0 {
                comment_parts.push(if before.contains(['\n', '\r']) {
                    Doc::Hard
                } else {
                    Doc::Line
                });
            }
            if top && from != 0 && before.lines().filter(|l| l.trim().is_empty()).count() >= 2 {
                comment_parts.push(Doc::Hard);
            }
            comment_parts.push(text(if comment.text.starts_with("///") {
                comment.text.clone()
            } else {
                comment.text.trim_end().to_owned()
            }));
            parts.push(group(cat(comment_parts)));
            parts.push(Doc::Hard);
            end = comment.span.end;
            self.next += 1;
        }
        let whitespace = &self.source[end as usize..to as usize];
        if parts.is_empty() {
            parts.push(ordinary);
        }
        if top && from != 0 && whitespace.bytes().filter(|&b| b == b'\n').count() >= 2 {
            parts.push(Doc::Hard);
        }
        cat(parts)
    }

    fn datum(&mut self, datum: &Datum) -> Doc {
        match &datum.kind {
            K::Quote(d) | K::Unquote(d) | K::Splice(d) => {
                let mut parts = vec![text(if matches!(datum.kind, K::Quote(_)) {
                    "`"
                } else {
                    "~"
                })];
                let mut end = datum.span.start + 1;
                if matches!(datum.kind, K::Splice(_)) {
                    let dot = &self.tokens[self
                        .tokens
                        .partition_point(|t| t.span.start <= datum.span.start)];
                    parts.push(self.gap(end, dot.span.start, cat(vec![]), false));
                    parts.push(text(".."));
                    end = dot.span.end;
                }
                parts.push(self.gap(end, d.span.start, cat(vec![]), false));
                parts.push(self.datum(d));
                cat(parts)
            }
            K::Round(xs) => {
                let distinguished = match xs.first().map(|d| &d.kind) {
                    Some(K::Symbol(s)) if matches!(s.as_str(), "=" | "->" | "|" | "@") => 2,
                    _ => 1,
                };
                self.brackets(datum, xs, "(", ")", distinguished, false)
            }
            K::Square(xs) => self.brackets(datum, xs, "[", "]", 0, false),
            K::Curly(xs) => self.brackets(datum, xs, "{", "}", 0, xs.len() > 1),
            K::Bits(xs) => self.brackets(datum, xs, "<<", ">>", 0, false),
            _ => text(&self.source[datum.span.start as usize..datum.span.end as usize]),
        }
    }

    fn brackets(
        &mut self,
        datum: &Datum,
        xs: &[Datum],
        open: &str,
        close: &str,
        prefix: usize,
        vertical: bool,
    ) -> Doc {
        let mut head = Vec::new();
        let mut body = Vec::new();
        let mut end = datum.span.start + open.len() as u32;
        for (i, child) in xs.iter().enumerate() {
            let separator = if i == 0 && prefix > 0 {
                cat(vec![])
            } else if i == 0 {
                Doc::Soft
            } else if vertical {
                Doc::Hard
            } else {
                Doc::Line
            };
            let gap = self.gap(end, child.span.start, separator, false);
            let child_doc = self.datum(child);
            if i < prefix {
                // The opening bracket and the callee never separate. Other
                // prefix parts can break themselves when they are large.
                head.push(gap);
                head.push(child_doc);
            } else {
                body.push(gap);
                body.push(child_doc);
            }
            end = child.span.end;
        }
        let tail = self.gap(end, datum.span.end - close.len() as u32, cat(vec![]), false);
        let closing_comment = has_hard(std::slice::from_ref(&tail));
        body.push(tail);
        let forced = vertical || has_hard(&head) || has_hard(&body);
        let mut parts = vec![text(open), group(cat(head)), nest(cat(body))];
        if !xs.is_empty() && !closing_comment {
            parts.push(Doc::Soft);
        }
        parts.push(text(close));
        if forced {
            parts.push(Doc::Force);
        }
        group(cat(parts))
    }
}

fn has_hard(docs: &[Doc]) -> bool {
    docs.iter().any(|d| match d {
        Doc::Hard | Doc::Force => true,
        Doc::Cat(xs) => has_hard(xs),
        Doc::Group(d, ..) | Doc::Nest(d) | Doc::Full(d) => has_hard(std::slice::from_ref(d)),
        _ => false,
    })
}
