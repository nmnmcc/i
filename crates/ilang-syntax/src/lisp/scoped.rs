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

/// A metered conversion distinguishes source syntax errors from compiler
/// resource exhaustion; the latter cannot be caught by guest code.
#[derive(Debug)]
pub enum MeteredError {
    Syntax(ScopedError),
    Resource(Box<dyn std::error::Error + Send + Sync>),
}

pub type ChargeResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

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
    /// Charge every occurrence before serializing or allocating its AST.
    /// The bound includes the virtual text, both origin maps, traversal stacks,
    /// Datum and AST nodes, and worst-case escaping of string data.
    fn preflight(
        &self,
        root: &Syntax,
        charge: &mut impl FnMut(u64, u64) -> ChargeResult,
    ) -> ChargeResult {
        charge(1, 1)?;
        let mut pending = vec![root];
        while let Some(syntax) = pending.pop() {
            charge(1, 64)?;
            match syntax.node() {
                Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                    let n = xs.len() as u64;
                    charge(n, n.saturating_mul(8))?;
                    pending.extend(xs.iter().rev());
                }
                Node::Quote(x) | Node::Unquote(x) | Node::Splice(x) => {
                    charge(1, 8)?;
                    pending.push(x);
                }
                Node::Name(text)
                | Node::Discard(text)
                | Node::Up(text)
                | Node::Symbol(text)
                | Node::Text(text) => {
                    let bytes = (text.len() as u64).saturating_mul(24).saturating_add(16);
                    charge(bytes, 4 + bytes.div_ceil(8))?;
                }
                Node::Int(n) => {
                    let limbs = n.bits().div_ceil(32);
                    let bytes = n.bits().saturating_add(1);
                    charge(
                        limbs.saturating_mul(limbs).saturating_add(bytes),
                        4 + bytes.div_ceil(8),
                    )?;
                }
                Node::Float(_) => charge(32, 8)?,
            }
        }
        Ok(())
    }

    fn append(&mut self, syntax: &Syntax, call_site: &Syntax) -> Datum {
        enum Task<'a> {
            Enter(&'a Syntax, Option<u64>),
            Separator,
            Finish {
                syntax: &'a Syntax,
                start: u32,
                owner: Option<u64>,
                count: usize,
                close: &'static str,
            },
        }
        self.text.push('\n');
        let mut tasks = vec![Task::Enter(syntax, None)];
        let mut built = Vec::<Datum>::new();
        while let Some(task) = tasks.pop() {
            match task {
                Task::Separator => self.text.push(' '),
                Task::Enter(current, owner) => {
                    let start = self.position();
                    let child_owner = current.definition_head().or(owner);
                    match current.node() {
                        Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                            let bits = matches!(current.node(), Node::Bits(_));
                            let (open, close) = match current.node() {
                                Node::Round(_) => ("(", ")"),
                                Node::Square(_) => ("[", "]"),
                                Node::Curly(_) => ("{", "}"),
                                Node::Bits(_) if xs.is_empty() => ("<<", ">>"),
                                Node::Bits(_) => ("<<", " >>"),
                                _ => unreachable!(),
                            };
                            self.text.push_str(open);
                            tasks.push(Task::Finish {
                                syntax: current,
                                start,
                                owner,
                                count: xs.len(),
                                close,
                            });
                            for (index, child) in xs.iter().enumerate().rev() {
                                tasks.push(Task::Enter(child, child_owner));
                                if index != 0 || bits {
                                    tasks.push(Task::Separator);
                                }
                            }
                        }
                        Node::Quote(child) | Node::Unquote(child) | Node::Splice(child) => {
                            self.text.push_str(match current.node() {
                                Node::Quote(_) => "`",
                                Node::Unquote(_) => "~",
                                Node::Splice(_) => "~..",
                                _ => unreachable!(),
                            });
                            tasks.push(Task::Finish {
                                syntax: current,
                                start,
                                owner,
                                count: 1,
                                close: "",
                            });
                            tasks.push(Task::Enter(child, child_owner));
                        }
                        _ => {
                            let datum = current.to_datum();
                            self.text.push_str(&super::show(&datum));
                            let span = Span::new(start, self.position());
                            self.record(span, current, call_site, owner);
                            built.push(Datum {
                                kind: datum.kind,
                                span,
                                docs: Vec::new(),
                            });
                        }
                    }
                }
                Task::Finish {
                    syntax: current,
                    start,
                    owner,
                    count,
                    close,
                } => {
                    self.text.push_str(close);
                    let children = built.split_off(built.len() - count);
                    let kind = match current.node() {
                        Node::Round(_) => DatumKind::Round(children),
                        Node::Square(_) => DatumKind::Square(children),
                        Node::Curly(_) => DatumKind::Curly(children),
                        Node::Bits(_) => DatumKind::Bits(children),
                        Node::Quote(_) | Node::Unquote(_) | Node::Splice(_) => {
                            let [child] =
                                <[Datum; 1]>::try_from(children).expect("quote has one child");
                            match current.node() {
                                Node::Quote(_) => DatumKind::Quote(Box::new(child)),
                                Node::Unquote(_) => DatumKind::Unquote(Box::new(child)),
                                Node::Splice(_) => DatumKind::Splice(Box::new(child)),
                                _ => unreachable!(),
                            }
                        }
                        _ => unreachable!("only compound syntax has a finish task"),
                    };
                    let span = Span::new(start, self.position());
                    self.record(span, current, call_site, owner);
                    built.push(Datum {
                        kind,
                        span,
                        docs: Vec::new(),
                    });
                }
            }
        }
        let [datum] = <[Datum; 1]>::try_from(built).expect("one expanded root was built");
        datum
    }

    fn record(
        &mut self,
        span: Span,
        syntax: &Syntax,
        call_site: &Syntax,
        definition_head: Option<u64>,
    ) {
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
            self.starts.insert(span.start, origin).is_none(),
            "generated syntax starts must be unique"
        );
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
    /// Convert one result while charging its construction to this macro run.
    /// Resource failure occurs before the source or origin table is modified.
    pub fn expression_metered(
        &mut self,
        syntax: &Syntax,
        call_site: &Syntax,
        mut charge: impl FnMut(u64, u64) -> ChargeResult,
    ) -> Result<Expr, MeteredError> {
        self.preflight(syntax, &mut charge)
            .map_err(MeteredError::Resource)?;
        self.expression(syntax, call_site)
            .map_err(MeteredError::Syntax)
    }
    pub fn pattern_metered(
        &mut self,
        syntax: &Syntax,
        call_site: &Syntax,
        mut charge: impl FnMut(u64, u64) -> ChargeResult,
    ) -> Result<Pattern, MeteredError> {
        self.preflight(syntax, &mut charge)
            .map_err(MeteredError::Resource)?;
        self.pattern(syntax, call_site)
            .map_err(MeteredError::Syntax)
    }
    pub fn ty_metered(
        &mut self,
        syntax: &Syntax,
        call_site: &Syntax,
        mut charge: impl FnMut(u64, u64) -> ChargeResult,
    ) -> Result<TypeExpr, MeteredError> {
        self.preflight(syntax, &mut charge)
            .map_err(MeteredError::Resource)?;
        self.ty(syntax, call_site).map_err(MeteredError::Syntax)
    }
    pub fn items_metered(
        &mut self,
        syntax: &Syntax,
        call_site: &Syntax,
        mut charge: impl FnMut(u64, u64) -> ChargeResult,
    ) -> Result<Vec<Item>, MeteredError> {
        self.preflight(syntax, &mut charge)
            .map_err(MeteredError::Resource)?;
        self.items(syntax, call_site).map_err(MeteredError::Syntax)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lisp::syntax::{Node, Scope};
    use std::collections::BTreeSet;

    #[test]
    fn deep_virtual_source_builds_and_releases_without_recursive_stack() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let scope = Scope {
                    module: 0,
                    marks: BTreeSet::new(),
                };
                let parsed = crate::lisp::parse("x").unwrap();
                let mut syntax = Syntax::from_datum(&parsed[0], &scope);
                for _ in 0..20_000 {
                    syntax = Syntax::make(Node::Round(vec![syntax.clone()]), &syntax).unwrap();
                }
                let mut source = ScopedSource::new("source".into());
                let datum = source.append(&syntax, &syntax);
                let mut pending = vec![datum];
                while let Some(datum) = pending.pop() {
                    match datum.kind {
                        DatumKind::Round(xs)
                        | DatumKind::Square(xs)
                        | DatumKind::Curly(xs)
                        | DatumKind::Bits(xs) => pending.extend(xs),
                        DatumKind::Quote(x) | DatumKind::Unquote(x) | DatumKind::Splice(x) => {
                            pending.push(*x)
                        }
                        _ => {}
                    }
                }
                drop(source);
                drop(syntax);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn metered_conversion_failure_does_not_modify_virtual_source() {
        let scope = Scope {
            module: 0,
            marks: BTreeSet::new(),
        };
        let parsed = crate::lisp::parse("(+ a b)").unwrap();
        let syntax = Syntax::from_datum(&parsed[0], &scope);
        let mut source = ScopedSource::new("original".into());
        let mut remaining = 2;
        let result = source.expression_metered(&syntax, &syntax, |_, _| {
            if remaining == 0 {
                return Err(Box::new(std::io::Error::other("resource limit")));
            }
            remaining -= 1;
            Ok(())
        });
        assert!(matches!(result, Err(MeteredError::Resource(_))));
        assert_eq!(source.text(), "original");
        assert!(source.spans.is_empty());
    }
}
