//! Groups for printing a tree that was not read from the normal surface.
//!
//! A group `{ e }` is a block in the tree, and the printer writes the tree as
//! it is. A tree read from the Lisp surface, made by a macro or by the
//! converter has no groups, so they are put in here before it is printed.

use super::ast::*;

/// Puts a group `{ e }` where the grammar of syntax three reads an operand
/// of a stronger level than `e` has: today's `|>` is stronger than `<>` and
/// the comparisons, and is the weakest operator now (§7.3, stage 5's row).
/// A group is a block in the tree, so the converter writes it.
pub fn regroup(module: &mut Module) {
    for item in &mut module.items {
        for a in &mut item.attributes {
            if let Some(args) = &mut a.args {
                group_items(args);
            }
        }
        match &mut item.kind {
            ItemKind::Bind(b) => {
                group_pattern(&mut b.pattern);
                group_expr(&mut b.value);
            }
            ItemKind::Macro { args, .. } => group_items(args),
            ItemKind::TypeDef(_) | ItemKind::Decl { .. } => {}
        }
    }
}

fn group_items(items: &mut [DItem]) {
    for item in items {
        match item {
            DItem::Pos(e) | DItem::Label(_, e) | DItem::Spread(e) => group_expr(e),
        }
    }
}

fn group_block(b: &mut Block) {
    for s in &mut b.stmts {
        match s {
            Stmt::Bind(b) => {
                group_pattern(&mut b.pattern);
                group_expr(&mut b.value);
            }
            Stmt::Expr(e) => group_expr(e),
        }
    }
}

fn group_format(f: &mut Option<Format>) {
    if let Some(arg) = f.as_mut().and_then(|f| f.arg.as_mut()) {
        group_expr(arg);
    }
}

fn group_pattern(p: &mut Pattern) {
    match &mut p.kind {
        PatternKind::Ctor {
            args: Some(items), ..
        }
        | PatternKind::Data(items) => {
            for i in items {
                match i {
                    DPItem::Pos(p) | DPItem::Key(_, p) => group_pattern(p),
                    DPItem::Rest(_) => {}
                }
            }
        }
        PatternKind::List { items, .. } | PatternKind::And(items) => {
            items.iter_mut().for_each(group_pattern)
        }
        PatternKind::BitArray(segments) => {
            for s in segments {
                group_pattern(&mut s.value);
                group_format(&mut s.format);
            }
        }
        _ => {}
    }
}

/// Makes `e` an operand of at least the level `min`.
fn at_least(e: &mut Expr, min: i8) {
    group_expr(e);
    let literal =
        min == super::print::POSTFIX + 1 && matches!(e.kind, ExprKind::Int(_) | ExprKind::Float(_));
    if super::print::level(e) < min.min(super::print::POSTFIX) || literal {
        let span = e.span;
        let inner = std::mem::replace(e, Expr::new(ExprKind::Data(Vec::new()), span.0));
        *e = Expr {
            kind: ExprKind::Block(Block {
                stmts: vec![Stmt::Expr(inner)],
                span,
            }),
            span,
        };
    }
}

fn group_expr(e: &mut Expr) {
    use super::print::{LOWEST, PIPE, POSTFIX};
    match &mut e.kind {
        ExprKind::Quote(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::String(_)
        | ExprKind::Name { .. } => {}
        ExprKind::Interp(parts) => {
            for p in parts {
                if let InterpPart::Expr(e) = p {
                    group_expr(e);
                }
            }
        }
        ExprKind::Data(items) => group_items(items),
        ExprKind::List { items, rest } => {
            items.iter_mut().for_each(group_expr);
            if let Some(r) = rest {
                group_expr(r);
            }
        }
        ExprKind::BitArray(segments) => {
            for s in segments {
                group_expr(&mut s.value);
                group_format(&mut s.format);
            }
        }
        ExprKind::Block(b) => group_block(b),
        ExprKind::Chain(fs) => {
            for f in fs {
                for p in &mut f.params {
                    p.choices.iter_mut().for_each(group_pattern);
                }
                if let Some(g) = &mut f.guard {
                    group_expr(g);
                }
                group_block(&mut f.body);
            }
        }
        ExprKind::Call { callee, args } => {
            // The target of a call, `.f` or `.k` is a postfix expression,
            // and not a number.
            at_least(callee, POSTFIX + 1);
            group_items(args);
        }
        ExprKind::Field { target, .. } | ExprKind::Index { target, .. } => {
            at_least(target, POSTFIX + 1)
        }
        ExprKind::Provide { target, services } => {
            at_least(target, LOWEST);
            at_least(services, POSTFIX);
        }
        ExprKind::Unary { expr, .. } => at_least(expr, POSTFIX),
        ExprKind::Binary { op, lhs, rhs } => {
            let lvl = op.level() as i8;
            at_least(lhs, lvl);
            at_least(rhs, lvl + 1);
        }
        ExprKind::Pipe { lhs, rhs } => {
            at_least(lhs, PIPE);
            at_least(rhs, PIPE + 1);
        }
    }
}
