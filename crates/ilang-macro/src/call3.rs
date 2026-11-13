//! `call` on the tree of syntax three: extracts macro call candidates
//! without evaluating or traversing arguments.
//!
//! The candidates are those of `call` on the lowered tree: a call whose
//! first item is a spread is a record update and never a macro call, and a
//! pipe into a chain is a match or a pipe into a function.
use ilang_syntax::syn3::ast::{DItem, Expr, ExprKind, Ident, Type, TypeKind};

pub struct ExprCall<'a> {
    pub module: Option<&'a Ident>,
    pub name: &'a Ident,
    /// A pipe contributes its left operand before the written arguments.
    pub prefix: Option<&'a Expr>,
    pub args: &'a [DItem],
}

/// The label and the value of a call's item. A spread that is not the first
/// item is an error of the reader; its value stands at its position.
pub fn argument(item: &DItem) -> (Option<&Ident>, &Expr) {
    match item {
        DItem::Pos(value) | DItem::Spread(value) => (None, value),
        DItem::Label(label, value) => (Some(label), value),
    }
}

/// `C(..base, f = e)`: a record update.
pub fn is_update(args: &[DItem]) -> bool {
    matches!(args.first(), Some(DItem::Spread(_)))
}

fn callee(e: &Expr) -> Option<(Option<&Ident>, &Ident)> {
    match &e.kind {
        ExprKind::Name { module, name } => Some((module.as_ref(), name)),
        ExprKind::Field { target, name } => match &target.kind {
            ExprKind::Name {
                module: None,
                name: alias,
            } => Some((Some(alias), name)),
            _ => None,
        },
        _ => None,
    }
}

pub fn expression(e: &Expr) -> Option<ExprCall<'_>> {
    match &e.kind {
        ExprKind::Call {
            callee: function,
            args,
        } if !is_update(args) => {
            let (module, name) = callee(function)?;
            Some(ExprCall {
                module,
                name,
                prefix: None,
                args,
            })
        }
        ExprKind::Pipe { lhs, rhs } => {
            let (function, args) = match &rhs.kind {
                ExprKind::Call { args, .. } if is_update(args) => return None,
                ExprKind::Call { callee, args } => (callee.as_ref(), args.as_slice()),
                _ => (rhs.as_ref(), &[][..]),
            };
            let (module, name) = callee(function)?;
            Some(ExprCall {
                module,
                name,
                prefix: Some(lhs),
                args,
            })
        }
        _ => None,
    }
}

pub struct TypeCall<'a> {
    pub module: Option<&'a Ident>,
    pub name: &'a Ident,
    pub args: &'a [Type],
}

pub fn ty(ty: &Type) -> Option<TypeCall<'_>> {
    match &ty.kind {
        TypeKind::Call { module, name, args } => Some(TypeCall {
            module: module.as_ref(),
            name,
            args,
        }),
        _ => None,
    }
}
