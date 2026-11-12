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
mod encode_charge;
pub mod expansion;
mod format;
mod from_ast;
pub mod quote_lower;
mod render;
pub mod scoped;
pub mod source_syntax;
pub mod syntax;
mod to_ast;
mod to_source;
pub(crate) mod trampoline;
pub use encode::{SourceEncoder, expression_syntax, item_syntax, pattern_syntax, type_syntax};
pub use encode_charge::SyntaxCharge;
pub use from_ast::{print_module, print_with_comments};
pub use to_ast::{
    parse_expression, parse_item, parse_items, parse_module, parse_pattern, parse_type,
};
pub use to_source::{normal_source, normal_with_comments};

#[derive(Debug)]
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

// Compare arbitrary-depth syntax without recursing through Rust stack frames.
impl PartialEq for Datum {
    fn eq(&self, other: &Self) -> bool {
        let mut work = vec![(self, other)];
        while let Some((a, b)) = work.pop() {
            if a.span != b.span || a.docs != b.docs {
                return false;
            }
            use DatumKind as K;
            match (&a.kind, &b.kind) {
                (K::Name(x), K::Name(y))
                | (K::Discard(x), K::Discard(y))
                | (K::UpName(x), K::UpName(y))
                | (K::String(x), K::String(y))
                | (K::Symbol(x), K::Symbol(y))
                    if x == y => {}
                (K::Int(x), K::Int(y)) if x == y => {}
                (K::Float(x), K::Float(y)) if x == y => {}
                (K::Round(x), K::Round(y))
                | (K::Square(x), K::Square(y))
                | (K::Curly(x), K::Curly(y))
                | (K::Bits(x), K::Bits(y)) => {
                    if x.len() != y.len() {
                        return false;
                    }
                    work.extend(x.iter().zip(y));
                }
                (K::Quote(x), K::Quote(y))
                | (K::Unquote(x), K::Unquote(y))
                | (K::Splice(x), K::Splice(y)) => work.push((x, y)),
                _ => return false,
            }
        }
        true
    }
}

// Datum trees may be supplied by macros and can be much deeper than the host
// call stack. Both copying quotes and disposing failed conversions must walk
// the tree explicitly; a depth limit here would change the language contract.
impl Clone for Datum {
    fn clone(&self) -> Self {
        enum Work<'a> {
            Visit(&'a Datum),
            Build(&'a Datum, usize),
        }
        let mut work = vec![Work::Visit(self)];
        let mut values: Vec<Datum> = Vec::new();
        while let Some(task) = work.pop() {
            match task {
                Work::Visit(d) => {
                    let children: &[Datum] = match &d.kind {
                        DatumKind::Round(xs)
                        | DatumKind::Square(xs)
                        | DatumKind::Curly(xs)
                        | DatumKind::Bits(xs) => xs,
                        DatumKind::Quote(x) | DatumKind::Unquote(x) | DatumKind::Splice(x) => {
                            std::slice::from_ref(x)
                        }
                        _ => &[],
                    };
                    work.push(Work::Build(d, children.len()));
                    work.extend(children.iter().rev().map(Work::Visit));
                }
                Work::Build(d, n) => {
                    let children = values.split_off(values.len() - n);
                    let kind = match &d.kind {
                        DatumKind::Name(x) => DatumKind::Name(x.clone()),
                        DatumKind::Discard(x) => DatumKind::Discard(x.clone()),
                        DatumKind::UpName(x) => DatumKind::UpName(x.clone()),
                        DatumKind::Int(x) => DatumKind::Int(x.clone()),
                        DatumKind::Float(x) => DatumKind::Float(*x),
                        DatumKind::String(x) => DatumKind::String(x.clone()),
                        DatumKind::Symbol(x) => DatumKind::Symbol(x.clone()),
                        DatumKind::Round(_) => DatumKind::Round(children),
                        DatumKind::Square(_) => DatumKind::Square(children),
                        DatumKind::Curly(_) => DatumKind::Curly(children),
                        DatumKind::Bits(_) => DatumKind::Bits(children),
                        DatumKind::Quote(_) => {
                            DatumKind::Quote(Box::new(children.into_iter().next().unwrap()))
                        }
                        DatumKind::Unquote(_) => {
                            DatumKind::Unquote(Box::new(children.into_iter().next().unwrap()))
                        }
                        DatumKind::Splice(_) => {
                            DatumKind::Splice(Box::new(children.into_iter().next().unwrap()))
                        }
                    };
                    values.push(Datum {
                        kind,
                        span: d.span,
                        docs: d.docs.clone(),
                    });
                }
            }
        }
        values.pop().expect("datum clone has a root")
    }
}
impl Drop for Datum {
    fn drop(&mut self) {
        if matches!(
            &self.kind,
            DatumKind::Name(_)
                | DatumKind::Discard(_)
                | DatumKind::UpName(_)
                | DatumKind::Int(_)
                | DatumKind::Float(_)
                | DatumKind::String(_)
                | DatumKind::Symbol(_)
        ) {
            return;
        }
        let mut work = vec![std::mem::replace(
            &mut self.kind,
            DatumKind::Symbol(String::new()),
        )];
        while let Some(kind) = work.pop() {
            match kind {
                DatumKind::Round(mut xs)
                | DatumKind::Square(mut xs)
                | DatumKind::Curly(mut xs)
                | DatumKind::Bits(mut xs) => {
                    for child in &mut xs {
                        work.push(std::mem::replace(
                            &mut child.kind,
                            DatumKind::Symbol(String::new()),
                        ));
                    }
                }
                DatumKind::Quote(mut x) | DatumKind::Unquote(mut x) | DatumKind::Splice(mut x) => {
                    work.push(std::mem::replace(
                        &mut x.kind,
                        DatumKind::Symbol(String::new()),
                    ));
                }
                _ => {}
            }
        }
    }
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
    let (_, diagnostics) = parse_source(source);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    format::format(source)
}

struct Parser<'a> {
    tokens: &'a [Token],
    at: usize,
}

impl Parser<'_> {
    fn current(&self) -> &Token {
        &self.tokens[self.at]
    }

    fn datum(&mut self, mut quoted: bool) -> Result<Datum, Diagnostic> {
        enum Frame {
            Group {
                token: Token,
                end: T,
                parts: Vec<Datum>,
                quoted: bool,
            },
            Prefix {
                token: Token,
                splice: Option<bool>,
            },
        }
        let mut frames = Vec::new();
        loop {
            if let Some(Frame::Group {
                token, end, parts, ..
            }) = frames.last_mut()
            {
                if self.current().kind == *end {
                    let close = self.current().span;
                    self.at += 1;
                    let token = token.clone();
                    let parts = std::mem::take(parts);
                    frames.pop();
                    let kind = match token.kind {
                        T::LParen => DatumKind::Round(parts),
                        T::LBracket => DatumKind::Square(parts),
                        T::LBrace => DatumKind::Curly(parts),
                        T::LtLt => DatumKind::Bits(parts),
                        _ => unreachable!(),
                    };
                    let completed = Datum {
                        span: token.span.to(close),
                        docs: Vec::new(),
                        kind,
                    };
                    if let Some(result) = complete(&mut frames, completed, &mut quoted) {
                        return Ok(result);
                    }
                    continue;
                }
                if self.current().kind == T::Eof {
                    return Err(Diagnostic::new(
                        Code::UnclosedBracket,
                        token.span,
                        "unclosed datum",
                    ));
                }
            }
            let token = self.current().clone();
            self.at += 1;
            let kind = match token.kind.clone() {
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
                    frames.push(Frame::Group {
                        token,
                        end,
                        parts: Vec::new(),
                        quoted,
                    });
                    continue;
                }
                T::Quote => {
                    if quoted {
                        return Err(Diagnostic::new(
                            Code::Unexpected,
                            token.span,
                            "a quote inside a quote must occur in an unquote or splice",
                        ));
                    }
                    frames.push(Frame::Prefix {
                        token,
                        splice: None,
                    });
                    quoted = true;
                    continue;
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
                    frames.push(Frame::Prefix {
                        token,
                        splice: Some(splice),
                    });
                    quoted = false;
                    continue;
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
            let completed = Datum {
                kind,
                span: token.span,
                docs: Vec::new(),
            };
            if let Some(result) = complete(&mut frames, completed, &mut quoted) {
                return Ok(result);
            }
        }

        fn complete(frames: &mut Vec<Frame>, mut value: Datum, quoted: &mut bool) -> Option<Datum> {
            loop {
                match frames.last_mut() {
                    Some(Frame::Group {
                        parts,
                        quoted: group_quoted,
                        ..
                    }) => {
                        parts.push(value);
                        *quoted = *group_quoted;
                        return None;
                    }
                    Some(Frame::Prefix { .. }) => {
                        let Frame::Prefix { token, splice } = frames.pop().unwrap() else {
                            unreachable!()
                        };
                        let span = token.span.to(value.span);
                        let kind = match splice {
                            None => DatumKind::Quote(Box::new(value)),
                            Some(false) => DatumKind::Unquote(Box::new(value)),
                            Some(true) => DatumKind::Splice(Box::new(value)),
                        };
                        value = Datum {
                            kind,
                            span,
                            docs: Vec::new(),
                        };
                    }
                    None => return Some(value),
                }
            }
        }
    }
}

/// Print a datum on one line. Whitespace separates tokens that might join.
pub fn show(datum: &Datum) -> String {
    show_tree(datum)
}

enum View<'a, T> {
    Atom(&'a str),
    Int(&'a BigUint),
    Float(f64),
    String(&'a str),
    Bracket(&'static str, &'static str, &'a [T]),
    Prefix(&'static str, &'a T),
}

trait Printable: Sized {
    fn view(&self) -> View<'_, Self>;
}

impl Printable for Datum {
    fn view(&self) -> View<'_, Self> {
        use DatumKind::*;
        match &self.kind {
            Name(s) | Discard(s) | UpName(s) | Symbol(s) => View::Atom(s),
            Int(n) => View::Int(n),
            Float(n) => View::Float(*n),
            String(s) => View::String(s),
            Round(xs) => View::Bracket("(", ")", xs),
            Square(xs) => View::Bracket("[", "]", xs),
            Curly(xs) => View::Bracket("{", "}", xs),
            Bits(xs) if xs.is_empty() => View::Bracket("<<", ">>", xs),
            Bits(xs) => View::Bracket("<< ", " >>", xs),
            Quote(x) => View::Prefix("`", x),
            Unquote(x) => View::Prefix("~", x),
            Splice(x) => View::Prefix("~..", x),
        }
    }
}

impl Printable for syntax::Syntax {
    fn view(&self) -> View<'_, Self> {
        use syntax::Node::*;
        match self.node() {
            Name(s) | Discard(s) | Up(s) | Symbol(s) => View::Atom(s),
            Int(n) => View::Int(n),
            Float(n) => View::Float(*n),
            Text(s) => View::String(s),
            Round(xs) => View::Bracket("(", ")", xs),
            Square(xs) => View::Bracket("[", "]", xs),
            Curly(xs) => View::Bracket("{", "}", xs),
            Bits(xs) if xs.is_empty() => View::Bracket("<<", ">>", xs),
            Bits(xs) => View::Bracket("<< ", " >>", xs),
            Quote(x) => View::Prefix("`", x),
            Unquote(x) => View::Prefix("~", x),
            Splice(x) => View::Prefix("~..", x),
        }
    }
}

fn show_tree<T: Printable>(tree: &T) -> String {
    use std::fmt::Write;
    enum Task<'a, T> {
        Node(&'a T),
        Text(&'static str),
    }
    let mut pending = vec![Task::Node(tree)];
    let mut out = String::new();
    while let Some(task) = pending.pop() {
        let node = match task {
            Task::Text(s) => {
                out.push_str(s);
                continue;
            }
            Task::Node(n) => n,
        };
        match node.view() {
            View::Atom(s) => out.push_str(s),
            View::Int(n) => write!(&mut out, "{n}").expect("writing to a String"),
            View::Float(n) => out.push_str(&float(n)),
            View::String(s) => out.push_str(&crate::print::string_literal(s)),
            View::Prefix(prefix, child) => {
                out.push_str(prefix);
                pending.push(Task::Node(child));
            }
            View::Bracket(open, close, children) => {
                out.push_str(open);
                pending.push(Task::Text(close));
                for i in (0..children.len()).rev() {
                    pending.push(Task::Node(&children[i]));
                    if i != 0 {
                        pending.push(Task::Text(" "));
                    }
                }
            }
        }
    }
    out
}

fn float(n: f64) -> String {
    let s = format!("{n:?}");
    match s.split_once('e') {
        Some((mantissa, exponent)) if !mantissa.contains('.') => format!("{mantissa}.0e{exponent}"),
        _ => s,
    }
}
