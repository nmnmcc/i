//! Report syntax spans for the test migration; edits remain source-preserving.
use ilang_syntax::{
    ast::{ExprKind, ItemKind, PatternKind},
    lexer::lex,
    token::TokenKind,
};
use serde_json::{Value, json};
use std::io::{self, BufRead};

fn inspect(source: &str) -> Value {
    let (module, diagnostics) = ilang_syntax::parse_module(source);
    if !diagnostics.is_empty() {
        return json!({"error": diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>()});
    }
    let tokens = lex(source).tokens;
    let mut tests = Vec::new();
    for item in &module.items {
        for attr in &item.attributes {
            if attr.module.is_some() || attr.name.name != "test" {
                continue;
            }
            let ItemKind::Binding(binding) = &item.kind else {
                return json!({"error":"@test is not a binding"});
            };
            let PatternKind::Var(name) = &binding.pattern.kind else {
                return json!({"error":"@test has a non-name pattern"});
            };
            if !matches!(binding.value.kind, ExprKind::Fn(_))
                || item.attributes.len() != 1
                || !attr.args.is_empty()
                || attr.call.is_some()
            {
                return json!({"error":format!("@test {name} has an unsupported declaration")});
            }
            let references = tokens
                .iter()
                .filter(|t| matches!(&t.kind, TokenKind::Name(n) if n == name))
                .count()
                > 1
                || module.exports.iter().any(|e| e.name.name == *name);
            tests.push(
                json!({"name":name, "attr_start":attr.span.start, "attr_end":attr.span.end,
                "binding_start":binding.span.start, "binding_end":binding.span.end,
                "value_start":binding.value.span.start, "value_end":binding.value.span.end,
                "referenced":references}),
            );
        }
    }
    let calls: Vec<_> = module
        .items
        .iter()
        .filter_map(|item| {
            let ItemKind::MacroItem {
                module: None,
                name,
                args,
            } = &item.kind
            else {
                return None;
            };
            if name.name != "test" || args.len() != 2 {
                return None;
            }
            let ExprKind::String(label) = &args[0].value.kind else {
                return None;
            };
            Some(
                json!({"name":label, "start":item.span.start, "end":item.span.end,
            "body_start":args[1].value.span.start, "body_end":args[1].value.span.end}),
            )
        })
        .collect();
    json!({"tests": tests, "calls": calls, "imports":module.imports_span.map(|s| json!([s.start,s.end])),
        "has_test_import":module.imports.iter().any(|i| i.path.iter().map(|p|p.name.as_str()).collect::<Vec<_>>() == ["std", "test"]),
        "test_name_taken":module.items.iter().any(|item| matches!(&item.kind, ItemKind::Binding(b) if matches!(&b.pattern.kind, PatternKind::Var(n) if n == "test")))})
}
fn main() {
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let source: String = serde_json::from_str(&line).unwrap();
        println!("{}", inspect(&source));
    }
}
