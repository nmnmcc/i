//! The order of top-level values (§2.5) and the groups of top-level
//! bindings (§4.7).
//!
//! For a binding `b` of a module, `uses(b)` are the bindings of this module
//! whose names are in its value, and `outside(b)` are those used outside the
//! function bodies of its value. Then:
//! - the full needs of `b` are the bindings reached from `b` by `uses`;
//! - the needs of a binding with a direct value are `outside(b)` and their
//!   needs; the needs of any other binding are its full needs.

use crate::code;
use crate::hir::*;
use ilang_package::Diagnostic;
use std::collections::BTreeSet;

/// What a binding uses: `(binding, inside a function body)`.
fn uses(program: &Program, m: ModuleId, b: &Binding) -> Vec<(u32, bool)> {
    references(&b.pattern, &b.value)
        .into_iter()
        .filter_map(|(d, inside)| {
            if d.module != m {
                return None;
            }
            match program.def(d).kind {
                DefKind::Binding { binding, .. } => Some((binding, inside)),
                _ => None,
            }
        })
        .collect()
}

/// All resolved value references in a binding, including across modules.
pub fn references(pattern: &Pattern, value: &Expr) -> Vec<(DefId, bool)> {
    let mut out = Vec::new();
    let mut w = Walker { out: &mut out };
    w.pattern(pattern, false);
    w.expr(value, false);
    out
}

struct Walker<'a> {
    out: &'a mut Vec<(DefId, bool)>,
}

impl Walker<'_> {
    fn expr(&mut self, e: &Expr, inside: bool) {
        match &e.kind {
            ExprKind::Name(ValueRef::Def(d)) => self.out.push((*d, inside)),
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Name(_)
            | ExprKind::Error => {}
            ExprKind::Tuple(items) => items.iter().for_each(|x| self.expr(x, inside)),
            ExprKind::List { items, rest } => {
                items
                    .iter()
                    .chain(rest.as_deref())
                    .for_each(|x| self.expr(x, inside));
            }
            ExprKind::BitArray(segs) => {
                for s in segs {
                    self.expr(&s.value, inside);
                    self.format(s.format.as_ref(), inside);
                }
            }
            ExprKind::Block(b) => self.block(b, inside),
            ExprKind::Fn(f) => {
                for p in &f.params {
                    if let Param::Pattern { pattern, .. } = p {
                        self.pattern(pattern, true);
                    }
                }
                self.block(&f.body, true);
            }
            ExprKind::Case { subject, clauses } => {
                self.expr(subject, inside);
                for c in clauses {
                    c.patterns.iter().for_each(|p| self.pattern(p, inside));
                    c.guard.iter().for_each(|g| self.expr(g, inside));
                    self.expr(&c.body, inside);
                }
            }
            ExprKind::Call { callee, args } => {
                self.expr(callee, inside);
                args.iter().for_each(|a| self.expr(&a.value, inside));
            }
            ExprKind::Field { target, .. } | ExprKind::TupleIndex { target, .. } => {
                self.expr(target, inside)
            }
            ExprKind::RecordUpdate { base, fields, .. } => {
                self.expr(base, inside);
                fields.iter().for_each(|(_, x)| self.expr(x, inside));
            }
            ExprKind::Unary { expr, .. } => self.expr(expr, inside),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.expr(lhs, inside);
                self.expr(rhs, inside);
            }
        }
    }

    fn block(&mut self, b: &Block, inside: bool) {
        for s in &b.stmts {
            match s {
                Stmt::Bind(b) => {
                    self.pattern(&b.pattern, inside);
                    self.expr(&b.value, inside);
                }
                Stmt::Env { value, .. } => self.expr(value, inside),
                Stmt::Expr(e) => self.expr(e, inside),
            }
        }
    }

    fn format(&mut self, f: Option<&Format>, inside: bool) {
        if let Some(arg) = f.and_then(|f| f.arg.as_ref()) {
            self.expr(arg, inside);
        }
    }

    /// Patterns only use names in bit array lengths.
    fn pattern(&mut self, p: &Pattern, inside: bool) {
        match &p.kind {
            PatternKind::Named { pattern, .. } => self.pattern(pattern, inside),
            PatternKind::Constructor { args, .. }
            | PatternKind::Tuple(args)
            | PatternKind::Error(args) => args.iter().for_each(|a| self.pattern(a, inside)),
            PatternKind::List { items, .. } => items.iter().for_each(|a| self.pattern(a, inside)),
            PatternKind::BitArray(segs) => {
                for s in segs {
                    self.pattern(&s.value, inside);
                    self.format(s.format.as_ref(), inside);
                }
            }
            _ => {}
        }
    }
}

/// The needs and full needs of every top-level binding of a module (§2.5),
/// as indexes in `Module::bindings`.
pub fn needs(program: &Program, m: ModuleId) -> Vec<(BTreeSet<u32>, BTreeSet<u32>)> {
    let module = program.module(m);
    let used: Vec<Vec<(u32, bool)>> = module
        .bindings
        .iter()
        .map(|b| uses(program, m, &b.binding))
        .collect();
    let direct: Vec<bool> = module.bindings.iter().map(|b| b.binding.direct).collect();
    let n = module.bindings.len();
    let mut out = Vec::with_capacity(n);
    for start in 0..n {
        // Full needs: reached by every use.
        let full = reach(&used, start as u32, |_, _| true);
        // Needs: from a direct binding, only uses outside function bodies,
        // until a binding that is not direct is reached; from there on, its
        // full needs.
        let mut needs = BTreeSet::new();
        let mut stack = vec![(start as u32, false)];
        let mut seen = BTreeSet::new();
        while let Some((b, full_mode)) = stack.pop() {
            if !seen.insert((b, full_mode)) {
                continue;
            }
            let full_mode = full_mode || !direct[b as usize];
            for &(c, inside) in &used[b as usize] {
                if full_mode || !inside {
                    needs.insert(c);
                    stack.push((c, full_mode));
                }
            }
        }
        out.push((needs, full));
    }
    out
}

fn reach(used: &[Vec<(u32, bool)>], start: u32, take: impl Fn(u32, bool) -> bool) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    let mut stack = vec![start];
    while let Some(b) = stack.pop() {
        for &(c, inside) in &used[b as usize] {
            if take(c, inside) && out.insert(c) {
                stack.push(c);
            }
        }
    }
    out
}

/// Fills `program.order`, and reports the bindings that are among their own
/// needs.
pub fn order(program: &mut Program, diags: &mut Vec<Diagnostic>) {
    let mut groups = Vec::new();
    let mut init = Vec::new();
    for m in module_order(program) {
        let module = program.module(m);
        let needs = needs(program, m);
        for (i, (need, _)) in needs.iter().enumerate() {
            if need.contains(&(i as u32)) {
                let b = &module.bindings[i];
                let name = b.defs.first().map_or("this binding".to_string(), |d| {
                    format!("`{}`", program.def(*d).name)
                });
                diags.push(Diagnostic::error(
                    code::NEEDS_ITSELF,
                    Some(module.source),
                    b.binding.pattern.span,
                    format!(
                        "the value of {name} is needed to work out itself (§2.5); only names inside the function bodies of a direct value may refer back"
                    ),
                ));
            }
        }
        // Groups: the strongly connected components by use.
        let mut g = petgraph::graph::DiGraph::<u32, ()>::new();
        let nodes: Vec<_> = (0..module.bindings.len() as u32)
            .map(|i| g.add_node(i))
            .collect();
        for (i, b) in module.bindings.iter().enumerate() {
            for (c, _) in uses(program, m, &b.binding) {
                g.update_edge(nodes[i], nodes[c as usize], ());
            }
        }
        for scc in petgraph::algo::tarjan_scc(&g) {
            let mut group: Vec<u32> = scc.iter().map(|n| g[*n]).collect();
            group.sort();
            groups.push(
                group
                    .into_iter()
                    .map(|index| BindingRef { module: m, index })
                    .collect(),
            );
        }
        // Init: each binding after its needs, and otherwise in source order.
        let n = module.bindings.len();
        let mut done = vec![false; n];
        for _ in 0..n {
            let next = (0..n)
                .find(|&i| {
                    !done[i]
                        && needs[i]
                            .0
                            .iter()
                            .all(|&c| done[c as usize] || c as usize == i)
                })
                .or_else(|| (0..n).find(|&i| !done[i]));
            let Some(i) = next else { break };
            done[i] = true;
            init.push(BindingRef {
                module: m,
                index: i as u32,
            });
        }
    }
    groups = type_order(program, groups, diags, false);
    program.order = Order { groups, init };
}

/// The modules, each after the modules it imports.
pub fn module_order(program: &Program) -> Vec<ModuleId> {
    let mut g = petgraph::graph::DiGraph::<u32, ()>::new();
    let nodes: Vec<_> = (0..program.modules.len() as u32)
        .map(|i| g.add_node(i))
        .collect();
    for (i, m) in program.modules.iter().enumerate() {
        // The prelude comes before every module.
        if let Some(p) = program.prelude
            && p.0 as usize != i
        {
            g.update_edge(nodes[i], nodes[p.0 as usize], ());
        }
        for t in &m.imports {
            g.update_edge(nodes[i], nodes[t.0 as usize], ());
        }
    }
    let mut out = Vec::new();
    for scc in petgraph::algo::tarjan_scc(&g) {
        let mut ms: Vec<u32> = scc.iter().map(|n| g[*n]).collect();
        ms.sort();
        out.extend(ms.into_iter().map(ModuleId));
    }
    out
}

#[derive(Clone)]
struct TypeRead {
    site: OfId,
    reader: BindingRef,
    annotation: ilang_syntax::span::Span,
}

/// Value-type edges order already computed expression SCCs. They never
/// change SCC membership, runtime needs, or initialization (§2.5, §4.7).
fn type_order(
    p: &mut Program,
    groups: Vec<Vec<BindingRef>>,
    diags: &mut Vec<Diagnostic>,
    reverse: bool,
) -> Vec<Vec<BindingRef>> {
    if p.of_sites.is_empty() && !reverse {
        return groups;
    }
    let mut owners = std::collections::HashMap::new();
    for (i, group) in groups.iter().enumerate() {
        for b in group {
            owners.insert(*b, i);
        }
    }
    let mut graph = petgraph::graph::DiGraph::<usize, Option<TypeRead>>::new();
    let nodes = (0..groups.len())
        .map(|i| graph.add_node(i))
        .collect::<Vec<_>>();
    let mut has_reads = false;
    for (i, group) in groups.iter().enumerate() {
        for b in group {
            for (index, _) in uses(p, b.module, &p.binding(*b).binding) {
                if let Some(&j) = owners.get(&BindingRef {
                    module: b.module,
                    index,
                }) && i != j
                {
                    // Keep any parallel type edge and its written provenance.
                    graph.add_edge(nodes[i], nodes[j], None);
                }
            }
            binding_types(&p.binding(*b).binding, &mut |t| {
                let mut sources = Vec::new();
                type_sources(p, t, &mut BTreeSet::new(), &mut sources);
                for (site, source) in sources {
                    let ValueRef::Def(d) = source else { continue };
                    let DefKind::Binding { binding, .. } = p.def(d).kind else {
                        continue;
                    };
                    let Some(&j) = owners.get(&BindingRef {
                        module: d.module,
                        index: binding,
                    }) else {
                        continue;
                    };
                    has_reads = true;
                    graph.add_edge(
                        nodes[i],
                        nodes[j],
                        Some(TypeRead {
                            site,
                            reader: *b,
                            annotation: t.span,
                        }),
                    );
                }
            });
        }
    }
    if !has_reads && !reverse {
        return groups;
    }
    let mut bad = BTreeSet::new();
    for scc in petgraph::algo::tarjan_scc(&graph) {
        let members = scc
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        let cyclic = scc.len() > 1 || scc.iter().any(|n| graph.contains_edge(*n, *n));
        if !cyclic {
            continue;
        }
        for n in &scc {
            bad.insert(graph[*n]);
        }
        for n in &scc {
            for edge in graph.edges(*n) {
                use petgraph::visit::EdgeRef;
                if !members.contains(&edge.target()) {
                    continue;
                }
                if let Some(read) = edge.weight() {
                    let info = &p.of_sites[read.site.0 as usize];
                    let from = read.reader;
                    let reader = p
                        .binding(from)
                        .defs
                        .first()
                        .map(|d| p.def(*d).name.as_str())
                        .unwrap_or("this binding");
                    let source = match info.value {
                        ValueRef::Def(d) => p.def(d).name.as_str(),
                        _ => "this value",
                    };
                    let goal = *n;
                    let (_, path) =
                        petgraph::algo::astar(&graph, edge.target(), |n| n == goal, |_| 1, |_| 0)
                            .expect("an edge inside an SCC has a return path");
                    let chain = std::iter::once(*n)
                        .chain(path)
                        .map(|n| {
                            groups[graph[n]]
                                .iter()
                                .flat_map(|b| &p.binding(*b).defs)
                                .map(|d| p.def(*d).name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .collect::<Vec<_>>()
                        .join(" -> ");
                    let mut aliases = Vec::new();
                    binding_types(&p.binding(read.reader).binding, &mut |t| {
                        if t.span == read.annotation {
                            let mut paths = Vec::new();
                            type_paths(p, t, &mut BTreeSet::new(), &mut Vec::new(), &mut paths);
                            if let Some((_, _, path)) =
                                paths.into_iter().find(|(site, _, _)| *site == read.site)
                            {
                                aliases = path;
                            }
                        }
                    });
                    let via = if aliases.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " through {}",
                            aliases
                                .iter()
                                .map(|id| p.ty(*id).name.as_str())
                                .collect::<Vec<_>>()
                                .join(" -> ")
                        )
                    };
                    diags.push(Diagnostic::error(code::VALUE_TYPE_CIRCLE,Some(p.module(from.module).source),read.annotation,
                        format!("the type of {source} is needed to type {reader}{via}, but the dependencies form a circle ({chain}); write the type instead")));
                }
            }
        }
    }
    // O4: failed components poison their dependents without another error.
    // Keep the expression SCCs intact; only recovery readiness is propagated.
    let mut pending = std::collections::VecDeque::from_iter(bad.iter().copied());
    while let Some(i) = pending.pop_front() {
        for n in graph.neighbors_directed(nodes[i], petgraph::Direction::Incoming) {
            if bad.insert(graph[n]) {
                pending.push_back(graph[n]);
            }
        }
    }
    for i in &bad {
        p.failed_type_groups.extend(groups[*i].iter().copied());
    }
    for (i, site) in p.of_sites.iter().enumerate() {
        let ValueRef::Def(d) = site.value else {
            continue;
        };
        let DefKind::Binding { binding, .. } = p.def(d).kind else {
            continue;
        };
        if owners
            .get(&BindingRef {
                module: d.module,
                index: binding,
            })
            .is_some_and(|i| bad.contains(i))
        {
            p.failed_reads.insert(OfId(i as u32));
        }
    }
    let mut remaining = (0..groups.len())
        .map(|i| {
            if bad.contains(&i) {
                0
            } else {
                graph.neighbors(nodes[i]).count()
            }
        })
        .collect::<Vec<_>>();
    let mut ready = (0..groups.len())
        .filter(|i| remaining[*i] == 0)
        .collect::<BTreeSet<_>>();
    let mut out = Vec::new();
    while let Some(next) = if reverse {
        ready.pop_last()
    } else {
        ready.pop_first()
    } {
        out.push(groups[next].clone());
        for n in graph.neighbors_directed(nodes[next], petgraph::Direction::Incoming) {
            let i = graph[n];
            if remaining[i] > 0 {
                remaining[i] -= 1;
                if remaining[i] == 0 {
                    ready.insert(i);
                }
            }
        }
    }
    debug_assert_eq!(out.len(), groups.len());
    out
}

/// Resolved value references reached through a written type and its aliases.
pub(crate) fn type_sources(
    p: &Program,
    t: &Type,
    seen: &mut BTreeSet<TypeId>,
    out: &mut Vec<(OfId, ValueRef)>,
) {
    walk_type(t, &mut |t| match &t.kind {
        TypeKind::Of { site, value, .. } => out.push((*site, *value)),
        TypeKind::Named {
            ty: TypeRef::Custom(id),
            ..
        } if seen.insert(*id) => {
            if let Some(sites) = p.alias_sources.get(id.0 as usize) {
                out.extend(
                    sites
                        .iter()
                        .map(|site| (*site, p.of_sites[site.0 as usize].value)),
                );
            }
        }
        _ => {}
    });
}

fn type_paths(
    p: &Program,
    t: &Type,
    seen: &mut BTreeSet<TypeId>,
    chain: &mut Vec<TypeId>,
    out: &mut Vec<(OfId, ValueRef, Vec<TypeId>)>,
) {
    walk_type(t, &mut |t| match &t.kind {
        TypeKind::Of { site, value, .. } => out.push((*site, *value, chain.clone())),
        TypeKind::Named {
            ty: TypeRef::Custom(id),
            ..
        } if seen.insert(*id) => {
            if let TypeBody::Alias(body) = &p.ty(*id).body {
                chain.push(*id);
                type_paths(p, body, seen, chain, out);
                chain.pop();
            }
        }
        _ => {}
    });
}

/// Test entry point for independent-group order invariance.
pub fn reverse_type_ties(p: &mut Program) {
    let groups = std::mem::take(&mut p.order.groups);
    p.order.groups = type_order(p, groups, &mut Vec::new(), true);
}
