//! Native validation and facts fixed before macro expansion. Names and spans
//! preserve the standalone context of generated fragments; no old nodes live here.
use super::{ast as n, walk};
use crate::{Diagnostic, span::Span};
use std::collections::{HashMap, HashSet};
pub mod groups;
pub mod pre;

#[derive(Clone, Debug, Default)]
pub struct Transition {
    pub(crate) complete: bool,
    pub(crate) parameters: HashMap<(Span, Span), Vec<(String, Span)>>,
    pub(crate) sums: HashMap<Span, Vec<Span>>,
    pub(crate) expressions: HashSet<Span>,
    pub(crate) types: HashSet<Span>,
    pub(crate) patterns: HashSet<Span>,
}
impl Transition {
    pub fn extend(&mut self, other: &Self) {
        self.complete |= other.complete;
        self.parameters.extend(other.parameters.clone());
        self.sums.extend(other.sums.clone());
        self.expressions.extend(&other.expressions);
        self.types.extend(&other.types);
        self.patterns.extend(&other.patterns);
    }
    pub fn is_expression(&self, span: Span) -> bool {
        self.expressions.contains(&span)
    }
    pub fn is_type(&self, span: Span) -> bool {
        self.types.contains(&span)
    }
    pub fn is_pattern(&self, span: Span) -> bool {
        self.patterns.contains(&span)
    }
    pub fn is_complete(&self) -> bool {
        self.complete
    }
    pub fn formals(&self, binding: Span, pattern: Span) -> Option<&[(String, Span)]> {
        self.parameters.get(&(binding, pattern)).map(Vec::as_slice)
    }
    pub fn sum_members(&self, parent: Span) -> Option<&[Span]> {
        self.sums.get(&parent).map(Vec::as_slice)
    }
    pub fn expression(&mut self, span: Span) {
        self.expressions.insert(span);
    }
    pub fn ty(&mut self, span: Span) {
        self.types.insert(span);
    }
    pub fn pattern(&mut self, span: Span) {
        self.patterns.insert(span);
    }

    fn binding_native(&mut self, b: &n::Binding, plan: &pre::Pre) {
        if let Some(params) = plan.type_params.get(&pre::addr(b)) {
            self.parameters.insert(
                (b.span.0, b.pattern.span.0),
                params.iter().map(|p| (p.name.clone(), p.span)).collect(),
            );
        }
    }
    fn quoted_native(&mut self, datum: &crate::lisp::Datum) {
        use crate::lisp::DatumKind;
        let mut work = vec![datum];
        while let Some(datum) = work.pop() {
            match &datum.kind {
                DatumKind::Unquote(inner) | DatumKind::Splice(inner) => {
                    self.expression(inner.span);
                    if let Ok(native) = super::lisp::expr_from_datum(inner) {
                        let _ = self.capture_expression(&native);
                    }
                }
                DatumKind::Quote(inner) => work.push(inner),
                DatumKind::Round(xs)
                | DatumKind::Square(xs)
                | DatumKind::Curly(xs)
                | DatumKind::Bits(xs) => work.extend(xs),
                _ => {}
            }
        }
    }
    fn visitor<'a>(&'a mut self, plan: &'a pre::Pre) -> Capture<'a> {
        Capture { facts: self, plan }
    }
    pub fn capture_module(module: &n::Module, lisp: bool) -> (Self, Vec<Diagnostic>) {
        Self::capture_module_with(module, lisp, false)
    }
    fn capture_module_with(
        module: &n::Module,
        lisp: bool,
        standalone: bool,
    ) -> (Self, Vec<Diagnostic>) {
        super::deep::deep(|| {
            let mut facts = Self::default();
            let mut module = groups::module(module, &mut Self::default());
            let plan = if standalone {
                pre::pre_items(&module.items)
            } else {
                pre::pre(&module, lisp)
            };
            facts.complete = true;
            let mut sums: HashMap<usize, Vec<usize>> = HashMap::new();
            for (member, parent) in &plan.member_of {
                sums.entry(*parent).or_default().push(*member);
            }
            for (parent, mut members) in sums {
                members.sort();
                let span = module.items[parent].span.0;
                let members: Vec<_> = members
                    .into_iter()
                    .map(|i| module.items[i].span.0)
                    .filter(|member| *member != span)
                    .collect();
                if !members.is_empty() {
                    facts.sums.insert(span, members);
                }
            }
            for item in &module.items {
                if let n::ItemKind::Bind(b) = &item.kind {
                    facts.binding_native(b, &plan);
                }
            }
            walk::module(&mut module, &mut facts.visitor(&plan));
            (facts, plan.errors)
        })
    }
    pub fn capture_expression(&mut self, expression: &n::Expr) -> Result<(), Diagnostic> {
        super::deep::deep(|| {
            let mut tree = groups::expression(expression, &mut Self::default());
            let plan = pre::pre_expr(&tree);
            if let Some(error) = plan.errors.first() {
                return Err(error.clone());
            }
            walk::expr(&mut tree, &mut self.visitor(&plan));
            self.expression(expression.span.0);
            Ok(())
        })
    }
    pub fn capture_pattern(&mut self, pattern: &n::Pattern) -> Result<(), Diagnostic> {
        super::deep::deep(|| {
            let mut tree = pattern.clone();
            groups::pattern(&mut tree);
            let plan = pre::pre_pattern(&tree);
            if let Some(error) = plan.errors.first() {
                return Err(error.clone());
            }
            walk::pattern(&mut tree, &mut self.visitor(&plan));
            self.pattern(pattern.span.0);
            Ok(())
        })
    }
    pub fn validate_type(ty: &n::Type) -> Result<(), Diagnostic> {
        super::deep::deep(|| {
            pre::pre_type(ty)
                .errors
                .into_iter()
                .next()
                .map_or(Ok(()), Err)
        })
    }
    pub fn capture_items(&mut self, items: &[n::Item]) -> Result<(), Diagnostic> {
        super::deep::deep(|| {
            let module = n::Module {
                items: items.to_vec(),
                ..Default::default()
            };
            let (facts, errors) = Self::capture_module_with(&module, false, true);
            if let Some(error) = errors.into_iter().next() {
                return Err(error);
            }
            self.extend(&facts);
            Ok(())
        })
    }
}
struct Capture<'a> {
    facts: &'a mut Transition,
    plan: &'a pre::Pre,
}
impl walk::Visitor for Capture<'_> {
    fn block(&mut self, b: &mut n::Block, enter: bool) {
        if enter {
            for s in &b.stmts {
                if let n::Stmt::Bind(b) = s {
                    self.facts.binding_native(b, self.plan);
                }
            }
        }
    }
    fn expr(&mut self, e: &mut n::Expr, enter: bool) {
        if enter && let n::ExprKind::Quote(q) = &e.kind {
            self.facts.quoted_native(&q.0);
        }
    }
}
pub fn quote_captures(
    q: &n::QuotePattern,
) -> Result<std::borrow::Cow<'_, [(bool, n::Pattern)]>, Diagnostic> {
    let parsed = super::lisp::quote_captures(&q.datum.0, true)?;
    Ok(if q.captures.is_empty() {
        std::borrow::Cow::Owned(parsed)
    } else {
        std::borrow::Cow::Borrowed(&q.captures)
    })
}
