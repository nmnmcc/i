//! Editor queries retain written-source facts without another expression or
//! type tree, so the compiler's source nodes remain the authority for spans.
use super::*;
use ilang_syntax::syn3::{self, ast as n, ast::resolved::Ident};

#[derive(Debug, PartialEq, Default)]
pub struct Module {
    pub doc: Vec<String>,
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
    pub items: Vec<Item>,
    pub imports_span: Option<Span>,
    pub exports_span: Option<Span>,
    pub calls: Vec<Call>,
}
#[derive(Debug, PartialEq)]
pub struct Import {
    pub alias: Option<Ident>,
    pub names: Vec<Ident>,
    pub path: Vec<Ident>,
    pub span: Span,
}
#[derive(Debug, PartialEq)]
pub struct Export {
    pub name: Ident,
}
#[derive(Debug, PartialEq)]
pub struct Item {
    pub doc: Vec<String>,
    pub is_macro: bool,
    pub kind: ItemKind,
    pub span: Span,
}
#[derive(Debug, PartialEq)]
pub enum ItemKind {
    TypeDef {
        name: Ident,
        params: Vec<Ident>,
        alias_pattern: bool,
        constructors: Vec<Constructor>,
    },
    Binding {
        binder: Option<Ident>,
        type_params: Vec<Ident>,
    },
    Declaration {
        type_params: Vec<Ident>,
    },
    MacroItem,
}
#[derive(Debug, PartialEq)]
pub struct Constructor {
    pub name: Ident,
    pub span: Span,
}
#[derive(Debug, PartialEq)]
pub struct Call {
    pub module: Option<Ident>,
    pub name: Ident,
    pub callee: Span,
    pub span: Span,
}
fn id(i: &n::Ident) -> Ident {
    Ident {
        name: i.name.clone(),
        span: i.span.0,
    }
}

pub(super) fn from_new(tree: &n::Module, lisp: bool) -> Module {
    let normalized = crate::resolve::normalize_source(tree);
    let tree = &normalized;
    let (type_params, member_of, absorbed) = crate::resolve::source_facts(tree, lisp);
    let mut out = Module {
        doc: tree.doc.clone(),
        imports: tree
            .imports
            .iter()
            .map(|i| Import {
                alias: i.alias.as_ref().map(id),
                names: i.names.iter().map(id).collect(),
                path: i.path.iter().map(id).collect(),
                span: i.span.0,
            })
            .collect(),
        exports: tree
            .exports
            .iter()
            .filter(|e| e.constructors || !absorbed.contains(&e.name.name))
            .map(|e| Export { name: id(&e.name) })
            .collect(),
        imports_span: tree.imports.iter().map(|i| i.span.0).reduce(Span::to),
        exports_span: tree.exports.iter().map(|i| i.span.0).reduce(Span::to),
        ..Module::default()
    };
    for (index, i) in tree.items.iter().enumerate() {
        if member_of.contains_key(&index) {
            continue;
        }
        let kind = match &i.kind {
            n::ItemKind::TypeDef(d) => {
                let alias_pattern = d.alias_pattern;
                let mut constructors: Vec<Constructor> = tree
                    .items
                    .iter()
                    .enumerate()
                    .filter_map(|(j, i)| {
                        if member_of.get(&j) != Some(&index) {
                            return None;
                        }
                        let n::ItemKind::TypeDef(d) = &i.kind else {
                            return None;
                        };
                        Some(Constructor {
                            name: id(&d.name),
                            span: i.span.0,
                        })
                    })
                    .collect();
                if constructors.is_empty() && matches!(d.rhs, n::TypeRhs::Data(_)) {
                    constructors.push(Constructor {
                        name: id(&d.name),
                        span: i.span.0,
                    });
                }
                let params = if alias_pattern {
                    Vec::new()
                } else {
                    d.head
                        .iter()
                        .map(|t| match &t.kind {
                            n::TypeKind::Named {
                                module: None,
                                name,
                                args,
                            } if args.is_empty() => Some(id(name)),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>()
                        .unwrap_or_default()
                };
                ItemKind::TypeDef {
                    name: id(&d.name),
                    params,
                    alias_pattern,
                    constructors,
                }
            }
            n::ItemKind::Bind(b) => ItemKind::Binding {
                binder: match &b.pattern.kind {
                    n::PatternKind::Var(name) => Some(Ident {
                        name: name.clone(),
                        span: b.pattern.span.0,
                    }),
                    n::PatternKind::Typed { binder, .. } => Some(Ident {
                        name: binder.name.clone(),
                        span: binder.span.0,
                    }),
                    _ => None,
                },
                type_params: type_params
                    .get(&(i.span.0, b.pattern.span.0))
                    .cloned()
                    .unwrap_or_default(),
            },
            n::ItemKind::Decl { ty, .. } => ItemKind::Declaration {
                type_params: match &ty.kind {
                    n::TypeKind::Fn(f) => f.generics.iter().map(id).collect(),
                    _ => Vec::new(),
                },
            },
            n::ItemKind::Macro { .. } => ItemKind::MacroItem,
        };
        out.items.push(Item {
            doc: i.doc.clone(),
            is_macro: i
                .attributes
                .iter()
                .any(|a| a.module.is_none() && a.name.name == "macro"),
            kind,
            span: i.span.0,
        });
    }
    struct Calls(Vec<Call>);
    impl syn3::walk::Visitor for Calls {
        fn expr(&mut self, e: &mut n::Expr, enter: bool) {
            if !enter {
                return;
            }
            let n::ExprKind::Call { callee, .. } = &e.kind else {
                return;
            };
            let (module, name) = match &callee.kind {
                n::ExprKind::Name { module, name } => (module.as_ref().map(id), id(name)),
                n::ExprKind::Field { target, name } => match &target.kind {
                    n::ExprKind::Name {
                        module: None,
                        name: base,
                    } => (Some(id(base)), id(name)),
                    _ => return,
                },
                _ => return,
            };
            self.0.push(Call {
                module,
                name,
                callee: callee.span.0,
                span: e.span.0,
            });
        }
    }
    let mut calls = Calls(Vec::new());
    syn3::walk::module(&mut tree.clone(), &mut calls);
    out.calls = calls.0;
    out
}
