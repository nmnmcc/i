//! The syntax of i: tokens, the line rule, the parser, the AST, and a printer.
//!
//! The only authority is `docs/spec.md`; section numbers in comments refer to it.

pub mod ast;
pub mod format;
pub mod lexer;
pub mod lisp;
#[doc(hidden)]
pub mod print;
pub mod span;
pub mod syn3;
pub mod deep;
pub mod facts;
pub mod group;
pub mod surface;
pub mod rows;
mod parse;
pub use print::string_literal;
pub mod token;
pub mod trace;

use span::Span;

pub use lexer::{Lexed, lex};

/// Parses a module of syntax three into the tree the compiler reads. The
/// module is complete as far as the source allows; the diagnostics say what
/// was wrong, sorted by position.
pub fn parse_module(source: &str) -> (ast::Module, Vec<Diagnostic>) {
    let (module, mut errors) = syn3::parse_module(source);
    errors.extend(facts::Transition::capture_module(&module, false).1);
    errors.sort_by_key(|d| (d.span.start, d.span.end));
    (module, errors)
}

/// Token marks used to explain errors in packages published before A3.
/// This never changes how a name is parsed or resolved.
pub fn old_syntax_marks(source: &str) -> bool {
    let lexed = lex(source);
    let mut depth = 0usize;
    for (i, token) in lexed.tokens.iter().enumerate() {
        use token::TokenKind as T;
        if let T::Name(name) = &token.kind {
            if depth == 0
                && (i == 0 || token.nl)
                && matches!(name.as_str(), "import" | "export")
                && lexed
                    .tokens
                    .get(i + 1)
                    .is_some_and(|t| t.kind == T::LParen && !t.nl)
            {
                return true;
            }
            if name == "case" {
                let mut j = i + 1;
                while let Some(t) = lexed.tokens.get(j) {
                    if t.nl || t.brk || t.kind == T::Eof {
                        break;
                    }
                    if t.kind == T::LBrace {
                        j += 1;
                        while let Some(t) = lexed.tokens.get(j) {
                            if t.kind == T::RBrace || t.brk {
                                break;
                            }
                            if t.kind == T::Arrow {
                                return true;
                            }
                            j = lexed.close[j].map_or(j + 1, |c| c as usize + 1);
                        }
                        break;
                    }
                    j = lexed.close[j].map_or(j + 1, |c| c as usize + 1);
                }
            }
        }
        match token.kind {
            T::LParen | T::LBracket | T::LBrace | T::LtLt => depth += 1,
            T::RParen | T::RBracket | T::RBrace | T::GtGt => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    false
}

/// A migration hint for an error path, never a grammar restriction.
pub fn old_syntax_hint(source: &str) -> Option<&'static str> {
    let lexed = lex(source);
    let has = |name: &str| {
        lexed
            .tokens
            .iter()
            .any(|t| matches!(&t.kind, token::TokenKind::Name(n) if n == name))
    };
    if old_syntax_marks(source) || has("import") || has("export") || has("case") {
        Some(
            "if this is old syntax: replace `import (…)` and `export (…)` with header groups `(…)`, and `case e { … }` with `e |> (pattern) { value } | (_) { other }`; write guards with `?` instead of `if`",
        )
    } else if has("if") {
        Some("if this is an old clause guard, replace `if condition` with `? condition`")
    } else {
        None
    }
}

/// The kind of a syntax error. Each kind has a stable code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Code {
    UnexpectedChar,
    UnterminatedString,
    BadEscape,
    BadUnicodeEscape,
    BadDigitGroup,
    BadNumber,
    FloatNotFinite,
    BadName,
    UnmatchedBracket,
    UnclosedBracket,
    Unexpected,
    MissingBody,
    ImportPlacement,
    ShortModulePath,
    BlockEnd,
    DiscardValue,
    BadSpread,
    BadPattern,
    ArrowTail,
    ArgOrder,
    PipeTarget,
    TypeGroupBreak,
    RecordReserved,
    HeaderGroup,
}

impl Code {
    pub fn as_str(self) -> &'static str {
        match self {
            Code::UnexpectedChar => "E0001",
            Code::UnterminatedString => "E0002",
            Code::BadEscape => "E0003",
            Code::BadUnicodeEscape => "E0004",
            Code::BadDigitGroup => "E0005",
            Code::BadNumber => "E0006",
            Code::FloatNotFinite => "E0007",
            Code::BadName => "E0008",
            Code::UnmatchedBracket => "E0009",
            Code::UnclosedBracket => "E0010",
            Code::Unexpected => "E0101",
            Code::MissingBody => "E0102",
            Code::ImportPlacement => "E0103",
            Code::ShortModulePath => "E0104",
            Code::BlockEnd => "E0105",
            Code::DiscardValue => "E0106",
            Code::BadSpread => "E0107",
            Code::BadPattern => "E0108",
            Code::ArrowTail => "E0109",
            Code::ArgOrder => "E0110",
            Code::PipeTarget => "E0111",
            Code::TypeGroupBreak => "E0112",
            Code::RecordReserved => "E0114",
            Code::HeaderGroup => "E0115",
        }
    }
}

/// A compile error found while reading the source.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{}: {message}", code.as_str())]
pub struct Diagnostic {
    pub code: Code,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn new(code: Code, span: Span, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            code,
            message: message.into(),
            span,
        }
    }
}

#[doc(hidden)]
pub mod walk;

#[cfg(test)]
#[allow(dead_code)]
mod legacy_shape {
//! Structural normalization for write validation and syntax-tree tests.
use migration_syntax::{
    ast::*,
    span::Span,
    walk::{Visitor, Walk},
};

struct Unspan;
impl Visitor for Unspan {
    fn span(&mut self, span: &mut Span) {
        *span = Span::default();
    }
}

/// The debug form of a module with its spans normalized structurally.
/// String literals, names and documentation are never interpreted as spans.
pub fn shape(module: &Module) -> String {
    let mut module = module.clone();
    module.walk(&mut Unspan);
    format!("{module:?}")
}

struct Canon;
impl Visitor for Canon {
    fn expr(&mut self, e: &mut Expr, enter: bool) {
        if !enter
            && let ExprKind::Block(b) = &mut e.kind
            && let [Stmt::Expr(inner)] = &mut b.stmts[..]
        {
            // Children are already normalized. Move the child out rather
            // than cloning it at every level of a nested group chain.
            let child = std::mem::replace(
                inner,
                Expr {
                    kind: ExprKind::Tuple(Vec::new()),
                    span: Span::default(),
                },
            );
            *e = child;
        }
    }
}

/// Remove transparent expression groups and normalize spans. All other
/// data, including every string payload, participates in AST equality.
pub(crate) fn canonical(module: &Module) -> Module {
    let mut module = module.clone();
    module.walk(&mut Canon);
    module.walk(&mut Unspan);
    module
}

struct Floats(Vec<String>);
impl Visitor for Floats {
    fn float(&mut self, value: &mut f64) {
        self.0.push(format!("{value:?}"));
        *value = 0.0;
    }
}

/// Compare syntax after removing transparent groups and source spans.
/// Unlike the debug form, this is safe for deeply nested generated syntax.
pub fn equivalent(a: &Module, b: &Module) -> bool {
    // `canon` historically compares Debug text, which distinguishes signed
    // zero and prints all NaNs alike. Normalize floating fields separately to
    // retain exactly that contract while using stack-safe structural equality.
    let mut a = canonical(a);
    let mut b = canonical(b);
    let mut af = Floats(Vec::new());
    let mut bf = Floats(Vec::new());
    a.walk(&mut af);
    b.walk(&mut bf);
    af.0 == bf.0 && a == b
}

/// Compare expression structure with the same normalization as `equivalent`.
pub fn equivalent_expression(a: &Expr, b: &Expr) -> bool {
    let mut a = a.clone();
    let mut b = b.clone();
    a.walk(&mut Canon);
    b.walk(&mut Canon);
    a.walk(&mut Unspan);
    b.walk(&mut Unspan);
    let mut af = Floats(Vec::new());
    let mut bf = Floats(Vec::new());
    a.walk(&mut af);
    b.walk(&mut bf);
    af.0 == bf.0 && a == b
}

/// The canonical debug form, for readable assertions in tests.
pub fn canon(module: &Module) -> String {
    format!("{:?}", canonical(module))
}

#[cfg(test)]
mod deep_tests {
    use super::*;
    #[test]
    fn equivalent_matches_debug_canonical_form_on_representative_values() {
        let samples = [
            "value = 1\n",
            "value = { 1 }\n",
            "value = 2\n",
            "value = (1, 2)\n",
            "value = (2, 1)\n",
            "value = (x: Int) { x + 1 }\n",
            "value = (x: String) { x + 1 }\n",
        ];
        let modules: Vec<_> = samples
            .iter()
            .map(|text| {
                let (module, errors) = migration_syntax::parse_module(text);
                assert!(errors.is_empty(), "{text}: {errors:?}");
                module
            })
            .collect();
        for a in &modules {
            for b in &modules {
                assert_eq!(equivalent(a, b), canon(a) == canon(b));
                let ItemKind::Binding(left) = &a.items[0].kind else {
                    unreachable!()
                };
                let ItemKind::Binding(right) = &b.items[0].kind else {
                    unreachable!()
                };
                assert_eq!(
                    equivalent_expression(&left.value, &right.value),
                    equivalent(a, b)
                );
            }
        }
        let mut plus = modules[0].clone();
        let mut minus = plus.clone();
        let set = |m: &mut Module, value| {
            let ItemKind::Binding(b) = &mut m.items[0].kind else {
                unreachable!()
            };
            b.value.kind = ExprKind::Float(value);
        };
        set(&mut plus, 0.0);
        set(&mut minus, -0.0);
        assert_ne!(canon(&plus), canon(&minus));
        assert!(!equivalent(&plus, &minus));
        let ItemKind::Binding(p) = &plus.items[0].kind else {
            unreachable!()
        };
        let ItemKind::Binding(m) = &minus.items[0].kind else {
            unreachable!()
        };
        assert!(!equivalent_expression(&p.value, &m.value));
        set(&mut plus, f64::NAN);
        set(&mut minus, f64::NAN);
        assert_eq!(canon(&plus), canon(&minus));
        assert!(equivalent(&plus, &minus));
        let ItemKind::Binding(p) = &plus.items[0].kind else {
            unreachable!()
        };
        let ItemKind::Binding(m) = &minus.items[0].kind else {
            unreachable!()
        };
        assert!(equivalent_expression(&p.value, &m.value));
    }
}

}
