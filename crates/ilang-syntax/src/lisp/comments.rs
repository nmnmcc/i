//! Transfer comments attached to item or statement boundaries during conversion.
use crate::span::Span;

pub(super) fn transfer(
    source: &str,
    mut output: String,
    mut candidates: Vec<(Span, String)>,
    lisp: bool,
    keep_all: bool,
) -> (String, Vec<Span>) {
    let lexed = if lisp {
        crate::lexer::lex_lisp(source)
    } else {
        crate::lexer::lex(source)
    };
    candidates.sort_by_key(|(s, _)| (s.start, std::cmp::Reverse(s.end)));
    let mut positions = Vec::with_capacity(candidates.len());
    let mut cursor = 0;
    for (_, text) in &candidates {
        let position = output[cursor..].find(text).map(|offset| cursor + offset);
        if let Some(pos) = position {
            cursor = pos;
        }
        positions.push(position);
    }
    let mut insertions = Vec::new();
    let mut lost = Vec::new();
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
        let target = candidates.iter().enumerate().find(|(_, (span, _))| {
            span.start >= comment.span.end
                && !lexed.tokens.iter().any(|token| {
                    token.span.start >= comment.span.end
                        && token.span.end <= span.start
                        && token.span.start != token.span.end
                })
        });
        let exact = target
            .and_then(|(i, _)| positions[i])
            .filter(|_| standalone);
        let nearest = || {
            candidates
                .iter()
                .enumerate()
                .find_map(|(i, (s, _))| {
                    (s.start >= comment.span.end)
                        .then_some(positions[i])
                        .flatten()
                })
                .unwrap_or(output.len())
        };
        match exact.or_else(|| keep_all.then(nearest)) {
            Some(pos) => insertions.push((pos, comment.text.clone())),
            None => lost.push(comment.span),
        }
    }
    for (pos, comment) in insertions.into_iter().rev() {
        output.insert_str(pos, &format!("\n{comment}\n"));
    }
    (output, lost)
}
