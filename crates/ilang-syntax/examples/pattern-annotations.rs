//! JSON-lines tree inventory and minimal annotation edits for the migration.
//! Build against the old parser before applying the grammar change; rebuild
//! against the new parser to verify saved trees (spans aside).
use ilang_syntax::syn3::{self, ast::*, walk};
use ilang_syntax::token::TokenKind as T;
use serde_json::json;
use std::io::{self, BufRead, Write};

struct Annotations<'a> {
    source: &'a str,
    edits: Vec<(usize, usize)>,
}

impl Annotations<'_> {
    fn annotation(&mut self, name_end: usize, end: usize) {
        let lex = ilang_syntax::lexer::lex3(&self.source[name_end..end]);
        if lex.tokens.len() < 3 || lex.tokens[0].kind != T::Colon {
            return;
        }
        let start = lex.tokens[1].span.start as usize + name_end;
        let mut i = 1;
        while i + 1 < lex.tokens.len() {
            if lex.tokens[i].kind == T::Pipe {
                // Group the whole annotation. This also preserves a union in
                // an unbracketed function result. The formatter can later
                // choose the more local result braces.
                self.edits.push((start, end));
                return;
            }
            i = lex.close[i].map_or(i + 1, |close| close as usize + 1);
        }
    }
}

impl walk::Visitor for Annotations<'_> {
    fn pattern(&mut self, p: &mut Pattern, enter: bool) {
        if enter && let PatternKind::Typed { binder, .. } = &p.kind {
            self.annotation(binder.span.0.end as usize, p.span.0.end as usize);
        }
    }
}

fn main() {
    let mut output = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let request: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let source = request["source"].as_str().unwrap();
        let lisp = request["lisp"].as_bool().unwrap_or(false);
        let mut annotations = Annotations {
            source,
            edits: Vec::new(),
        };
        let mode = request["mode"].as_str().unwrap_or("auto");
        let (mut module, mut diagnostics) = if lisp {
            syn3::lisp::from_lisp(source)
        } else {
            syn3::parse_module(source)
        };
        let mut tree = format!("{module:?}");
        let mut parsed_mode = "module";
        let mut formatted = None;
        if !lisp && mode != "module" && !diagnostics.is_empty() {
            if let Ok(mut expr) = syn3::parse_expr(source) {
                tree = format!("{expr:?}");
                formatted = Some(syn3::print_expr(&expr));
                diagnostics.clear();
                parsed_mode = "expr";
                walk::expr(&mut expr, &mut annotations);
            } else if let Ok(mut pattern) = syn3::parse_pattern(source) {
                tree = format!("{pattern:?}");
                formatted = Some(syn3::print_pattern(&pattern));
                diagnostics.clear();
                parsed_mode = "pattern";
                walk::pattern(&mut pattern, &mut annotations);
            }
        }
        if !lisp && parsed_mode == "module" {
            for item in &module.items {
                if let ItemKind::Decl { name, .. } = &item.kind {
                    annotations.annotation(name.span.0.end as usize, item.span.0.end as usize);
                }
            }
            walk::module(&mut module, &mut annotations);
        }
        let mut colons = Vec::new();
        if request["format"].as_bool().unwrap_or(false) {
            if parsed_mode == "module" && diagnostics.is_empty() {
                formatted = if lisp {
                    ilang_syntax::lisp::format(source).ok()
                } else {
                    syn3::format(source).ok()
                };
            }
            if !lisp {
                let lex = ilang_syntax::lexer::lex3(source);
                let mut groups: Vec<(usize, T)> = Vec::new();
                for (i, token) in lex.tokens.iter().enumerate() {
                    while groups.last().is_some_and(|(end, _)| *end <= i) {
                        groups.pop();
                    }
                    if token.kind == T::Colon
                        && !groups.last().is_some_and(|(_, kind)| *kind == T::LtLt)
                    {
                        colons.push(token.span.start);
                    }
                    if let Some(end) = lex.close[i] {
                        groups.push((end as usize, token.kind.clone()));
                    }
                }
            }
        } else {
            formatted = None;
        }
        annotations.edits.sort_unstable();
        annotations.edits.dedup();
        writeln!(output, "{}", json!({"tree": tree, "mode": parsed_mode, "edits": annotations.edits, "colons": colons, "formatted": formatted,
            "diagnostics": diagnostics.iter().map(|d| format!("{}: {}", d.code.as_str(), d.message)).collect::<Vec<_>>() })).unwrap();
    }
}
