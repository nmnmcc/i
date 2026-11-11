//! Extract macro call candidates without evaluating or traversing arguments.
use ilang_syntax::ast::{Arg, BinOp, Expr, ExprKind, Ident, TypeArg, TypeExpr, TypeKind};

pub struct ExprCall<'a> {
    pub module: Option<&'a Ident>,
    pub name: &'a Ident,
    /// A pipe contributes its left operand before the written arguments.
    pub prefix: Option<&'a Expr>,
    pub args: &'a [Arg],
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
        } => {
            let (module, name) = callee(function)?;
            Some(ExprCall {
                module,
                name,
                prefix: None,
                args,
            })
        }
        ExprKind::Binary {
            op: BinOp::Pipe,
            lhs,
            rhs,
        } => {
            let (function, args) = match &rhs.kind {
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
    pub args: &'a [TypeArg],
}

pub fn ty(ty: &TypeExpr) -> Option<TypeCall<'_>> {
    match &ty.kind {
        TypeKind::Call { module, name, args } => Some(TypeCall {
            module: module.as_ref(),
            name,
            args,
        }),
        _ => None,
    }
}
