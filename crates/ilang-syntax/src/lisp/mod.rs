//! The Lisp surface's position-independent datum layer (meta §2.2).
//!
//! A datum carries its source span even when nested. The following layer maps
//! the datum to the core AST in a specific syntactic position.

use crate::lexer::{DocComment, lex_lisp};
use crate::span::Span;
use crate::token::{Token, TokenKind as T};
use crate::{Code, Diagnostic};
use num_bigint::BigUint;

mod comments;
pub mod definition_schedule;
pub mod definition_table;
pub mod definitions;
mod encode;
pub mod expansion;
mod from_ast;
pub mod quote_lower;
mod render;
pub mod scoped;
pub mod source_syntax;
pub mod syntax;
mod to_ast;
mod to_source;
pub use encode::{expression_syntax, item_syntax, pattern_syntax, type_syntax};
pub use from_ast::{print_module, print_with_comments};
pub use to_ast::{
    parse_expression, parse_item, parse_items, parse_module, parse_pattern, parse_type,
};
pub use to_source::{normal_source, normal_with_comments};

#[derive(Clone, Debug, PartialEq)]
pub struct Datum {
    pub kind: DatumKind,
    pub span: Span,
    /// Documentation comments immediately preceding a top-level datum.
    pub docs: Vec<DocComment>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DatumKind {
    Name(String),
    Discard(String),
    UpName(String),
    Int(BigUint),
    Float(f64),
    String(String),
    Symbol(String),
    Round(Vec<Datum>),
    Square(Vec<Datum>),
    Curly(Vec<Datum>),
    Bits(Vec<Datum>),
    Quote(Box<Datum>),
    Unquote(Box<Datum>),
    Splice(Box<Datum>),
}

/// A file's datums and documentation after its final datum.
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub datums: Vec<Datum>,
    pub trailing_docs: Vec<DocComment>,
}

impl std::ops::Deref for Document {
    type Target = [Datum];
    fn deref(&self) -> &[Datum] {
        &self.datums
    }
}

impl IntoIterator for Document {
    type Item = Datum;
    type IntoIter = std::vec::IntoIter<Datum>;
    fn into_iter(self) -> Self::IntoIter {
        self.datums.into_iter()
    }
}

/// Read all datums, ignoring the normal surface's line break rule.
pub fn parse(source: &str) -> Result<Document, Vec<Diagnostic>> {
    let lexed = lex_lisp(source);
    if !lexed.diagnostics.is_empty() {
        return Err(lexed.diagnostics);
    }
    let mut parser = Parser {
        tokens: &lexed.tokens,
        at: 0,
    };
    let mut result = Vec::new();
    while parser.current().kind != T::Eof {
        result.push(parser.datum(false).map_err(|d| vec![d])?);
    }
    let mut docs = lexed.docs.into_iter().peekable();
    for datum in &mut result {
        while docs
            .peek()
            .is_some_and(|doc| doc.span.end <= datum.span.start)
        {
            datum.docs.push(docs.next().unwrap());
        }
    }
    Ok(Document {
        datums: result,
        trailing_docs: docs.collect(),
    })
}

/// Parse a Lisp-surface source file into the common core tree.
pub fn parse_source(source: &str) -> (crate::ast::Module, Vec<Diagnostic>) {
    match parse(source).and_then(|d| parse_module(&d).map_err(|e| vec![e])) {
        Ok(module) => (module, Vec::new()),
        Err(diags) => (crate::ast::Module::default(), diags),
    }
}

/// Format a Lisp module without discarding its ordinary comments.
pub fn format(source: &str) -> Result<String, Vec<Diagnostic>> {
    let (module, diagnostics) = parse_source(source);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    Ok(from_ast::format_with_comments(source, &module))
}

struct Parser<'a> {
    tokens: &'a [Token],
    at: usize,
}

impl Parser<'_> {
    fn current(&self) -> &Token {
        &self.tokens[self.at]
    }

    fn datum(&mut self, quoted: bool) -> Result<Datum, Diagnostic> {
        let token = self.current().clone();
        self.at += 1;
        let kind = match token.kind {
            T::Name(s) => DatumKind::Name(s),
            T::Discard(s) => DatumKind::Discard(s),
            T::UpName(s) => DatumKind::UpName(s),
            T::Int(n) => DatumKind::Int(n),
            T::Float(n) => DatumKind::Float(n),
            T::Str(s) => DatumKind::String(s),
            T::LParen | T::LBracket | T::LBrace | T::LtLt => {
                let end = match token.kind {
                    T::LParen => T::RParen,
                    T::LBracket => T::RBracket,
                    T::LBrace => T::RBrace,
                    T::LtLt => T::GtGt,
                    _ => unreachable!(),
                };
                let mut parts = Vec::new();
                while self.current().kind != end {
                    if self.current().kind == T::Eof {
                        return Err(Diagnostic::new(
                            Code::UnclosedBracket,
                            token.span,
                            "unclosed datum",
                        ));
                    }
                    parts.push(self.datum(quoted)?);
                }
                let close = self.current().span;
                self.at += 1;
                return Ok(Datum {
                    span: token.span.to(close),
                    docs: Vec::new(),
                    kind: match token.kind {
                        T::LParen => DatumKind::Round(parts),
                        T::LBracket => DatumKind::Square(parts),
                        T::LBrace => DatumKind::Curly(parts),
                        T::LtLt => DatumKind::Bits(parts),
                        _ => unreachable!(),
                    },
                });
            }
            T::Quote => {
                if quoted {
                    return Err(Diagnostic::new(
                        Code::Unexpected,
                        token.span,
                        "a quote inside a quote must occur in an unquote or splice",
                    ));
                }
                let child = self.datum(true)?;
                return Ok(Datum {
                    span: token.span.to(child.span),
                    docs: Vec::new(),
                    kind: DatumKind::Quote(Box::new(child)),
                });
            }
            T::Tilde => {
                if !quoted {
                    return Err(Diagnostic::new(
                        Code::Unexpected,
                        token.span,
                        "unquote is only allowed inside a quote",
                    ));
                }
                let splice = self.current().kind == T::DotDot;
                if splice {
                    self.at += 1;
                }
                let child = self.datum(false)?;
                return Ok(Datum {
                    span: token.span.to(child.span),
                    docs: Vec::new(),
                    kind: if splice {
                        DatumKind::Splice(Box::new(child))
                    } else {
                        DatumKind::Unquote(Box::new(child))
                    },
                });
            }
            T::Eof | T::RParen | T::RBracket | T::RBrace | T::GtGt => {
                return Err(Diagnostic::new(
                    Code::Unexpected,
                    token.span,
                    "expected a datum",
                ));
            }
            symbol => DatumKind::Symbol(symbol.text().into()),
        };
        Ok(Datum {
            kind,
            span: token.span,
            docs: Vec::new(),
        })
    }
}

/// Print a datum on one line. Whitespace separates tokens that might join.
pub fn show(datum: &Datum) -> String {
    use DatumKind::*;
    match &datum.kind {
        Name(s) | Discard(s) | UpName(s) | Symbol(s) => s.clone(),
        Int(n) => n.to_string(),
        Float(n) => float(*n),
        String(s) => crate::print::string_literal(s),
        Round(xs) => bracket("(", ")", xs),
        Square(xs) => bracket("[", "]", xs),
        Curly(xs) => bracket("{", "}", xs),
        Bits(xs) => {
            if xs.is_empty() {
                "<<>>".into()
            } else {
                bracket("<< ", " >>", xs)
            }
        }
        Quote(x) => format!("`{}", show(x)),
        Unquote(x) => format!("~{}", show(x)),
        Splice(x) => format!("~..{}", show(x)),
    }
}

fn float(n: f64) -> String {
    let s = format!("{n:?}");
    match s.split_once('e') {
        Some((mantissa, exponent)) if !mantissa.contains('.') => format!("{mantissa}.0e{exponent}"),
        _ => s,
    }
}

fn bracket(open: &str, close: &str, xs: &[Datum]) -> String {
    format!(
        "{open}{}{close}",
        xs.iter().map(show).collect::<Vec<_>>().join(" ")
    )
}
