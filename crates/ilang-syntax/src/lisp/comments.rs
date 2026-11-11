//! Transfer comments attached to item or statement boundaries during conversion.
use crate::span::Span;

pub(super) fn transfer(
    source: &str,
    mut output: String,
    mut candidates: Vec<(Span, String)>,
    lisp: bool,
    keep_all: bool,
    output_lisp: bool,
) -> (String, Vec<Span>) {
    let lexed = if lisp {
        crate::lexer::lex_lisp(source)
    } else {
        crate::lexer::lex(source)
    };
    candidates.sort_by_key(|(s, _)| (s.start, std::cmp::Reverse(s.end)));
    let output_lexed = if output_lisp {
        crate::lexer::lex_lisp(&output)
    } else {
        crate::lexer::lex(&output)
    };
    let starts: std::collections::HashSet<usize> = output_lexed
        .tokens
        .iter()
        .map(|token| token.span.start as usize)
        .collect();
    let mut positions = Vec::with_capacity(candidates.len());
    let mut cursor = 0;
    let mut previous: Option<(Span, &str)> = None;
    for (span, text) in &candidates {
        if previous.is_some_and(|(old, old_text)| {
            span.start > old.start && !(span.end <= old.end && old_text == text.as_str())
        }) && cursor < output.len()
        {
            cursor += output[cursor..]
                .chars()
                .next()
                .expect("cursor before end")
                .len_utf8();
        }
        let position = output[cursor..]
            .match_indices(text)
            .map(|(offset, _)| cursor + offset)
            .find(|pos| starts.contains(pos));
        if let Some(pos) = position {
            cursor = pos;
            previous = Some((*span, text));
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

/// Transfer Lisp comments using source and output offsets captured by the
/// renderer. Both streams follow source order, so no subtree rendering or
/// output searching is needed for a nested statement.
pub(super) fn transfer_anchored(
    source: &str,
    output: String,
    anchors: Vec<(Span, usize)>,
) -> (String, Vec<Span>) {
    let lexed = crate::lexer::lex_lisp(source);
    let mut candidate = 0;
    let mut token = 0;
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
        while candidate < anchors.len() && anchors[candidate].0.start < comment.span.end {
            candidate += 1;
        }
        while token < lexed.tokens.len()
            && (lexed.tokens[token].span.start < comment.span.end
                || lexed.tokens[token].span.start == lexed.tokens[token].span.end)
        {
            token += 1;
        }
        let exact = anchors.get(candidate).filter(|(span, _)| {
            token == lexed.tokens.len() || lexed.tokens[token].span.end > span.start
        });
        if let Some((_, at)) = exact.filter(|_| standalone) {
            insertions.push((*at, comment.text.as_str()));
        } else {
            lost.push(comment.span);
        }
    }
    let extra = insertions
        .iter()
        .map(|(_, text)| text.len() + 2)
        .sum::<usize>();
    let mut result = String::with_capacity(output.len() + extra);
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
