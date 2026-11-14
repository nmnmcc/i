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
    /// Outer-to-inner calls that produced this occurrence.
    pub call_chain: Vec<Invocation>,
}

#[derive(Clone, Debug)]
pub struct Invocation {
    pub syntax: Syntax,
    pub macro_name: String,
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
#[derive(Clone, Debug)]
pub struct ScopedSource {
    text: String,
    original_len: u32,
    transition: crate::syn3::lower::Transition,
    spans: HashMap<Span, Origin>,
    starts: HashMap<u32, Origin>,
    children: HashMap<Span, Vec<Span>>,
    item_overrides: HashMap<Span, (Syntax, Span)>,
}

impl ScopedSource {
    pub fn new(source: String) -> Self {
        let original_len = u32::try_from(source.len()).expect("source is too large for spans");
        Self {
            text: source,
            original_len,
            transition: Default::default(),
            spans: HashMap::new(),
            starts: HashMap::new(),
            children: HashMap::new(),
            item_overrides: HashMap::new(),
        }
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn transition(&self) -> &crate::syn3::lower::Transition {
        &self.transition
    }
    pub fn original_len(&self) -> u32 {
        self.original_len
    }
    /// Locate a generated node, including a derived span that begins at one.
    pub fn origin(&self, span: Span) -> Option<&Origin> {
        self.spans
            .get(&span)
            .or_else(|| self.starts.get(&span.start))
            .or_else(|| {
                self.spans
                    .iter()
                    .filter(|(s, _)| s.start <= span.start && span.end <= s.end)
                    .min_by_key(|(s, _)| s.end - s.start)
                    .map(|(_, origin)| origin)
            })
    }
    pub fn set_item_override(&mut self, item: Span, syntax: Syntax, root: Span) {
        self.item_overrides.insert(item, (syntax, root));
    }
    pub fn item_override(&self, item: Span) -> Option<&(Syntax, Span)> {
        self.item_overrides.get(&item)
    }

    /// The complete generated definition that owns a constructor name.
    /// The old tree may have absorbed this item into a sum, but every
    /// virtual occurrence still has its original scoped datum.
    pub fn definition_origin(&self, at: Span, name: &str) -> Option<(Syntax, Span)> {
        fn head_name(syntax: &Syntax) -> Option<&str> {
            let mut syntax = syntax;
            loop {
                let Node::Round(parts) = syntax.node() else {
                    return None;
                };
                match parts.as_slice() {
                    [at, _, inner] if matches!(at.node(), Node::Symbol(s) if s == "@") => {
                        syntax = inner
                    }
                    [eq, head, _] if matches!(eq.node(), Node::Symbol(s) if s == "=") => {
                        return match head.node() {
                            Node::Up(name) => Some(name),
                            Node::Round(parts) => match parts.first()?.node() {
                                Node::Up(name) => Some(name),
                                _ => None,
                            },
                            _ => None,
                        };
                    }
                    _ => return None,
                }
            }
        }
        self.spans
            .iter()
            .filter(|(span, origin)| {
                span.start <= at.start
                    && at.end <= span.end
                    && head_name(&origin.syntax) == Some(name)
            })
            .max_by_key(|(span, _)| span.end - span.start)
            .map(|(span, origin)| {
                self.item_overrides
                    .get(span)
                    .cloned()
                    .unwrap_or_else(|| (origin.syntax.clone(), *span))
            })
    }
    pub fn child_span(&self, parent: Span, index: usize) -> Option<Span> {
        self.children
            .get(&parent)
            .and_then(|children| children.get(index))
            .copied()
    }
    pub fn is_generated(&self, span: Span) -> bool {
        span.start > self.original_len
    }

    /// Rebuild a previously encoded item after type expansions. Replacement
    /// spans are occurrence spans, not the original nodes' source spans: two
    /// splices of one Syntax may expand independently. Each replacement target
    /// names the virtual root produced by the corresponding expansion.
    pub fn rewrite_occurrences(
        &self,
        root: &Syntax,
        root_span: Span,
        replacements: &[(Span, Span)],
    ) -> Syntax {
        enum Task {
            Enter(Syntax, Span),
            Finish(Syntax, usize, bool),
        }
        let replacements: HashMap<Span, Span> = replacements.iter().copied().collect();
        let mut tasks = vec![Task::Enter(root.clone(), root_span)];
        let mut values: Vec<(Syntax, bool)> = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                Task::Enter(mut node, mut span) => {
                    let mut replaced = false;
                    let mut transitions = 0;
                    while let Some(&target) = replacements.get(&span) {
                        transitions += 1;
                        assert!(
                            transitions <= replacements.len(),
                            "cyclic type expansion replacements"
                        );
                        span = target;
                        node = self
                            .origin(span)
                            .expect("expanded type has an origin")
                            .syntax
                            .clone();
                        replaced = true;
                    }
                    let nodes: Vec<Syntax> = match node.node() {
                        Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                            xs.clone()
                        }
                        Node::Quote(x) | Node::Unquote(x) | Node::Splice(x) => vec![x.clone()],
                        _ => Vec::new(),
                    };
                    if nodes.is_empty() {
                        values.push((node, replaced));
                        continue;
                    }
                    let spans = self
                        .children
                        .get(&span)
                        .cloned()
                        .unwrap_or_else(|| nodes.iter().map(Syntax::span).collect());
                    assert_eq!(nodes.len(), spans.len(), "origin children match syntax");
                    tasks.push(Task::Finish(node, nodes.len(), replaced));
                    for (child, span) in nodes.into_iter().zip(spans).rev() {
                        tasks.push(Task::Enter(child, span));
                    }
                }
                Task::Finish(node, count, replaced) => {
                    let children = values.split_off(values.len() - count);
                    let changed = children.iter().any(|(_, changed)| *changed);
                    let result = if changed {
                        node.replace_children(
                            children.into_iter().map(|(child, _)| child).collect(),
                        )
                    } else {
                        node
                    };
                    values.push((result, changed || replaced));
                }
            }
        }
        values.pop().expect("one rewritten syntax root").0
    }

    /// Attach this invocation and any enclosing invocations to every node in
    /// the newly appended result. Called after each successful conversion.
    pub fn inherit_call_chain(&mut self, root: Span, invocation: Span, macro_name: &str) {
        let parent = self.origin(invocation).cloned();
        let current = self
            .origin(root)
            .expect("expanded root has an origin")
            .call_site
            .clone();
        let mut chain = parent.map_or_else(Vec::new, |o| o.call_chain);
        chain.push(Invocation {
            syntax: current,
            macro_name: macro_name.to_owned(),
        });
        for (span, origin) in &mut self.spans {
            if root.start <= span.start && span.end <= root.end {
                origin.call_chain = chain.clone();
            }
        }
        for (start, origin) in &mut self.starts {
            if root.start <= *start && *start < root.end {
                origin.call_chain = chain.clone();
            }
        }
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
                Node::Float(_) | Node::TypeView(_) => charge(32, 8)?,
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
                            let mut datum = current.to_datum();
                            self.text.push_str(&super::show(&datum));
                            let span = Span::new(start, self.position());
                            self.record(span, current, call_site, owner);
                            datum.span = span;
                            built.push(datum);
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
                    let child_spans = children.iter().map(|child| child.span).collect();
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
                    self.children.insert(span, child_spans);
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
            call_chain: Vec::new(),
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
    // ---- The same conversions to the tree of syntax three ----
    //
    // The datum is read in the rows of syntax three (§6.8) by plain
    // recursive readers: the caller is on a deep stack (`syn3::deep`).

    pub fn expression3_metered(
        &mut self,
        syntax: &Syntax,
        call_site: &Syntax,
        mut charge: impl FnMut(u64, u64) -> ChargeResult,
    ) -> Result<crate::syn3::ast::Expr, MeteredError> {
        self.preflight(syntax, &mut charge)
            .map_err(MeteredError::Resource)?;
        self.expression3(syntax, call_site)
            .map_err(MeteredError::Syntax)
    }
    pub fn pattern3_metered(
        &mut self,
        syntax: &Syntax,
        call_site: &Syntax,
        mut charge: impl FnMut(u64, u64) -> ChargeResult,
    ) -> Result<crate::syn3::ast::Pattern, MeteredError> {
        self.preflight(syntax, &mut charge)
            .map_err(MeteredError::Resource)?;
        self.pattern3(syntax, call_site)
            .map_err(MeteredError::Syntax)
    }
    pub fn ty3_metered(
        &mut self,
        syntax: &Syntax,
        call_site: &Syntax,
        mut charge: impl FnMut(u64, u64) -> ChargeResult,
    ) -> Result<crate::syn3::ast::Type, MeteredError> {
        self.preflight(syntax, &mut charge)
            .map_err(MeteredError::Resource)?;
        self.ty3(syntax, call_site).map_err(MeteredError::Syntax)
    }
    pub fn items3_metered(
        &mut self,
        syntax: &Syntax,
        call_site: &Syntax,
        mut charge: impl FnMut(u64, u64) -> ChargeResult,
    ) -> Result<Vec<crate::syn3::ast::Item>, MeteredError> {
        self.preflight(syntax, &mut charge)
            .map_err(MeteredError::Resource)?;
        self.items3(syntax, call_site).map_err(MeteredError::Syntax)
    }
    pub fn expression3(
        &mut self,
        syntax: &Syntax,
        call_site: &Syntax,
    ) -> Result<crate::syn3::ast::Expr, ScopedError> {
        let datum = self.append(syntax, call_site);
        let tree = crate::syn3::deep::deep(|| crate::syn3::lisp::expr_from_datum(&datum))
            .map_err(|error| self.error(error))?;
        let mut old = crate::syn3::lower::lower_expr(&tree).map_err(|error| self.error(error))?;
        self.transition.record_expression(&mut old);
        self.transition.expression(tree.span.0);
        Ok(tree)
    }
    pub fn pattern3(
        &mut self,
        syntax: &Syntax,
        call_site: &Syntax,
    ) -> Result<crate::syn3::ast::Pattern, ScopedError> {
        let datum = self.append(syntax, call_site);
        let tree = crate::syn3::deep::deep(|| crate::syn3::lisp::pattern_from_datum(&datum))
            .map_err(|error| self.error(error))?;
        let mut old =
            crate::syn3::lower::lower_pattern(&tree).map_err(|error| self.error(error))?;
        self.transition.record_pattern(&mut old);
        self.transition.pattern(tree.span.0);
        Ok(tree)
    }
    pub fn ty3(
        &mut self,
        syntax: &Syntax,
        call_site: &Syntax,
    ) -> Result<crate::syn3::ast::Type, ScopedError> {
        let datum = self.append(syntax, call_site);
        let tree = crate::syn3::deep::deep(|| crate::syn3::lisp::type_from_datum(&datum))
            .map_err(|error| self.error(error))?;
        syntax3::syn3::facts::Transition::validate_type(&tree)
            .map_err(|error| self.error(error))?;
        self.transition.ty(tree.span.0);
        Ok(tree)
    }
    pub fn items3(
        &mut self,
        syntax: &Syntax,
        call_site: &Syntax,
    ) -> Result<Vec<crate::syn3::ast::Item>, ScopedError> {
        let datum = self.append(syntax, call_site);
        let tree = crate::syn3::deep::deep(|| crate::syn3::lisp::items_from_datum(&datum))
            .map_err(|error| self.error(error))?;
        let mut old = crate::syn3::lower::lower_items(&tree).map_err(|error| self.error(error))?;
        self.transition.record_items(&mut old);
        Ok(tree)
    }

    pub fn expression(&mut self, syntax: &Syntax, call_site: &Syntax) -> Result<Expr, ScopedError> {
        let datum = self.append(syntax, call_site);
        if super::syntax::ROWS3 {
            return crate::syn3::deep::deep(|| {
                let tree = crate::syn3::lisp::expr_from_datum(&datum)?;
                crate::syn3::lower::lower_expr(&tree)
            })
            .map_err(|error| self.error(error));
        }
        super::parse_expression(&datum).map_err(|error| self.error(error))
    }
    pub fn pattern(&mut self, syntax: &Syntax, call_site: &Syntax) -> Result<Pattern, ScopedError> {
        let datum = self.append(syntax, call_site);
        if super::syntax::ROWS3 {
            return crate::syn3::deep::deep(|| {
                let tree = crate::syn3::lisp::pattern_from_datum(&datum)?;
                crate::syn3::lower::lower_pattern(&tree)
            })
            .map_err(|error| self.error(error));
        }
        super::parse_pattern(&datum).map_err(|error| self.error(error))
    }
    pub fn ty(&mut self, syntax: &Syntax, call_site: &Syntax) -> Result<TypeExpr, ScopedError> {
        let datum = self.append(syntax, call_site);
        if super::syntax::ROWS3 {
            return crate::syn3::deep::deep(|| {
                let tree = crate::syn3::lisp::type_from_datum(&datum)?;
                crate::syn3::lower::lower_type(&tree)
            })
            .map_err(|error| self.error(error));
        }
        super::parse_type(&datum).map_err(|error| self.error(error))
    }
    pub fn items(&mut self, syntax: &Syntax, call_site: &Syntax) -> Result<Vec<Item>, ScopedError> {
        let datum = self.append(syntax, call_site);
        if super::syntax::ROWS3 {
            return crate::syn3::deep::deep(|| {
                let tree = crate::syn3::lisp::items_from_datum(&datum)?;
                crate::syn3::lower::lower_items(&tree)
            })
            .map_err(|error| self.error(error));
        }
        super::parse_items(&datum).map_err(|error| self.error(error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lisp::syntax::{Node, Scope};
    use std::collections::BTreeSet;

    fn name(text: &str, module: u32, span: Span, mark: Option<u64>) -> Syntax {
        let datum = Datum {
            kind: DatumKind::Name(text.into()),
            span,
            docs: Vec::new(),
        };
        Syntax::from_datum(
            &datum,
            &Scope {
                module,
                marks: mark.into_iter().collect(),
            },
        )
    }

    #[test]
    fn nested_expansions_keep_every_call_and_the_real_template() {
        let mut source = ScopedSource::new("outer(x)".into());
        let call = name("outer", 0, Span::new(0, 8), None);
        let first_template = name("inner", 1, Span::new(4, 9), Some(11));
        let first = source.expression(&first_template, &call).unwrap();
        source.inherit_call_chain(first.span, call.span(), "outer");
        let nested_call = source.origin(first.span).unwrap().syntax.clone();
        let second_template = name("value", 2, Span::new(7, 12), Some(12));
        let second = source.expression(&second_template, &nested_call).unwrap();
        source.inherit_call_chain(second.span, first.span, "inner");
        let origin = source.origin(second.span).unwrap();
        assert_eq!(origin.syntax.scope().module, 2);
        assert_eq!(origin.syntax.span(), Span::new(7, 12));
        assert_eq!(origin.call_chain.len(), 2);
        assert_eq!(origin.call_chain[0].macro_name, "outer");
        assert_eq!(origin.call_chain[0].syntax.span(), Span::new(0, 8));
        assert_eq!(origin.call_chain[1].macro_name, "inner");
        assert_eq!(origin.call_chain[1].syntax.scope().module, 1);
    }

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
                drop(datum);
                drop(source);
                drop(syntax);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn rewriting_one_duplicate_occurrence_preserves_the_other() {
        let scope = Scope {
            module: 0,
            marks: BTreeSet::new(),
        };
        let parsed = crate::lisp::parse("(Pair X X)").unwrap();
        let original = Syntax::from_datum(&parsed[0], &scope);
        let Node::Round(parts) = original.node() else {
            panic!("round");
        };
        let shared = parts[1].clone();
        let root =
            original.replace_children(vec![parts[0].clone(), shared.clone(), shared.clone()]);
        let replacement = Syntax::from_datum(&crate::lisp::parse("Int").unwrap()[0], &scope);
        let mut source = ScopedSource::new(String::new());
        let datum = source.append(&root, &root);
        let DatumKind::Round(children) = &datum.kind else {
            panic!("round");
        };
        let old = children[1].span;
        let target = source.append(&replacement, &root).span;
        let rewritten = source.rewrite_occurrences(&root, datum.span, &[(old, target)]);
        assert_eq!(rewritten.show(), "(Pair Int X)");
        let Node::Round(parts) = rewritten.node() else {
            panic!("round");
        };
        assert!(parts[2].identical(&shared));
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
