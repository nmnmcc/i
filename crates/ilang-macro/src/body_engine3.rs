//! The parts of `body_engine` that read the tree, on the tree of syntax
//! three: the hook of the body walker, the encoding of a call's arguments,
//! and `is_direct`.
use crate::{
    body_engine::{BodyEngine, Error, metered_error},
    body_walk3::{Hook, Local},
    call3,
    prepare::MacroPosition,
    registry::Macro,
};
use ilang_syntax::lisp::{
    Datum,
    expansion::MacroArgument,
    syntax::{Scope, Syntax},
};
use ilang_syntax::span::Span;
use ilang_syntax::syn3::ast::{DItem, Expr, ExprKind, Ident, Item, Pattern, Stmt, Type};

/// A node that a macro can take as an argument.
pub trait Encode {
    /// The node in the rows of syntax three (§6.8), with the spans of
    /// `syn3::lisp::*_datum`.
    fn datum(&self) -> Datum;
    fn span(&self) -> Span;
}
impl Encode for Expr {
    fn datum(&self) -> Datum {
        ilang_syntax::syn3::lisp::expr_datum(self)
    }
    fn span(&self) -> Span {
        self.span.0
    }
}
impl Encode for Type {
    fn datum(&self) -> Datum {
        ilang_syntax::syn3::lisp::type_datum(self)
    }
    fn span(&self) -> Span {
        self.span.0
    }
}
impl Encode for Pattern {
    fn datum(&self) -> Datum {
        ilang_syntax::syn3::lisp::pattern_datum(self)
    }
    fn span(&self) -> Span {
        self.span.0
    }
}
impl Encode for Item {
    fn datum(&self) -> Datum {
        ilang_syntax::syn3::lisp::item_datum(self)
    }
    fn span(&self) -> Span {
        self.span.0
    }
}

/// The syntax value of a written node: the argument of a macro call. The
/// builder is a plain recursion over the node; the callers are on a deep
/// stack (`body_walk3::expressions`).
pub fn encode_node(node: &impl Encode, scope: &Scope) -> Syntax {
    Syntax::from_datum(&node.datum(), scope)
}

/// Whether a value is made without running any code. `is_module` answers
/// whether a name is a module alias at this place.
///
/// The answers are those of `BodyEngine::is_direct` on the lowered tree:
/// every chain is a function (`Fn` or `Clauses`); data with a label or a
/// spread is a record and not direct; a call with a leading spread is a
/// record update; an interpolation is a chain of `<>`; a pipe into a chain
/// is a match or a pipe into a function.
pub fn is_direct(expression: &Expr, mut is_module: impl FnMut(&Ident) -> bool) -> bool {
    fn ungroup(mut expression: &Expr) -> &Expr {
        while let ExprKind::Block(block) = &expression.kind {
            let [Stmt::Expr(inner)] = block.stmts.as_slice() else {
                break;
            };
            expression = inner;
        }
        expression
    }
    fn constructor(expression: &Expr) -> bool {
        matches!(&expression.kind, ExprKind::Name { name, .. } if name.name.chars().next().is_some_and(char::is_uppercase))
    }
    fn values(args: &[DItem]) -> impl Iterator<Item = &Expr> {
        args.iter().map(|arg| call3::argument(arg).1)
    }
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match &ungroup(expression).kind {
            ExprKind::Chain(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Name { .. } => {}
            ExprKind::Unary { expr, .. }
                if matches!(
                    ungroup(expr).kind,
                    ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::String(_)
                ) => {}
            ExprKind::Field { target, .. } => match &target.kind {
                ExprKind::Name { module: None, name } if is_module(name) => {}
                _ => return false,
            },
            ExprKind::Data(items) if items.iter().all(|i| matches!(i, DItem::Pos(_))) => {
                pending.extend(values(items))
            }
            ExprKind::List { items, rest } => {
                pending.extend(items);
                pending.extend(rest.as_deref());
            }
            ExprKind::Call { callee, args } if constructor(callee) && !call3::is_update(args) => {
                pending.extend(values(args))
            }
            ExprKind::Pipe { lhs, rhs } => {
                pending.push(lhs);
                match &rhs.kind {
                    ExprKind::Name { .. } if constructor(rhs) => {}
                    ExprKind::Call { callee, args }
                        if constructor(callee) && !call3::is_update(args) =>
                    {
                        pending.extend(values(args))
                    }
                    _ => return false,
                }
            }
            _ => return false,
        }
    }
    true
}

/// The arguments of an expression macro call: the left side of a pipe, then
/// the written items.
pub fn expression_arguments(
    written: &call3::ExprCall<'_>,
    mut encode: impl FnMut(&Expr) -> Syntax,
) -> Vec<MacroArgument> {
    let mut args = Vec::new();
    if let Some(prefix) = written.prefix {
        args.push(MacroArgument {
            label: None,
            syntax: encode(prefix),
        });
    }
    args.extend(item_arguments(written.args, encode));
    args
}

/// The written items of a call (an expression call, an item call, an
/// attribute) as macro arguments.
pub fn item_arguments(
    items: &[DItem],
    mut encode: impl FnMut(&Expr) -> Syntax,
) -> Vec<MacroArgument> {
    items
        .iter()
        .map(|item| {
            let (label, value) = call3::argument(item);
            MacroArgument {
                label: label.map(|label| label.name.clone()),
                syntax: encode(value),
            }
        })
        .collect()
}

impl BodyEngine<'_> {
    pub(crate) fn atom3(&self, id: &Ident) -> Syntax {
        if let Some(origin) = self.scoped.origin(id.span.0) {
            return origin.syntax.clone();
        }
        Syntax::from_datum(
            &Datum {
                kind: ilang_syntax::lisp::DatumKind::Name(id.name.clone()),
                span: id.span.0,
                docs: Vec::new(),
            },
            &self.scope(),
        )
    }
    pub(crate) fn resolve3(
        &self,
        module: Option<&Ident>,
        name: &Ident,
        locals: &[Local],
        depth: u32,
    ) -> Option<Macro> {
        let module = module.map(|m| self.atom3(m));
        self.registry.resolve(
            self.source,
            module.as_ref(),
            &self.atom3(name),
            &self.hygiene_locals(locals, depth),
        )
    }
    /// The syntax of a node: what a macro made it from, or the written node
    /// encoded.
    pub(crate) fn node_syntax(&self, node: &impl Encode) -> Syntax {
        if let Some(origin) = self.scoped.origin(node.span()) {
            return origin.syntax.clone();
        }
        encode_node(node, &self.scope())
    }
}

impl Hook for BodyEngine<'_> {
    type Error = Error;
    fn is_macro(&mut self, expression: &Expr, locals: &[Local], depth: u32) -> Result<bool, Error> {
        Ok(call3::expression(expression).is_some_and(|call| {
            self.resolve3(call.module, call.name, locals, depth)
                .is_some_and(|mac| mac.definition.signature.is_none())
        }))
    }
    fn expand(&mut self, expression: &Expr, locals: &[Local], depth: u32) -> Result<Expr, Error> {
        let written = call3::expression(expression).expect("body walker identified a call");
        let mac = self
            .resolve3(written.module, written.name, locals, depth)
            .expect("body walker resolved a macro");
        let args = expression_arguments(&written, |e| self.node_syntax(e));
        let call = self.node_syntax(expression);
        let invocation_span = expression.span.0;
        let (result, budget, transactions) = self.execute(
            &mac,
            args,
            &call,
            invocation_span,
            None,
            MacroPosition::Expression,
        )?;
        let start = u32::try_from(self.scoped.text().len() + 1).expect("source span overflow");
        let expression = self
            .scoped
            .expression3_metered(&result, &call, |work, words| {
                budget.charge(work, words).map_err(|e| Box::new(e) as _)
            })
            .map_err(|error| metered_error(error, &mac.definition.name, &call, invocation_span))?;
        let end = u32::try_from(self.scoped.text().len()).expect("source span overflow");
        self.scoped.inherit_call_chain(
            Span::new(start, end),
            invocation_span,
            &mac.definition.name,
        );
        self.transactions.extend(transactions);
        Ok(expression)
    }
    fn is_type_macro(&mut self, ty: &Type, locals: &[Local], depth: u32) -> Result<bool, Error> {
        Ok(call3::ty(ty).is_some_and(|call| {
            self.resolve3(call.module, call.name, locals, depth)
                .is_some()
        }))
    }
    fn expand_type(&mut self, ty: &Type, locals: &[Local], depth: u32) -> Result<Type, Error> {
        let written = call3::ty(ty).expect("body walker identified a type call");
        let mac = self
            .resolve3(written.module, written.name, locals, depth)
            .expect("body walker resolved a macro");
        let args = written
            .args
            .iter()
            .map(|arg| MacroArgument {
                label: None,
                syntax: self.node_syntax(arg),
            })
            .collect();
        let old_span = ty.span.0;
        let call = self.node_syntax(ty);
        let (result, budget, transactions) =
            self.execute(&mac, args, &call, old_span, None, MacroPosition::Type)?;
        let start = u32::try_from(self.scoped.text().len() + 1).expect("source span overflow");
        let ty = self
            .scoped
            .ty3_metered(&result, &call, |work, words| {
                budget.charge(work, words).map_err(|e| Box::new(e) as _)
            })
            .map_err(|error| metered_error(error, &mac.definition.name, &call, old_span))?;
        let end = u32::try_from(self.scoped.text().len()).expect("source span overflow");
        self.scoped
            .inherit_call_chain(Span::new(start, end), old_span, &mac.definition.name);
        self.type_replacements.push((old_span, ty.span.0, result));
        self.transactions.extend(transactions);
        self.item_replacements.push((old_span, ty.span.0, budget));
        Ok(ty)
    }
    fn depth_error(&mut self, span: Span) -> Error {
        Error::Depth(span)
    }
    fn is_direct(&mut self, expression: &Expr, locals: &[Local], depth: u32) -> bool {
        is_direct(expression, |name| {
            self.registry.is_module(
                self.source,
                &self.atom3(name),
                &self.hygiene_locals(locals, depth),
            )
        })
    }
}
