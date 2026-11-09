//! The syntax of i: tokens, the line rule, the parser, the AST, and a printer.
//!
//! The only authority is `docs/spec.md`; section numbers in comments refer to it.

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod print;
pub mod span;
pub mod token;

use span::Span;

pub use lexer::{Lexed, lex};
pub use parser::parse_module;

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
