//! Shape grouping on the native tree. The reserved call is an internal form;
//! it is resolved directly to HIR Multi, never reparsed or lowered.
use super::Transition;
use crate::span::Span;
use crate::syn3::{ast as n, walk};

fn shape(f: &n::Function) -> (usize, Vec<String>) {
    let mut labels: Vec<_> = f
        .params
        .iter()
        .filter_map(|p| p.key.as_ref())
        .map(|k| k.name.clone())
        .collect();
    labels.sort();
    (f.params.len() - labels.len(), labels)
}
fn multi(fs: &[n::Function]) -> bool {
    fs.first().is_some_and(|first| {
        fs.iter()
            .any(|f| f.params.iter().any(|p| p.key.is_some()) || shape(f) != shape(first))
    })
}
fn groups(fs: &[n::Function]) -> Vec<Vec<n::Function>> {
    let mut result: Vec<Vec<n::Function>> = Vec::new();
    for f in fs {
        let group = result.iter_mut().find(|g| shape(&g[0]) == shape(f));
        match group {
            Some(g) => g.push(f.clone()),
            None => result.push(vec![f.clone()]),
        }
    }
    result
}
fn canonical(fs: &mut [n::Function]) {
    for f in fs {
        let mut params = std::mem::take(&mut f.params);
        params.sort_by(|a, b| {
            a.key
                .as_ref()
                .map(|k| &k.name)
                .cmp(&b.key.as_ref().map(|k| &k.name))
        });
        for p in &mut params {
            p.key = None;
        }
        f.params = params;
    }
}
fn name(text: &str, at: Span) -> n::Expr {
    n::Expr::new(
        n::ExprKind::Name {
            module: None,
            name: n::Ident::new(text, at),
        },
        at,
    )
}
fn pack(groups: Vec<(String, n::Expr)>, at: Span) -> n::Expr {
    let at = Span::new(at.end.saturating_sub(1), at.end);
    n::Expr::new(
        n::ExprKind::Call {
            callee: Box::new(name("%multi", at)),
            args: groups
                .into_iter()
                .map(|(shape, value)| n::DItem::Label(n::Ident::new(shape, value.span.0), value))
                .collect(),
        },
        at,
    )
}
fn text(f: &n::Function) -> String {
    let (positions, labels) = shape(f);
    let mut text = positions.to_string();
    for label in labels {
        text.push(',');
        text.push_str(&label);
    }
    text
}
struct Normalize<'a> {
    transition: &'a mut Transition,
}
impl Normalize<'_> {
    fn binding(&mut self, b: n::Binding) -> Vec<n::Binding> {
        let (n::PatternKind::Var(name), n::ExprKind::Chain(fs)) = (&b.pattern.kind, &b.value.kind)
        else {
            return vec![b];
        };
        if !multi(fs) || self.transition.is_expression(b.value.span.0) {
            return vec![b];
        }
        let mut out = Vec::new();
        let mut values = Vec::new();
        for (i, mut group) in groups(fs).into_iter().enumerate() {
            let first = group[0].span.0;
            let at = Span::new(first.start, first.start + 1);
            let label = text(&group[0]);
            let cover = first.to(group.last().unwrap().span.0);
            canonical(&mut group);
            let hidden = format!("{name}%g{i}");
            out.push(n::Binding {
                pattern: n::Pattern::new(n::PatternKind::Var(hidden.clone()), at),
                value: n::Expr::new(n::ExprKind::Chain(group), cover),
                span: b.span,
            });
            values.push((label, self::name(&hidden, at)));
        }
        out.push(n::Binding {
            pattern: b.pattern.clone(),
            value: pack(values, b.value.span.0),
            span: b.span,
        });
        out
    }
}
impl walk::Visitor for Normalize<'_> {
    fn pattern(&mut self, p: &mut n::Pattern, enter: bool) {
        if enter
            && let n::PatternKind::Quote(q) = &mut p.kind
            && q.captures.is_empty()
            && let Ok(captures) = crate::syn3::lisp::quote_captures(&q.datum.0, true)
        {
            q.captures = captures;
        }
    }
    fn block(&mut self, b: &mut n::Block, enter: bool) {
        if !enter {
            return;
        }
        b.stmts = std::mem::take(&mut b.stmts)
            .into_iter()
            .flat_map(|s| match s {
                n::Stmt::Bind(b) => self.binding(b).into_iter().map(n::Stmt::Bind).collect(),
                s => vec![s],
            })
            .collect();
    }
    fn expr(&mut self, e: &mut n::Expr, enter: bool) {
        if !enter {
            return;
        }
        let n::ExprKind::Chain(fs) = &e.kind else {
            return;
        };
        if !multi(fs) {
            return;
        }
        let mut values = Vec::new();
        for mut group in groups(fs) {
            let label = text(&group[0]);
            let cover = group[0].span.0.to(group.last().unwrap().span.0);
            canonical(&mut group);
            values.push((label, n::Expr::new(n::ExprKind::Chain(group), cover)));
        }
        let standalone = self.transition.is_expression(e.span.0);
        *e = pack(values, e.span.0);
        if standalone {
            self.transition.expression(e.span.0);
        }
    }
}
pub fn module(module: &n::Module, transition: &mut Transition) -> n::Module {
    let mut module = module.clone();
    let mut normalize = Normalize { transition };
    let mut hidden = Vec::new();
    module.items = std::mem::take(&mut module.items)
        .into_iter()
        .flat_map(|mut item| {
            let n::ItemKind::Bind(binding) = &item.kind else {
                return vec![item];
            };
            let expanded = normalize.binding(binding.clone());
            let mut items = Vec::new();
            if expanded.len() > 1 {
                let n::PatternKind::Var(original) = &binding.pattern.kind else {
                    unreachable!()
                };
                for b in &expanded[..expanded.len() - 1] {
                    let n::PatternKind::Var(group) = &b.pattern.kind else {
                        unreachable!()
                    };
                    hidden.push((original.clone(), group.clone()));
                }
            }
            let last = expanded.len() - 1;
            for (i, binding) in expanded.into_iter().enumerate() {
                if i == last {
                    item.kind = n::ItemKind::Bind(binding);
                    items.push(item.clone());
                } else {
                    items.push(n::Item {
                        doc: Vec::new(),
                        attributes: Vec::new(),
                        span: binding.span,
                        kind: n::ItemKind::Bind(binding),
                    });
                }
            }
            items
        })
        .collect();
    module.exports = std::mem::take(&mut module.exports)
        .into_iter()
        .flat_map(|export| {
            let mut exports: Vec<_> = hidden
                .iter()
                .filter(|(name, _)| !export.constructors && *name == export.name.name)
                .map(|(_, group)| n::Export {
                    name: n::Ident::new(group, export.span.0),
                    constructors: false,
                    span: export.span,
                })
                .collect();
            exports.push(export);
            exports
        })
        .collect();
    walk::module(&mut module, &mut normalize);
    module
}
pub fn expression(e: &n::Expr, transition: &mut Transition) -> n::Expr {
    let mut e = e.clone();
    walk::expr(&mut e, &mut Normalize { transition });
    e
}

pub fn pattern(pattern: &mut n::Pattern) {
    crate::syn3::walk::pattern(
        pattern,
        &mut Normalize {
            transition: &mut Default::default(),
        },
    );
}
