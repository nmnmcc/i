//! A traversal of the tree in source order. The visitor sees every
//! expression, pattern, type and block twice: before its children
//! (`enter`) and after them.

use super::ast::*;

pub trait Visitor {
    fn expr(&mut self, _e: &mut Expr, _enter: bool) {}
    fn pattern(&mut self, _p: &mut Pattern, _enter: bool) {}
    fn ty(&mut self, _t: &mut Type, _enter: bool) {}
    fn block(&mut self, _b: &mut Block, _enter: bool) {}
}

pub fn module(m: &mut Module, v: &mut impl Visitor) {
    for i in &mut m.items {
        item(i, v);
    }
}

pub fn item(i: &mut Item, v: &mut impl Visitor) {
    for a in &mut i.attributes {
        if let Some(args) = &mut a.args {
            ditems(args, v);
        }
    }
    match &mut i.kind {
        ItemKind::TypeDef(def) => {
            for t in &mut def.head {
                ty(t, v);
            }
            match &mut def.rhs {
                TypeRhs::Opaque => {}
                TypeRhs::Data(items) => titems(items, v),
                TypeRhs::Type(t) => ty(t, v),
            }
        }
        ItemKind::Bind(b) => binding(b, v),
        ItemKind::Decl { ty: t, .. } => ty(t, v),
        ItemKind::Macro { args, .. } => ditems(args, v),
    }
}

fn binding(b: &mut Binding, v: &mut impl Visitor) {
    pattern(&mut b.pattern, v);
    expr(&mut b.value, v);
}

fn ditems(items: &mut [DItem], v: &mut impl Visitor) {
    for i in items {
        match i {
            DItem::Pos(e) | DItem::Label(_, e) | DItem::Spread(e) => expr(e, v),
        }
    }
}

fn titems(items: &mut [TItem], v: &mut impl Visitor) {
    for i in items {
        match i {
            TItem::Pos(t) | TItem::Label(_, t) | TItem::Spread(t) => ty(t, v),
        }
    }
}

fn dpitems(items: &mut [DPItem], v: &mut impl Visitor) {
    for i in items {
        match i {
            DPItem::Pos(p) | DPItem::Key(_, p) => pattern(p, v),
            DPItem::Rest(_) => {}
        }
    }
}

fn format(f: &mut Option<Format>, v: &mut impl Visitor) {
    if let Some(arg) = f.as_mut().and_then(|f| f.arg.as_mut()) {
        expr(arg, v);
    }
}

pub fn block(b: &mut Block, v: &mut impl Visitor) {
    v.block(b, true);
    for s in &mut b.stmts {
        match s {
            Stmt::Bind(b) => binding(b, v),
            Stmt::Expr(e) => expr(e, v),
        }
    }
    v.block(b, false);
}

pub fn function(f: &mut Function, v: &mut impl Visitor) {
    for p in &mut f.params {
        for c in &mut p.choices {
            pattern(c, v);
        }
    }
    if let Some(g) = &mut f.guard {
        expr(g, v);
    }
    for s in f.services.iter_mut().flatten() {
        ty(&mut s.ty, v);
    }
    if let Some(r) = &mut f.ret {
        ty(r, v);
    }
    block(&mut f.body, v);
}

pub fn expr(e: &mut Expr, v: &mut impl Visitor) {
    v.expr(e, true);
    match &mut e.kind {
        ExprKind::Quote(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::String(_)
        | ExprKind::Name { .. } => {}
        ExprKind::Interp(parts) => {
            for p in parts {
                if let InterpPart::Expr(e) = p {
                    expr(e, v);
                }
            }
        }
        ExprKind::Data(items) => ditems(items, v),
        ExprKind::List { items, rest } => {
            for i in items {
                expr(i, v);
            }
            if let Some(r) = rest {
                expr(r, v);
            }
        }
        ExprKind::BitArray(segments) => {
            for s in segments {
                expr(&mut s.value, v);
                format(&mut s.format, v);
            }
        }
        ExprKind::Block(b) => block(b, v),
        ExprKind::Chain(fs) => {
            for f in fs {
                function(f, v);
            }
        }
        ExprKind::Call { callee, args } => {
            expr(callee, v);
            ditems(args, v);
        }
        ExprKind::Provide { target, services } => {
            expr(target, v);
            expr(services, v);
        }
        ExprKind::Field { target, .. }
        | ExprKind::Index { target, .. }
        | ExprKind::Unary { expr: target, .. } => expr(target, v),
        ExprKind::Binary { lhs, rhs, .. } | ExprKind::Pipe { lhs, rhs } => {
            expr(lhs, v);
            expr(rhs, v);
        }
    }
    v.expr(e, false);
}

pub fn pattern(p: &mut Pattern, v: &mut impl Visitor) {
    v.pattern(p, true);
    match &mut p.kind {
        PatternKind::Quote(q) => {
            for (_, capture) in &mut q.captures {
                pattern(capture, v);
            }
        }
        PatternKind::Var(_)
        | PatternKind::Discard(_)
        | PatternKind::Int { .. }
        | PatternKind::Float { .. }
        | PatternKind::String(_)
        | PatternKind::Prefix { .. } => {}
        PatternKind::Typed { ty: t, .. } => ty(t, v),
        PatternKind::Ctor { args, .. } => {
            if let Some(args) = args {
                dpitems(args, v);
            }
        }
        PatternKind::Data(items) => dpitems(items, v),
        PatternKind::List { items, .. } | PatternKind::And(items) => {
            for i in items {
                pattern(i, v);
            }
        }
        PatternKind::BitArray(segments) => {
            for s in segments {
                pattern(&mut s.value, v);
                format(&mut s.format, v);
            }
        }
    }
    v.pattern(p, false);
}

pub fn ty(t: &mut Type, v: &mut impl Visitor) {
    v.ty(t, true);
    match &mut t.kind {
        TypeKind::Lit(_) | TypeKind::ValueOf { .. } | TypeKind::Hole => {}
        TypeKind::Named { args, .. }
        | TypeKind::Call { args, .. }
        | TypeKind::Union(args)
        | TypeKind::Inter(args)
        | TypeKind::Diff(args) => {
            for a in args {
                ty(a, v);
            }
        }
        TypeKind::Data(items) => titems(items, v),
        TypeKind::Fn(f) => {
            ty(&mut f.param, v);
            if let Some(s) = &mut f.services {
                ty(s, v);
            }
            ty(&mut f.ret, v);
        }
    }
    v.ty(t, false);
}
