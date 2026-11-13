//! Preserve written provider contracts before local generalization (§7.3).
//! The keys identify a typed binding and its one adjacent provided use.
use crate::span::Span;
use crate::syn3::{ast as n, deep, walk};
use std::collections::{HashMap, HashSet};

pub type Key = (Span, Span, Span);

enum Flat<'a> {
    Bind(&'a n::Binding),
    Env(&'a n::Expr),
    Other,
}

#[derive(Default)]
struct Plans {
    keys: HashSet<Key>,
    bindings: HashSet<(Span, Span)>,
    environments: HashSet<Span>,
    counts: Vec<HashMap<String, usize>>,
    generics: Vec<Vec<String>>,
    lisp: bool,
    generated_from: Option<u32>,
    transition: super::Transition,
}

impl Plans {
    fn keeps_generalization(&self, b: &n::Binding) -> bool {
        let n::PatternKind::Typed { ty, .. } = &b.pattern.kind else {
            return false;
        };
        struct Boundary<'a> {
            generics: &'a [Vec<String>],
            open: bool,
            function: bool,
            lisp: bool,
            generated_from: Option<u32>,
            matches: HashSet<Span>,
        }
        impl walk::Visitor for Boundary<'_> {
            fn ty(&mut self, t: &mut n::Type, enter: bool) {
                if enter {
                    self.open |= matches!(t.kind, n::TypeKind::Hole)
                        || matches!(&t.kind, n::TypeKind::Named { module: None, name, args }
                            if args.is_empty() && self.generics.iter().flatten().any(|g| g == &name.name));
                }
            }
            fn expr(&mut self, e: &mut n::Expr, enter: bool) {
                if enter {
                    if let n::ExprKind::Pipe { rhs, .. } = &e.kind
                        && (!self.lisp
                            || self
                                .generated_from
                                .is_some_and(|start| e.span.0.start >= start))
                        && let n::ExprKind::Chain(fs) = &rhs.kind
                        && super::pre::is_match(fs, false)
                    {
                        self.matches.insert(rhs.span.0);
                    }
                    self.function |= matches!(&e.kind, n::ExprKind::Chain(fs)
                        if !fs.is_empty() && !self.matches.contains(&e.span.0));
                }
            }
        }
        let mut boundary = Boundary {
            generics: &self.generics,
            open: false,
            function: false,
            lisp: self.lisp,
            generated_from: self.generated_from,
            matches: HashSet::new(),
        };
        walk::ty(&mut ty.clone(), &mut boundary);
        if boundary.open {
            return false;
        }
        walk::expr(&mut b.value.clone(), &mut boundary);
        boundary.function
    }
    fn flatten_block<'a>(&self, b: &'a n::Block, out: &mut Vec<Flat<'a>>) {
        for (i, s) in b.stmts.iter().enumerate() {
            match s {
                n::Stmt::Bind(b) => {
                    if !self.bindings.contains(&(b.span.0, b.pattern.span.0)) {
                        out.push(Flat::Bind(b));
                    }
                }
                n::Stmt::Expr(n::Expr {
                    kind: n::ExprKind::Provide { target, services },
                    ..
                }) if i + 1 == b.stmts.len() => self.provide(target, services, out),
                _ => out.push(Flat::Other),
            }
        }
    }
    fn provide<'a>(&self, target: &'a n::Expr, services: &'a n::Expr, out: &mut Vec<Flat<'a>>) {
        if let n::ExprKind::Data(items) = &services.kind {
            let apart = super::pre::provide_apart(items);
            if apart {
                out.extend(
                    items
                        .iter()
                        .filter_map(|i| matches!(i, n::DItem::Pos(_)).then_some(Flat::Other)),
                );
            }
            for item in items {
                if let n::DItem::Pos(e) = item {
                    out.push(if apart || self.environments.contains(&e.span.0) {
                        Flat::Other
                    } else {
                        Flat::Env(e)
                    });
                }
            }
        }
        match &target.kind {
            n::ExprKind::Block(b) => self.flatten_block(b, out),
            n::ExprKind::Provide { target, services } => self.provide(target, services, out),
            _ => out.push(Flat::Other),
        }
    }
}

impl walk::Visitor for Plans {
    fn binding(&mut self, b: &mut n::Binding, enter: bool) {
        if enter {
            self.generics.push(
                super::pre::binding_generics(
                    b,
                    !matches!(&b.pattern.kind,
                    n::PatternKind::Typed { ty, .. } if self.transition.is_type(ty.span.0)),
                )
                .into_iter()
                .map(|g| g.name.clone())
                .collect(),
            );
        } else {
            self.generics.pop().expect("binding entry");
        }
    }
    fn expr(&mut self, e: &mut n::Expr, enter: bool) {
        if enter
            && let n::ExprKind::Name { module: None, name } = &e.kind
            && let Some(counts) = self.counts.last_mut()
        {
            *counts.entry(name.name.clone()).or_default() += 1;
        }
    }
    fn block(&mut self, b: &mut n::Block, enter: bool) {
        if enter {
            self.counts.push(HashMap::new());
            return;
        }
        let mut counts = self.counts.pop().expect("block entry");
        let mut flat = Vec::new();
        self.flatten_block(b, &mut flat);
        // The old lowerer counts uses after processing nested blocks, once
        // before processing this block. Keep that order, including shadowing.
        let mut removed = Vec::new();
        let mut i = 0;
        while i + 1 < flat.len() {
            let contract = match (&flat[i], &flat[i + 1]) {
                (Flat::Bind(b), Flat::Env(e)) => match (&b.pattern.kind, &e.kind) {
                    (
                        n::PatternKind::Typed { binder, .. },
                        n::ExprKind::Name { module: None, name },
                    ) if !binder.discard
                        && binder.name == name.name
                        && counts.get(&binder.name) == Some(&1)
                        && !self.keeps_generalization(b) =>
                    {
                        Some(((b.span.0, b.pattern.span.0, e.span.0), binder.name.clone()))
                    }
                    _ => None,
                },
                _ => None,
            };
            if let Some((key, name)) = contract {
                self.keys.insert(key);
                self.bindings.insert((key.0, key.1));
                self.environments.insert(key.2);
                removed.push(name);
                flat.remove(i);
                flat[i] = Flat::Other;
            } else {
                i += 1;
            }
        }
        for name in removed {
            *counts.get_mut(&name).expect("provided name was counted") -= 1;
        }
        if let Some(parent) = self.counts.last_mut() {
            for (name, count) in counts {
                *parent.entry(name).or_default() += count;
            }
        }
    }
}

pub fn module(m: &n::Module) -> HashSet<Key> {
    module_with(m, false, None, &super::Transition::default())
}
pub fn module_with(
    m: &n::Module,
    lisp: bool,
    generated_from: Option<u32>,
    transition: &super::Transition,
) -> HashSet<Key> {
    deep::deep(|| {
        let mut plans = Plans {
            lisp,
            generated_from,
            transition: transition.clone(),
            ..Default::default()
        };
        walk::module(&mut m.clone(), &mut plans);
        plans.keys
    })
}
pub fn expression(e: &n::Expr) -> HashSet<Key> {
    expression_with(e, &super::Transition::default())
}
pub fn expression_with(e: &n::Expr, transition: &super::Transition) -> HashSet<Key> {
    deep::deep(|| {
        let mut plans = Plans {
            transition: transition.clone(),
            ..Default::default()
        };
        walk::expr(&mut e.clone(), &mut plans);
        plans.keys
    })
}
pub fn pattern(p: &n::Pattern) -> HashSet<Key> {
    deep::deep(|| {
        let mut plans = Plans::default();
        walk::pattern(&mut p.clone(), &mut plans);
        plans.keys
    })
}
