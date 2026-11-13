//! Removed by the final deletion script, after the engine/resolver flip.
use super::tree::*;
use ilang_syntax::ast;

pub fn parse(text: &str, lisp: bool) -> Module {
    let tree = if lisp {
        ilang_syntax::lisp::parse_source(text).0
    } else {
        ilang_syntax::parse_module(text).0
    };
    let mut calls = Vec::new();
    let mut visit = |e: &ast::Expr| {
        let ast::ExprKind::Call { callee, .. } = &e.kind else {
            return;
        };
        let (module, name) = match &callee.kind {
            ast::ExprKind::Name { module, name } => (module.clone(), name.clone()),
            ast::ExprKind::Field { target, name } => match &target.kind {
                ast::ExprKind::Name {
                    module: None,
                    name: base,
                } => (Some(base.clone()), name.clone()),
                _ => return,
            },
            _ => return,
        };
        calls.push(Call {
            module,
            name,
            callee: callee.span,
            span: e.span,
        });
    };
    for i in &tree.items {
        match &i.kind {
            ast::ItemKind::Binding(b) => ast_exprs(&b.value, &mut visit),
            ast::ItemKind::MacroItem { args, .. } => {
                args.iter().for_each(|a| ast_exprs(&a.value, &mut visit))
            }
            _ => {}
        }
    }
    Module {
        doc: tree.doc,
        imports: tree
            .imports
            .into_iter()
            .map(|i| Import {
                alias: i.alias,
                names: i.names,
                path: i.path,
                span: i.span,
            })
            .collect(),
        exports: tree
            .exports
            .into_iter()
            .map(|i| Export { name: i.name })
            .collect(),
        imports_span: tree.imports_span,
        exports_span: tree.exports_span,
        calls,
        items: tree
            .items
            .into_iter()
            .map(|i| Item {
                doc: i.doc,
                is_macro: i
                    .attributes
                    .iter()
                    .any(|a| a.module.is_none() && a.name.name == "macro"),
                span: i.span,
                kind: match i.kind {
                    ast::ItemKind::TypeDef(d) => ItemKind::TypeDef {
                        name: d.name,
                        params: d.params,
                        alias_pattern: d.alias_pattern,
                        constructors: match d.body {
                            ast::TypeBody::Custom(cs) => cs
                                .into_iter()
                                .map(|c| Constructor {
                                    name: c.name,
                                    span: c.span,
                                })
                                .collect(),
                            _ => Vec::new(),
                        },
                    },
                    ast::ItemKind::Binding(b) => ItemKind::Binding {
                        binder: match &b.pattern.kind {
                            ast::PatternKind::Var(name) => Some(ast::Ident {
                                name: name.clone(),
                                span: b.pattern.span,
                            }),
                            ast::PatternKind::Typed { binder, .. } => Some(ast::Ident {
                                name: binder.name.clone(),
                                span: binder.span,
                            }),
                            _ => None,
                        },
                        type_params: b.type_params,
                    },
                    ast::ItemKind::Declaration { type_params, .. } => {
                        ItemKind::Declaration { type_params }
                    }
                    ast::ItemKind::MacroItem { .. } => ItemKind::MacroItem,
                },
            })
            .collect(),
    }
}

/// Calls `f` with every expression of a written expression.
pub(crate) fn ast_exprs(e: &ast::Expr, f: &mut dyn FnMut(&ast::Expr)) {
    use ast::ExprKind as K;
    f(e);
    match &e.kind {
        K::Tuple(xs) => xs.iter().for_each(|x| ast_exprs(x, f)),
        K::Record { base, fields } => {
            if let Some(b) = base {
                ast_exprs(b, f);
            }
            fields.iter().for_each(|(_, x)| ast_exprs(x, f));
        }
        K::List { items, rest } => {
            items.iter().for_each(|x| ast_exprs(x, f));
            if let Some(r) = rest {
                ast_exprs(r, f);
            }
        }
        K::BitArray(segs) => segs.iter().for_each(|s| ast_exprs(&s.value, f)),
        K::Block(b) => ast_block(b, f),
        K::Fn(func) => ast_block(&func.body, f),
        K::Case { subject, clauses } => {
            ast_exprs(subject, f);
            ast_clauses(clauses, f);
        }
        K::Clauses(clauses) => ast_clauses(clauses, f),
        K::Call { callee, args } => {
            ast_exprs(callee, f);
            args.iter().for_each(|a| ast_exprs(&a.value, f));
        }
        K::Field { target, .. } | K::TupleIndex { target, .. } => ast_exprs(target, f),
        K::RecordUpdate {
            constructor,
            base,
            fields,
        } => {
            ast_exprs(constructor, f);
            ast_exprs(base, f);
            fields.iter().for_each(|(_, x)| ast_exprs(x, f));
        }
        K::Unary { expr, .. } => ast_exprs(expr, f),
        K::Binary { lhs, rhs, .. } => {
            ast_exprs(lhs, f);
            ast_exprs(rhs, f);
        }
        K::Quote(_) | K::Int(_) | K::Float(_) | K::String(_) | K::Name { .. } => {}
    }
}

fn ast_clauses(clauses: &[ast::Clause], f: &mut dyn FnMut(&ast::Expr)) {
    for c in clauses {
        if let Some(g) = &c.guard {
            ast_exprs(g, f);
        }
        ast_exprs(&c.body, f);
    }
}

fn ast_block(b: &ast::Block, f: &mut dyn FnMut(&ast::Expr)) {
    for s in &b.stmts {
        match s {
            ast::Stmt::Bind(b) => ast_exprs(&b.value, f),
            ast::Stmt::Env { value, .. } => ast_exprs(value, f),
            ast::Stmt::Expr(e) => ast_exprs(e, f),
        }
    }
}
