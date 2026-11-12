//! The syntax of i: tokens, the line rule, the parser, the AST, and a printer.
//!
//! The only authority is `docs/spec.md`; section numbers in comments refer to it.

pub mod ast;
mod ast_clone;
mod ast_eq;
pub mod format;
#[cfg(feature = "legacy")]
pub mod legacy;
pub mod lexer;
pub mod lisp;
#[doc(hidden)]
pub mod print;
#[doc(hidden)]
pub mod shape;
pub mod span;
pub mod syn3;
pub mod token;

use span::Span;

pub use lexer::{Lexed, lex};

/// Parses a module of syntax three into the tree the compiler reads. The
/// module is complete as far as the source allows; the diagnostics say what
/// was wrong, sorted by position.
pub fn parse_module(source: &str) -> (ast::Module, Vec<Diagnostic>) {
    syn3::parse_lowered(source)
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
            "if this is old syntax: replace `import (…)` and `export (…)` with header groups `(…)`, and `case e { … }` with `e |> { … }`; write guards with `?` instead of `if`",
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
