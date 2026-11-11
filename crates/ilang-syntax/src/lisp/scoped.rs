//! Lossless syntax-to-AST conversion for expanded code.
//!
//! Each occurrence gets a distinct, sliceable span in an in-memory suffix of
//! the caller's source file. The side table retains the real scoped syntax and
//! the call site, including when several occurrences share an original span.
use super::syntax::{Node, Syntax};
use super::{Datum, DatumKind};
use crate::ast::{Expr, Item, Pattern, TypeExpr};
use crate::span::Span;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Origin {
    pub syntax: Syntax,
    pub call_site: Syntax,
    /// Original definition head that contains this occurrence, if any.
    pub definition_head: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct ScopedError {
    pub code: crate::Code,
    pub message: String,
    pub origin: Origin,
}

/// Source text with a virtual, append-only suffix for expanded occurrences.
#[derive(Clone)]
pub struct ScopedSource {
    text: String,
    original_len: u32,
    spans: HashMap<Span, Origin>,
    starts: HashMap<u32, Origin>,
}

impl ScopedSource {
    pub fn new(source: String) -> Self {
        let original_len = u32::try_from(source.len()).expect("source is too large for spans");
        Self {
            text: source,
            original_len,
            spans: HashMap::new(),
            starts: HashMap::new(),
        }
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn original_len(&self) -> u32 {
        self.original_len
    }
    /// Locate a generated node, including a derived span that begins at one.
    pub fn origin(&self, span: Span) -> Option<&Origin> {
        self.spans
            .get(&span)
            .or_else(|| self.starts.get(&span.start))
    }
    pub fn is_generated(&self, span: Span) -> bool {
        span.start > self.original_len
    }

    fn position(&self) -> u32 {
        u32::try_from(self.text.len()).expect("expanded source exceeds span space")
    }
    fn datum(
        &mut self,
        syntax: &Syntax,
        call_site: &Syntax,
        definition_head: Option<u64>,
    ) -> Datum {
        let start = self.position();
        let child_head = syntax.definition_head().or(definition_head);
        let kind = match syntax.node() {
            Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                let (open, close) = match syntax.node() {
                    Node::Round(_) => ("(", ")"),
                    Node::Square(_) => ("[", "]"),
                    Node::Curly(_) => ("{", "}"),
                    Node::Bits(_) => ("<<", ">>"),
                    _ => unreachable!(),
                };
                self.text.push_str(open);
                let mut children = Vec::with_capacity(xs.len());
                for (i, child) in xs.iter().enumerate() {
                    if i != 0 || matches!(syntax.node(), Node::Bits(_)) {
                        self.text.push(' ');
                    }
                    children.push(self.datum(child, call_site, child_head));
                }
                if !xs.is_empty() && matches!(syntax.node(), Node::Bits(_)) {
                    self.text.push(' ');
                }
                self.text.push_str(close);
                match syntax.node() {
                    Node::Round(_) => DatumKind::Round(children),
                    Node::Square(_) => DatumKind::Square(children),
                    Node::Curly(_) => DatumKind::Curly(children),
                    Node::Bits(_) => DatumKind::Bits(children),
                    _ => unreachable!(),
                }
            }
            Node::Quote(child) | Node::Unquote(child) | Node::Splice(child) => {
                let prefix = match syntax.node() {
                    Node::Quote(_) => "`",
                    Node::Unquote(_) => "~",
                    Node::Splice(_) => "~..",
                    _ => unreachable!(),
                };
                self.text.push_str(prefix);
                let child = Box::new(self.datum(child, call_site, child_head));
                match syntax.node() {
                    Node::Quote(_) => DatumKind::Quote(child),
                    Node::Unquote(_) => DatumKind::Unquote(child),
                    Node::Splice(_) => DatumKind::Splice(child),
                    _ => unreachable!(),
                }
            }
            _ => {
                let mut datum = syntax.to_datum();
                self.text.push_str(&super::show(&datum));
                std::mem::replace(&mut datum.kind, DatumKind::Symbol(String::new()))
            }
        };
        let span = Span::new(start, self.position());
        let origin = Origin {
            syntax: syntax.clone(),
            call_site: call_site.clone(),
            definition_head,
        };
        assert!(
            self.spans.insert(span, origin.clone()).is_none(),
            "generated syntax spans must be unique"
        );
        assert!(
            self.starts.insert(start, origin).is_none(),
            "generated syntax starts must be unique"
        );
        Datum {
            kind,
            span,
            docs: Vec::new(),
        }
    }
    fn append(&mut self, syntax: &Syntax, call_site: &Syntax) -> Datum {
        self.text.push('\n');
        self.datum(syntax, call_site, None)
    }
    fn error(&self, error: crate::Diagnostic) -> ScopedError {
        let origin = self
            .origin(error.span)
            .expect("a syntax conversion error belongs to the expanded datum")
            .clone();
        ScopedError {
            code: error.code,
            message: error.message,
            origin,
        }
    }
    pub fn expression(&mut self, syntax: &Syntax, call_site: &Syntax) -> Result<Expr, ScopedError> {
        let datum = self.append(syntax, call_site);
        super::parse_expression(&datum).map_err(|error| self.error(error))
    }
    pub fn pattern(&mut self, syntax: &Syntax, call_site: &Syntax) -> Result<Pattern, ScopedError> {
        let datum = self.append(syntax, call_site);
        super::parse_pattern(&datum).map_err(|error| self.error(error))
    }
    pub fn ty(&mut self, syntax: &Syntax, call_site: &Syntax) -> Result<TypeExpr, ScopedError> {
        let datum = self.append(syntax, call_site);
        super::parse_type(&datum).map_err(|error| self.error(error))
    }
    pub fn items(&mut self, syntax: &Syntax, call_site: &Syntax) -> Result<Vec<Item>, ScopedError> {
        let datum = self.append(syntax, call_site);
        super::parse_items(&datum).map_err(|error| self.error(error))
    }
}
