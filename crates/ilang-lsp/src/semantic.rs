//! Token ranges retain expression highlighting inside interpolated strings.
use crate::position::LineIndex;
use ilang_syntax::{
    lexer,
    span::Span,
    token::{StringPart, Token, TokenKind as T},
};
use lsp_types::{SemanticToken, SemanticTokenType as S, SemanticTokens, SemanticTokensLegend};

pub(crate) fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            S::STRING,
            S::NUMBER,
            S::TYPE,
            S::VARIABLE,
            S::OPERATOR,
            S::COMMENT,
        ],
        token_modifiers: Vec::new(),
    }
}

fn ranges(tokens: &[Token], out: &mut Vec<(Span, u32)>) {
    for token in tokens {
        let ty = match &token.kind {
            T::Interpolated(parts) => {
                let mut at = token.span.start;
                for part in parts {
                    if let StringPart::Expr {
                        tokens,
                        span,
                        comments,
                    } = part
                    {
                        out.push((Span::new(at, span.start), 0));
                        ranges(tokens, out);
                        out.extend(comments.iter().map(|c| (c.span, 5)));
                        at = span.end;
                    }
                }
                out.push((Span::new(at, token.span.end), 0));
                continue;
            }
            T::Str(_) => 0,
            T::Int(_) | T::Float(_) => 1,
            T::UpName(_) => 2,
            T::Name(_) | T::Discard(_) => 3,
            T::Eof => continue,
            _ => 4,
        };
        out.push((token.span, ty));
    }
}

pub(crate) fn tokens(text: &str, lisp: bool) -> SemanticTokens {
    let lexed = if lisp {
        lexer::lex3_lisp(text)
    } else {
        lexer::lex3(text)
    };
    let mut spans = Vec::new();
    ranges(&lexed.tokens, &mut spans);
    spans.extend(lexed.comments.iter().map(|c| (c.span, 5)));
    spans.sort_by_key(|(s, _)| s.start);
    let ix = LineIndex::new(text);
    let mut data = Vec::new();
    let mut previous = (0, 0);
    for (span, token_type) in spans {
        let mut at = span.start as usize;
        let end = span.end as usize;
        while at < end {
            let stop = text[at..end].find(['\r', '\n']).map_or(end, |p| at + p);
            if stop > at {
                let p = ix.position(at as u32);
                let delta_line = p.line - previous.0;
                data.push(SemanticToken {
                    delta_line,
                    delta_start: if delta_line == 0 {
                        p.character - previous.1
                    } else {
                        p.character
                    },
                    length: text[at..stop].encode_utf16().count() as u32,
                    token_type,
                    token_modifiers_bitset: 0,
                });
                previous = (p.line, p.character);
            }
            at = stop;
            if text.as_bytes().get(at) == Some(&b'\r') {
                at += 1;
            }
            if text.as_bytes().get(at) == Some(&b'\n') {
                at += 1;
            }
        }
    }
    SemanticTokens {
        result_id: None,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interpolation_expressions_and_multiline_utf16_ranges() {
        let text = r####"value = #"""
  😀 \#(name <> "x")
  """#
"####;
        let result = tokens(text, false);
        let mut line = 0;
        let mut col = 0;
        let mut seen_name = false;
        for t in result.data {
            col = if t.delta_line == 0 {
                col + t.delta_start
            } else {
                t.delta_start
            };
            line += t.delta_line;
            if line == 1 && col == 8 && t.length == 4 {
                assert_eq!(t.token_type, 3);
                seen_name = true;
            }
        }
        assert!(seen_name);
    }
    #[test]
    fn comments_and_names_in_nested_interpolations_have_disjoint_ranges() {
        let text = r###"value = "a\( // leading
 #"inner \#(name)"#)b"
"###;
        let result = tokens(text, false);
        let ix = LineIndex::new(text);
        let (mut line, mut column) = (0, 0);
        let mut seen_comment = false;
        let mut seen_name = false;
        let mut end = 0;
        for token in result.data {
            column = if token.delta_line == 0 {
                column + token.delta_start
            } else {
                token.delta_start
            };
            line += token.delta_line;
            let a = ix.offset(lsp_types::Position::new(line, column));
            let z = ix.offset(lsp_types::Position::new(line, column + token.length));
            assert!(a >= end);
            let source = &text[a as usize..z as usize];
            if source == "// leading" {
                assert_eq!(token.token_type, 5);
                seen_comment = true;
            }
            if source == "name" {
                assert_eq!(token.token_type, 3);
                seen_name = true;
            }
            end = z;
        }
        assert!(seen_comment && seen_name);
    }
}
