//! Permanent value reads use immutable candidates and checked solver trials
//! (§4.7; value-types R3–R7). Nothing here inspects a runtime value.
use super::infer::Checker;
use super::solve::Solver;
use super::{FnTy, Head, ShownFn, ShownType, Ty, V};
use crate::code;
use crate::hir::*;
use ilang_syntax::span::Span;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub(super) enum Read {
    /// A successful projection operation has no reusable source type.
    Projected,
    Ready(Ty),
    Failed,
}
#[derive(Clone, Debug)]
pub(super) struct Fixed {
    pub ty: Ty,
    pub module: ModuleId,
    pub span: Span,
}
#[derive(Default)]
pub(super) struct Reads {
    pub sites: HashMap<OfId, Read>,
    pub fixed: HashMap<V, Fixed>,
    pub aliases: HashMap<TypeId, Option<()>>,
    pub completed: HashMap<DefId, Result<Ty, &'static str>>,
    pub patterns: HashSet<ProjectionId>,
    pub sources: HashSet<DefId>,
    pub boundary: usize,
    pub error_start: usize,
}

struct Closure<'a> {
    s: &'a Solver,
    fixed: &'a HashMap<V, Fixed>,
    boundary: usize,
    level: u32,
    open: bool,
    visiting: HashSet<V>,
    pub equations: HashMap<V, Ty>,
    pub shapes: Vec<(V, Ty)>,
}

impl<'a> Closure<'a> {
    fn new(
        s: &'a Solver,
        fixed: &'a HashMap<V, Fixed>,
        boundary: usize,
        level: u32,
        open: bool,
    ) -> Self {
        Self {
            s,
            fixed,
            boundary,
            level,
            open,
            visiting: HashSet::new(),
            equations: HashMap::new(),
            shapes: Vec::new(),
        }
    }

    fn bounds(&self, root: V) -> Result<(Vec<V>, Vec<Ty>), &'static str> {
        let mut vars = Vec::new();
        let mut bounds = Vec::new();
        let mut stack = vec![root];
        let mut seen = HashSet::new();
        while let Some(v) = stack.pop() {
            if !seen.insert(v) {
                continue;
            }
            if let Some(f) = self.fixed.get(&v) {
                bounds.push(f.ty.clone());
                continue;
            }
            let st = &self.s.vars[v.0 as usize];
            if st.level > self.level {
                return Err(code::VALUE_TYPE_OPEN);
            }
            if self.open || (v.0 as usize) < self.boundary {
                return Err(code::VALUE_TYPE_NOT_FIXED);
            }
            vars.push(v);
            stack.extend(st.from.iter().copied());
            for t in &st.lower {
                if let Ty::Var(w) = t {
                    stack.push(*w);
                } else {
                    bounds.push(t.clone());
                }
            }
        }
        if bounds.is_empty() {
            return Err(code::VALUE_TYPE_NOT_FIXED);
        }
        Ok((vars, bounds))
    }

    fn reaches(&self, start: V, target: V) -> bool {
        let mut stack = vec![start];
        let mut seen = HashSet::new();
        while let Some(v) = stack.pop() {
            if v == target {
                return true;
            }
            if !seen.insert(v) || self.fixed.contains_key(&v) {
                continue;
            }
            let st = &self.s.vars[v.0 as usize];
            stack.extend(st.from.iter().copied());
            stack.extend(
                st.lower
                    .iter()
                    .filter_map(|t| if let Ty::Var(v) = t { Some(*v) } else { None }),
            );
        }
        false
    }

    fn close(&mut self, t: &Ty) -> Result<Ty, &'static str> {
        match t {
            Ty::Var(v) => {
                if let Some(f) = self.fixed.get(v) {
                    return Ok(f.ty.clone());
                }
                if let Some(t) = self.equations.get(v) {
                    return Ok(t.clone());
                }
                let (vars, bounds) = self.bounds(*v)?;
                if vars.iter().any(|v| self.visiting.contains(v)) {
                    return Err(code::VALUE_TYPE_NOT_FIXED);
                }
                let component = vars
                    .iter()
                    .copied()
                    .filter(|w| self.reaches(*w, *v))
                    .collect::<Vec<_>>();
                self.visiting.extend(component.iter().copied());
                let closed = bounds
                    .iter()
                    .map(|b| self.close(b))
                    .collect::<Result<Vec<_>, _>>()?;
                let t = self.normalize(Ty::Union(closed))?;
                for w in component {
                    self.visiting.remove(&w);
                    self.equations.insert(w, t.clone());
                }
                // A predecessor may have a strictly smaller closed candidate.
                // Only a pure-variable SCC has the same candidate on all nodes.
                for w in vars {
                    if w != *v && !self.equations.contains_key(&w) {
                        self.close(&Ty::Var(w))?;
                    }
                }
                Ok(t)
            }
            Ty::Top => Err(code::VALUE_TYPE_OPEN),
            Ty::Never => Ok(Ty::Never),
            Ty::Con(h, args) => Ok(Ty::Con(
                *h,
                args.iter()
                    .map(|t| self.close(t))
                    .collect::<Result<_, _>>()?,
            )),
            Ty::Fn(f) => Ok(Ty::Fn(Box::new(FnTy {
                params: f
                    .params
                    .iter()
                    .map(|(n, t)| Ok((n.clone(), self.close(t)?)))
                    .collect::<Result<_, &'static str>>()?,
                ret: self.close(&f.ret)?,
                err: self.close(&f.err)?,
                env: self.close(&f.env)?,
            }))),
            Ty::Union(ts) => {
                let ts = ts.iter().map(|t| self.close(t)).collect::<Result<_, _>>()?;
                self.normalize(Ty::Union(ts))
            }
            Ty::Inter(ts) => {
                let ts = ts.iter().map(|t| self.close(t)).collect::<Result<_, _>>()?;
                self.normalize(Ty::Inter(ts))
            }
            Ty::Diff(t, ts) => {
                let t = self.close(t)?;
                let ts = ts.iter().map(|t| self.close(t)).collect::<Result<_, _>>()?;
                self.normalize(Ty::Diff(Box::new(t), ts))
            }
            Ty::Both(a, b) => {
                let a = self.close(a)?;
                let b = self.close(b)?;
                if closed_equal(self.s, &a, &b) {
                    Ok(a)
                } else {
                    Err(code::VALUE_TYPE_NOT_FIXED)
                }
            }
        }
    }

    fn normalize(&self, t: Ty) -> Result<Ty, &'static str> {
        let mut scratch = Solver::new(self.s.variance.clone(), self.s.type_names.clone());
        let result = match t {
            Ty::Union(ts) => scratch.union(ts),
            Ty::Inter(ts) => scratch.inter(ts),
            Ty::Diff(t, ts) => scratch.diff(*t, ts),
            t => t,
        };
        if !scratch.failures.is_empty() || contains_var(&result) {
            Err(code::VALUE_TYPE_NOT_FIXED)
        } else {
            Ok(result)
        }
    }

    fn expose(&mut self, t: &Ty) -> Result<Ty, &'static str> {
        match t {
            Ty::Var(v) => {
                if let Some(f) = self.fixed.get(v) {
                    return Ok(f.ty.clone());
                }
                let (vars, bounds) = self.bounds(*v)?;
                // Only the outer shape is observed. Its original arguments,
                // including skipped arguments, retain their solver identities.
                let mut unique = Vec::new();
                for t in bounds {
                    if !unique.contains(&t) {
                        unique.push(t);
                    }
                }
                if unique.len() != 1 {
                    return Err(code::TYPE_PATTERN_MISMATCH);
                }
                let shape = unique.pop().unwrap();
                if shape.head().is_none() {
                    return Err(code::TYPE_PATTERN_MISMATCH);
                }
                for w in vars {
                    self.shapes.push((w, shape.clone()));
                }
                Ok(shape)
            }
            t => Ok(t.clone()),
        }
    }
}

fn contains_var(t: &Ty) -> bool {
    match t {
        Ty::Var(_) | Ty::Top => true,
        Ty::Con(_, ts) | Ty::Union(ts) | Ty::Inter(ts) => ts.iter().any(contains_var),
        Ty::Fn(f) => {
            f.params.iter().any(|(_, t)| contains_var(t))
                || [&f.ret, &f.err, &f.env].into_iter().any(contains_var)
        }
        Ty::Diff(t, ts) => contains_var(t) || ts.iter().any(contains_var),
        Ty::Both(a, b) => contains_var(a) || contains_var(b),
        Ty::Never => false,
    }
}

fn closed_equal(s: &Solver, a: &Ty, b: &Ty) -> bool {
    let mut scratch = Solver::new(s.variance.clone(), s.type_names.clone());
    scratch.constrain(a, b);
    scratch.constrain(b, a);
    scratch.failures.is_empty()
}

/// Substitute fixed forms without reading any live bound. Remaining variables
/// have no known shape, including variables narrowed by expected/flow facts.
fn shape(t: &Ty, fixed: &HashMap<V, Fixed>) -> Ty {
    match t {
        Ty::Var(v) => fixed.get(v).map_or(Ty::Top, |f| f.ty.clone()),
        Ty::Con(h, ts) => Ty::Con(*h, ts.iter().map(|t| shape(t, fixed)).collect()),
        Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
            params: f
                .params
                .iter()
                .map(|(n, t)| (n.clone(), shape(t, fixed)))
                .collect(),
            ret: shape(&f.ret, fixed),
            err: shape(&f.err, fixed),
            env: shape(&f.env, fixed),
        })),
        Ty::Union(ts) => Ty::Union(ts.iter().map(|t| shape(t, fixed)).collect()),
        Ty::Inter(ts) => Ty::Inter(ts.iter().map(|t| shape(t, fixed)).collect()),
        Ty::Diff(t, ts) => Ty::Diff(
            Box::new(shape(t, fixed)),
            ts.iter().map(|t| shape(t, fixed)).collect(),
        ),
        Ty::Both(a, b) => Ty::Both(Box::new(shape(a, fixed)), Box::new(shape(b, fixed))),
        Ty::Never | Ty::Top => t.clone(),
    }
}

fn completed(
    t: &Ty,
    s: &Solver,
    fixed: &HashMap<V, Fixed>,
    level: u32,
) -> Result<Ty, &'static str> {
    match t {
        Ty::Var(v) if !fixed.contains_key(v) => Err(if s.vars[v.0 as usize].level > level {
            code::VALUE_TYPE_OPEN
        } else {
            code::VALUE_TYPE_NOT_FIXED
        }),
        Ty::Var(v) => Ok(fixed[v].ty.clone()),
        Ty::Con(h, ts) => Ok(Ty::Con(
            *h,
            ts.iter()
                .map(|t| completed(t, s, fixed, level))
                .collect::<Result<_, _>>()?,
        )),
        Ty::Fn(f) => Ok(Ty::Fn(Box::new(FnTy {
            params: f
                .params
                .iter()
                .map(|(n, t)| Ok((n.clone(), completed(t, s, fixed, level)?)))
                .collect::<Result<_, &'static str>>()?,
            ret: completed(&f.ret, s, fixed, level)?,
            err: completed(&f.err, s, fixed, level)?,
            env: completed(&f.env, s, fixed, level)?,
        }))),
        Ty::Union(ts) => Ok(Ty::Union(
            ts.iter()
                .map(|t| completed(t, s, fixed, level))
                .collect::<Result<_, _>>()?,
        )),
        Ty::Inter(ts) => Ok(Ty::Inter(
            ts.iter()
                .map(|t| completed(t, s, fixed, level))
                .collect::<Result<_, _>>()?,
        )),
        Ty::Diff(t, ts) => Ok(Ty::Diff(
            Box::new(completed(t, s, fixed, level)?),
            ts.iter()
                .map(|t| completed(t, s, fixed, level))
                .collect::<Result<_, _>>()?,
        )),
        Ty::Both(a, b) => {
            let a = completed(a, s, fixed, level)?;
            let b = completed(b, s, fixed, level)?;
            if closed_equal(s, &a, &b) {
                Ok(a)
            } else {
                Err(code::VALUE_TYPE_NOT_FIXED)
            }
        }
        Ty::Never => Ok(Ty::Never),
        Ty::Top => Err(code::VALUE_TYPE_OPEN),
    }
}

impl Checker<'_> {
    pub(super) fn initialize_reads(&mut self) {
        if self.p.of_sites.is_empty() {
            return;
        }
        self.reads.sources = self
            .p
            .of_sites
            .iter()
            .filter_map(|s| {
                if let ValueRef::Def(d) = s.value {
                    Some(d)
                } else {
                    None
                }
            })
            .collect();
        for site in &self.p.failed_reads {
            self.reads.sites.insert(*site, Read::Failed);
        }
        for (id, scheme) in &self.defs {
            if !self.reads.sources.contains(id) {
                continue;
            }
            self.reads.completed.insert(
                *id,
                completed(&scheme.ty, &self.s, &self.reads.fixed, scheme.level),
            );
        }
    }

    pub(super) fn read_shape(&self, value: ValueRef, site: OfId) -> Ty {
        if let Some(Read::Ready(t)) = self.reads.sites.get(&site) {
            return t.clone();
        }
        let scheme = match value {
            ValueRef::Local(v) => self.locals.get(&v),
            ValueRef::Def(d) => self.defs.get(&d),
            ValueRef::Ctor(_) => None,
        };
        scheme.map_or(Ty::Top, |scheme| {
            let principal = if scheme.level == u32::MAX {
                scheme.ty.clone()
            } else {
                super::display::simplify(&self.s, &scheme.ty, Some(scheme.level))
                    .unwrap_or_else(|()| scheme.ty.clone())
            };
            shape(&principal, &self.reads.fixed)
        })
    }

    pub(super) fn read_annotation(&mut self, value: ValueRef, site: OfId) -> Ty {
        if let Some(read) = self.reads.sites.get(&site) {
            return match read {
                Read::Ready(t) => t.clone(),
                Read::Failed => Ty::Never,
                Read::Projected if self.recovery => Ty::Never,
                Read::Projected => panic!("projection source read as an annotation"),
            };
        }
        let scheme = match value {
            ValueRef::Local(v) => self.locals.get(&v),
            ValueRef::Def(d) => self.defs.get(&d),
            ValueRef::Ctor(_) => None,
        }
        .cloned();
        let Some(mut scheme) = scheme else {
            if !self.recovery {
                panic!("scheduled value read {site:?} has no source");
            }
            return Ty::Never;
        };
        if let ValueRef::Def(d) = value
            && self.reads.completed.contains_key(&d)
        {
            match fixed_scheme(&scheme.ty, &self.s, &self.reads.fixed, scheme.level) {
                Ok(t) => scheme.ty = t,
                Err(c) => {
                    self.read_error(site, c);
                    return Ty::Never;
                }
            }
        }
        self.instantiate(&scheme)
    }

    fn read_error(&mut self, site: OfId, c: &'static str) {
        let info = &self.p.of_sites[site.0 as usize];
        let mut message =
            "the value-derived type is still open after source completion".to_string();
        if c == code::TYPE_PATTERN_MISMATCH {
            message = "the source type does not match the alias pattern head or constant part"
                .to_string();
        } else if c == code::TYPE_PATTERN_FORM {
            message = "the alias pattern cannot be projected through this type form".to_string();
        } else if c == code::VALUE_TYPE_OPEN {
            message = "a permanent alias cannot contain a generalized variable or an unknown type"
                .to_string();
        } else if let ValueRef::Def(d) = info.value {
            let name = &self.p.def(d).name;
            let aliases = self
                .p
                .types
                .iter()
                .enumerate()
                .filter(|(i, a)| {
                    a.module == d.module && self.out.alias_types.contains_key(&TypeId(*i as u32))
                })
                .map(|(_, a)| a)
                .filter(|a| {
                    let TypeBody::Alias(t) = &a.body else {
                        return false;
                    };
                    let mut found = false;
                    walk_type(t, &mut |t| {
                        if matches!(t.kind, TypeKind::Of { value: ValueRef::Def(x), .. } if x == d)
                        {
                            found = true;
                        }
                    });
                    found
                })
                .map(|a| a.name.clone())
                .collect::<Vec<_>>();
            message = format!(
                "the type of {}.{name} is still open after source completion",
                self.p.module(d.module).path
            );
            if let Some(alias) = aliases.first() {
                message.push_str(&format!(
                    "; use {alias}, the alias made by {}",
                    self.p.module(d.module).path
                ));
                if d.module != info.module
                    && !self
                        .p
                        .types
                        .iter()
                        .any(|a| a.module == d.module && a.name == *alias && a.exported)
                {
                    message.push_str(&format!(
                        "; export {alias} from {}",
                        self.p.module(d.module).path
                    ));
                }
            } else {
                message.push_str("; define and export a closed alias next to the source, or write a fully closed source annotation");
            }
        }
        self.set_module(info.module);
        self.error(c, info.span, message);
        self.reads.sites.insert(site, Read::Failed);
    }

    pub(super) fn begin_reads(&mut self, group: &[BindingRef]) {
        self.reads.boundary = self.s.vars.len();
        self.reads.error_start = self.diags.len();
        if self.p.of_sites.iter().any(|site| site.permanent && matches!(site.value, ValueRef::Def(d) if group.iter().any(|b| b.module == d.module && self.p.binding(*b).defs.contains(&d)))) {
            self.s.observation = Some((self.s.vars.len(), self.reads.fixed.keys().copied().collect(), Default::default()));
        }
    }

    pub(super) fn finish_reads(&mut self, group: &[BindingRef]) {
        if self.p.of_sites.is_empty() {
            return;
        }
        let open = self
            .s
            .observation
            .take()
            .is_some_and(|(_, _, seen)| !seen.borrow().is_empty());
        let failed = self.diags[self.reads.error_start..]
            .iter()
            .any(|d| d.is_error());
        let defs = group
            .iter()
            .flat_map(|b| self.p.binding(*b).defs.iter().copied())
            .collect::<HashSet<_>>();
        if !defs.iter().any(|d| self.reads.sources.contains(d)) {
            return;
        }
        if failed {
            for (i, site) in self.p.of_sites.iter().enumerate() {
                if matches!(site.value,ValueRef::Def(d) if defs.contains(&d)) {
                    self.reads.sites.insert(OfId(i as u32), Read::Failed);
                }
            }
            for d in &defs {
                self.reads
                    .completed
                    .insert(*d, Err(code::VALUE_TYPE_NOT_FIXED));
            }
            self.complete_ready_reads();
            return;
        }
        let completion_start = self.diags.len();
        let has_demands=self.p.of_sites.iter().any(|site|site.permanent && matches!(site.value,ValueRef::Def(d) if d.module==site.module && defs.contains(&d)));
        if !has_demands {
            for d in defs {
                if !self.reads.sources.contains(&d) {
                    continue;
                }
                if let Some(scheme) = self.defs.get(&d) {
                    self.reads.completed.insert(
                        d,
                        completed(&scheme.ty, &self.s, &self.reads.fixed, scheme.level),
                    );
                }
            }
            self.complete_ready_reads();
            return;
        }
        let snapshot = self.s.clone();
        let fixed = self.reads.fixed.clone();
        let mut equations = HashMap::new();
        let mut shapes = Vec::new();
        let mut results = Vec::new();
        let mut patterns = Vec::new();
        for (i, pattern) in self.p.alias_patterns.iter().enumerate() {
            let pid = ProjectionId(i as u32);
            if self.reads.patterns.contains(&pid) {
                continue;
            }
            if let TypeKind::Of { site, .. } = pattern.source.kind
                && matches!(self.reads.sites.get(&site), Some(Read::Failed))
            {
                self.fail_pattern(pid);
                continue;
            }
            let TypeKind::Of {
                value: ValueRef::Def(d),
                site,
                ..
            } = pattern.source.kind
            else {
                continue;
            };
            if d.module != pattern.module || !defs.contains(&d) {
                continue;
            }
            self.set_module(pattern.module);
            let Some(scheme) = self.defs.get(&d) else {
                continue;
            };
            let mut closure =
                Closure::new(&snapshot, &fixed, self.reads.boundary, scheme.level, open);
            let mut aliases = HashMap::new();
            let result = if failed {
                Err(code::VALUE_TYPE_NOT_FIXED)
            } else {
                self.match_type_pattern(
                    &pattern.pattern,
                    &scheme.ty,
                    &mut closure,
                    &mut aliases,
                    &mut HashSet::new(),
                )
            };
            match result {
                Ok(()) => {
                    equations.extend(closure.equations);
                    shapes.extend(closure.shapes);
                    patterns.push((pid, aliases, site));
                }
                Err(c) => {
                    self.read_error(site, c);
                    self.fail_pattern(pid);
                }
            }
        }
        for (i, info) in self.p.of_sites.iter().enumerate() {
            let id = OfId(i as u32);
            if !info.permanent || info.projection.is_some() || self.reads.sites.contains_key(&id) {
                continue;
            }
            let ValueRef::Def(d) = info.value else {
                continue;
            };
            if d.module != info.module || !defs.contains(&d) {
                continue;
            }
            let Some(scheme) = self.defs.get(&d) else {
                continue;
            };
            let mut closure =
                Closure::new(&snapshot, &fixed, self.reads.boundary, scheme.level, open);
            let result = if failed {
                Err(code::VALUE_TYPE_NOT_FIXED)
            } else {
                closure.close(&scheme.ty)
            };
            match result {
                Ok(t) => {
                    equations.extend(closure.equations);
                    results.push((id, t));
                }
                Err(c) => self.read_error(id, c),
            }
        }
        let mut staged_fixed = fixed.clone();
        let provenance = self
            .p
            .of_sites
            .iter()
            .find(|site| {
                matches!(site.value, ValueRef::Def(d) if defs.contains(&d)) && site.permanent
            })
            .map(|site| (site.module, site.span));
        if let Some((module, span)) = provenance {
            for (v, ty) in &equations {
                staged_fixed.insert(
                    *v,
                    Fixed {
                        ty: ty.clone(),
                        module,
                        span,
                    },
                );
            }
            for (v, t) in &shapes {
                if let Ok(t) = completed(t, &snapshot, &staged_fixed, u32::MAX) {
                    equations.entry(*v).or_insert(t);
                }
            }
        }
        let trial = if self.diags[completion_start..].iter().any(|d| d.is_error()) {
            None
        } else {
            checked_batch(&snapshot, &equations, &shapes, self.reads.boundary, &fixed)
        };
        if let Some(trial) = trial {
            self.s = trial;
            for (site, t) in results {
                self.reads.sites.insert(site, Read::Ready(t));
            }
            for (pid, aliases, site) in patterns {
                for (id, t) in aliases {
                    if let TypeBody::Alias(Type {
                        kind: TypeKind::Of { site, .. },
                        ..
                    }) = self.p.ty(id).body
                    {
                        self.reads.sites.insert(site, Read::Ready(t));
                    }
                }
                self.reads.sites.insert(site, Read::Projected);
                self.reads.patterns.insert(pid);
            }
            if let Some((module, span)) = provenance {
                for (v, ty) in equations {
                    self.reads.fixed.insert(v, Fixed { ty, module, span });
                }
            }
            // A shape may become wholly closed in this same batch. Record it
            // only when every original argument is already fixed.
            for (v, t) in shapes {
                if let Ok(ty) = completed(&t, &self.s, &self.reads.fixed, u32::MAX) {
                    let (module, span) = provenance.unwrap();
                    self.reads.fixed.insert(v, Fixed { ty, module, span });
                }
            }
        } else {
            for (site, _) in results {
                self.read_error(site, code::VALUE_TYPE_NOT_FIXED);
            }
            for (pid, _, site) in patterns {
                self.read_error(site, code::VALUE_TYPE_NOT_FIXED);
                self.fail_pattern(pid);
            }
        }
        for d in defs {
            if !self.reads.sources.contains(&d) {
                continue;
            }
            if let Some(scheme) = self.defs.get(&d) {
                let t = completed(&scheme.ty, &self.s, &self.reads.fixed, scheme.level);
                self.reads.completed.insert(d, t);
            }
        }
        self.complete_ready_reads();
    }

    fn fail_pattern(&mut self, pid: ProjectionId) {
        self.reads.patterns.insert(pid);
        for id in &self.p.alias_patterns[pid.0 as usize].binders {
            if let TypeBody::Alias(Type {
                kind: TypeKind::Of { site, .. },
                ..
            }) = self.p.ty(*id).body
            {
                self.reads.sites.insert(site, Read::Failed);
            }
        }
    }

    pub(super) fn complete_ready_reads(&mut self) {
        if self.p.of_sites.is_empty() {
            return;
        }
        // Imported reads are observations of completed closed definitions.
        let active_module = self.module;
        for (i, info) in self.p.of_sites.iter().enumerate() {
            let site = OfId(i as u32);
            if info.module != active_module
                || !info.permanent
                || info.projection.is_some()
                || self.reads.sites.contains_key(&site)
            {
                continue;
            }
            let ValueRef::Def(d) = info.value else {
                continue;
            };
            let Some(result) = self.reads.completed.get(&d).cloned() else {
                continue;
            };
            match result {
                Ok(t) => {
                    self.reads.sites.insert(site, Read::Ready(t));
                }
                Err(c) => self.read_error(site, c),
            }
        }
        for (i, pattern) in self.p.alias_patterns.iter().enumerate() {
            let pid = ProjectionId(i as u32);
            if pattern.module != active_module || self.reads.patterns.contains(&pid) {
                continue;
            }
            if let TypeKind::Of { site, .. } = pattern.source.kind
                && matches!(self.reads.sites.get(&site), Some(Read::Failed))
            {
                self.fail_pattern(pid);
                continue;
            }
            let TypeKind::Of {
                value: ValueRef::Def(d),
                site,
                ..
            } = pattern.source.kind
            else {
                continue;
            };
            let Some(result) = self.reads.completed.get(&d).cloned() else {
                continue;
            };
            match result {
                Ok(t) => {
                    let mut closure = Closure::new(
                        &self.s,
                        &self.reads.fixed,
                        self.s.vars.len(),
                        u32::MAX,
                        true,
                    );
                    let mut aliases = HashMap::new();
                    match self.match_type_pattern(
                        &pattern.pattern,
                        &t,
                        &mut closure,
                        &mut aliases,
                        &mut HashSet::new(),
                    ) {
                        Ok(()) => {
                            for (id, t) in aliases {
                                if let TypeBody::Alias(Type {
                                    kind: TypeKind::Of { site, .. },
                                    ..
                                }) = self.p.ty(id).body
                                {
                                    self.reads.sites.insert(site, Read::Ready(t));
                                }
                            }
                            self.reads.patterns.insert(pid);
                        }
                        Err(c) => {
                            self.read_error(site, c);
                            self.fail_pattern(pid);
                        }
                    }
                }
                Err(c) => {
                    self.read_error(site, c);
                    self.fail_pattern(pid);
                }
            }
        }
        self.assemble_aliases();
        self.set_module(active_module);
    }

    fn match_type_pattern(
        &self,
        pattern: &TypePattern,
        source: &Ty,
        c: &mut Closure<'_>,
        aliases: &mut HashMap<TypeId, Ty>,
        seen: &mut HashSet<TypeId>,
    ) -> Result<(), &'static str> {
        match pattern {
            TypePattern::Skip => Ok(()),
            TypePattern::Bind(id) => {
                aliases.insert(*id, c.close(source)?);
                Ok(())
            }
            TypePattern::Exact(t) => {
                let expected = self.closed_written(t, &HashMap::new(), &mut HashSet::new())?;
                let source = c.close(source)?;
                if closed_equal(c.s, &expected, &source) {
                    Ok(())
                } else {
                    Err(code::TYPE_PATTERN_MISMATCH)
                }
            }
            TypePattern::Error => Err(code::TYPE_PATTERN_FORM),
            TypePattern::Named(TypeRef::Custom(id), args)
                if matches!(self.p.ty(*id).body, TypeBody::Alias(_)) =>
            {
                if !seen.insert(*id) {
                    return Err(code::ALIAS_CIRCLE);
                }
                let def = self.p.ty(*id);
                let TypeBody::Alias(t) = &def.body else {
                    unreachable!()
                };
                if value_dependent(self.p, t) {
                    return Err(code::TYPE_PATTERN_FORM);
                }
                let map = def
                    .params
                    .iter()
                    .map(|(_, v)| *v)
                    .zip(args.iter().cloned())
                    .collect();
                let mut used = HashSet::new();
                let expanded = expand_pattern(t, &map, &mut used)?;
                let r = self.match_type_pattern(&expanded, source, c, aliases, seen);
                seen.remove(id);
                r
            }
            TypePattern::Named(head, args) => {
                let t = c.expose(source)?;
                let Ty::Con(Head::Named(got), parts) = t else {
                    return Err(code::TYPE_PATTERN_MISMATCH);
                };
                if *head != got || args.len() != parts.len() {
                    return Err(code::TYPE_PATTERN_MISMATCH);
                }
                for (p, t) in args.iter().zip(parts) {
                    self.match_type_pattern(p, &t, c, aliases, seen)?;
                }
                Ok(())
            }
            TypePattern::Tuple(args) => {
                let t = c.expose(source)?;
                let Ty::Con(Head::Tuple(n), parts) = t else {
                    return Err(code::TYPE_PATTERN_MISMATCH);
                };
                if n as usize != args.len() {
                    return Err(code::TYPE_PATTERN_MISMATCH);
                }
                for (p, t) in args.iter().zip(parts) {
                    self.match_type_pattern(p, &t, c, aliases, seen)?;
                }
                Ok(())
            }
        }
    }

    fn closed_written(
        &self,
        t: &Type,
        map: &HashMap<TyVarId, Ty>,
        seen: &mut HashSet<TypeId>,
    ) -> Result<Ty, &'static str> {
        match &t.kind {
            TypeKind::Var(v) => map.get(v).cloned().ok_or(code::TYPE_PATTERN_FORM),
            TypeKind::Named { ty, args } => {
                let parts = args
                    .iter()
                    .map(|t| self.closed_written(t, map, seen))
                    .collect::<Result<Vec<_>, _>>()?;
                if let TypeRef::Custom(id) = ty
                    && let TypeBody::Alias(body) = &self.p.ty(*id).body
                {
                    if !seen.insert(*id) {
                        return Err(code::ALIAS_CIRCLE);
                    }
                    let map = self
                        .p
                        .ty(*id)
                        .params
                        .iter()
                        .map(|(_, v)| *v)
                        .zip(parts)
                        .collect();
                    let t = self.closed_written(body, &map, seen);
                    seen.remove(id);
                    return t;
                }
                Ok(if *ty == TypeRef::Builtin(BuiltinType::Never) {
                    Ty::Never
                } else {
                    Ty::named(*ty, parts)
                })
            }
            TypeKind::Tuple(ts) => Ok(Ty::tuple(
                ts.iter()
                    .map(|t| self.closed_written(t, map, seen))
                    .collect::<Result<_, _>>()?,
            )),
            TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => Ok(Ty::Fn(Box::new(FnTy {
                params: params
                    .iter()
                    .map(|p| {
                        Ok((
                            p.label.as_ref().map(|n| n.name.clone()),
                            self.closed_written(&p.ty, map, seen)?,
                        ))
                    })
                    .collect::<Result<_, &'static str>>()?,
                ret: self.closed_written(ret, map, seen)?,
                err: err
                    .as_ref()
                    .map_or(Ok(Ty::Never), |t| self.closed_written(t, map, seen))?,
                env: env
                    .as_ref()
                    .map_or(Ok(Ty::Never), |t| self.closed_written(t, map, seen))?,
            }))),
            TypeKind::Union(ts) | TypeKind::Inter(ts) => {
                let parts = ts
                    .iter()
                    .map(|t| self.closed_written(t, map, seen))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut scratch = Solver::new(self.s.variance.clone(), self.s.type_names.clone());
                let t = if matches!(t.kind, TypeKind::Union(_)) {
                    scratch.union(parts)
                } else {
                    scratch.inter(parts)
                };
                if scratch.failures.is_empty() && !contains_var(&t) {
                    Ok(t)
                } else {
                    Err(code::TYPE_PATTERN_FORM)
                }
            }
            TypeKind::Diff(ts) => {
                let mut parts = ts
                    .iter()
                    .map(|t| self.closed_written(t, map, seen))
                    .collect::<Result<Vec<_>, _>>()?;
                if parts.is_empty() {
                    return Err(code::TYPE_PATTERN_FORM);
                }
                let first = parts.remove(0);
                let mut scratch = Solver::new(self.s.variance.clone(), self.s.type_names.clone());
                let t = scratch.diff(first, parts);
                if scratch.failures.is_empty() && !contains_var(&t) {
                    Ok(t)
                } else {
                    Err(code::TYPE_PATTERN_FORM)
                }
            }
            _ => Err(code::TYPE_PATTERN_FORM),
        }
    }
}

fn expand_pattern(
    t: &Type,
    map: &HashMap<TyVarId, TypePattern>,
    used: &mut HashSet<TyVarId>,
) -> Result<TypePattern, &'static str> {
    let mut variable = false;
    let mut unavailable = false;
    walk_type(t, &mut |t| match t.kind {
        TypeKind::Var(_) => variable = true,
        TypeKind::Hole | TypeKind::Of { .. } | TypeKind::Error => unavailable = true,
        _ => {}
    });
    if unavailable {
        return Err(code::TYPE_PATTERN_FORM);
    }
    if !variable {
        return Ok(TypePattern::Exact(t.clone()));
    }
    match &t.kind {
        TypeKind::Var(v) => {
            if !used.insert(*v) {
                return Err(code::TYPE_PATTERN_FORM);
            }
            map.get(v).cloned().ok_or(code::TYPE_PATTERN_FORM)
        }
        TypeKind::Named { ty, args } => Ok(TypePattern::Named(
            *ty,
            args.iter()
                .map(|t| expand_pattern(t, map, used))
                .collect::<Result<_, _>>()?,
        )),
        TypeKind::Tuple(ts) => Ok(TypePattern::Tuple(
            ts.iter()
                .map(|t| expand_pattern(t, map, used))
                .collect::<Result<_, _>>()?,
        )),
        _ => Err(code::TYPE_PATTERN_FORM),
    }
}

fn fixed_scheme(
    t: &Ty,
    s: &Solver,
    fixed: &HashMap<V, Fixed>,
    level: u32,
) -> Result<Ty, &'static str> {
    match t {
        Ty::Var(v) if s.vars[v.0 as usize].level > level => Ok(t.clone()),
        Ty::Var(v) => fixed
            .get(v)
            .map(|f| f.ty.clone())
            .ok_or(code::VALUE_TYPE_NOT_FIXED),
        Ty::Con(h, ts) => Ok(Ty::Con(
            *h,
            ts.iter()
                .map(|t| fixed_scheme(t, s, fixed, level))
                .collect::<Result<_, _>>()?,
        )),
        Ty::Fn(f) => Ok(Ty::Fn(Box::new(FnTy {
            params: f
                .params
                .iter()
                .map(|(n, t)| Ok((n.clone(), fixed_scheme(t, s, fixed, level)?)))
                .collect::<Result<_, &'static str>>()?,
            ret: fixed_scheme(&f.ret, s, fixed, level)?,
            err: fixed_scheme(&f.err, s, fixed, level)?,
            env: fixed_scheme(&f.env, s, fixed, level)?,
        }))),
        Ty::Union(ts) => Ok(Ty::Union(
            ts.iter()
                .map(|t| fixed_scheme(t, s, fixed, level))
                .collect::<Result<_, _>>()?,
        )),
        Ty::Inter(ts) => Ok(Ty::Inter(
            ts.iter()
                .map(|t| fixed_scheme(t, s, fixed, level))
                .collect::<Result<_, _>>()?,
        )),
        Ty::Diff(t, ts) => Ok(Ty::Diff(
            Box::new(fixed_scheme(t, s, fixed, level)?),
            ts.iter()
                .map(|t| fixed_scheme(t, s, fixed, level))
                .collect::<Result<_, _>>()?,
        )),
        Ty::Both(a, b) => Ok(Ty::Both(
            Box::new(fixed_scheme(a, s, fixed, level)?),
            Box::new(fixed_scheme(b, s, fixed, level)?),
        )),
        Ty::Never | Ty::Top => Ok(t.clone()),
    }
}

impl Checker<'_> {
    fn alias_dependencies_ready(
        &self,
        t: &Type,
        owner: TypeId,
        seen: &mut HashSet<TypeId>,
    ) -> Option<bool> {
        let mut state = Some(false);
        walk_type(t, &mut |t| {
            let next = match &t.kind {
                TypeKind::Of { site, .. } => self
                    .reads
                    .sites
                    .get(site)
                    .map(|r| matches!(r, Read::Failed)),
                TypeKind::Named {
                    ty: TypeRef::Custom(id),
                    ..
                } if *id != owner && seen.insert(*id) => {
                    if let TypeBody::Alias(body) = &self.p.ty(*id).body {
                        if value_dependent(self.p, body) {
                            self.reads.aliases.get(id).map(|r| r.is_none())
                        } else {
                            self.alias_dependencies_ready(body, owner, seen)
                        }
                    } else {
                        Some(false)
                    }
                }
                _ => Some(false),
            };
            state = match (state, next) {
                (Some(a), Some(b)) => Some(a || b),
                _ => None,
            };
        });
        state
    }

    fn assemble_aliases(&mut self) {
        loop {
            let mut progress = false;
            for (i, def) in self.p.types.iter().enumerate() {
                let id = TypeId(i as u32);
                let TypeBody::Alias(body) = &def.body else {
                    continue;
                };
                if self.reads.aliases.contains_key(&id) || !value_dependent(self.p, body) {
                    continue;
                }
                let Some(failed) = self.alias_dependencies_ready(body, id, &mut HashSet::new())
                else {
                    continue;
                };
                progress = true;
                if failed {
                    self.reads.aliases.insert(id, None);
                    continue;
                }
                self.set_module(def.module);
                let map = def
                    .params
                    .iter()
                    .map(|(n, v)| (*v, ShownType::Var(n.clone())))
                    .collect();
                if let Some(shown) = self.alias_shown(body, &map, &mut HashSet::new()) {
                    if def.exported {
                        let mut private = Vec::new();
                        self.shown_hidden(&shown, def.module, &mut private);
                        for name in private {
                            self.error(
                                code::EXPORT_HIDDEN_TYPE,
                                def.span,
                                format!(
                                    "the exported alias `{}` exposes private type `{name}`",
                                    def.name
                                ),
                            );
                        }
                    }
                    self.out.alias_types.insert(id, shown);
                    self.reads.aliases.insert(id, Some(()));
                } else {
                    self.error(
                        code::VALUE_TYPE_NOT_FIXED,
                        def.span,
                        "the value-dependent alias did not complete to a closed body",
                    );
                    self.reads.aliases.insert(id, None);
                }
            }
            if !progress {
                break;
            }
        }
    }

    fn shown_hidden(&self, t: &ShownType, module: ModuleId, out: &mut Vec<String>) {
        let mut refs = Vec::new();
        t.type_refs(&mut refs);
        for ty in refs {
            if let TypeRef::Custom(id) = ty {
                let def = self.p.ty(id);
                if def.module == module && !def.exported && !out.contains(&def.name) {
                    out.push(def.name.clone());
                }
            }
        }
    }

    fn alias_shown(
        &self,
        t: &Type,
        map: &HashMap<TyVarId, ShownType>,
        seen: &mut HashSet<TypeId>,
    ) -> Option<ShownType> {
        Some(match &t.kind {
            TypeKind::Of { site, .. } => {
                let Read::Ready(t) = self.reads.sites.get(site)? else {
                    return None;
                };
                if contains_var(t) {
                    return None;
                }
                super::display::tree(t, &mut |_| unreachable!("closed read contains a variable"))
            }
            TypeKind::Var(v) => map.get(v)?.clone(),
            TypeKind::Hole => ShownType::Hole,
            TypeKind::Error => return None,
            TypeKind::Named { ty, args } => {
                let args = args
                    .iter()
                    .map(|t| self.alias_shown(t, map, seen))
                    .collect::<Option<Vec<_>>>()?;
                if let TypeRef::Custom(id) = ty
                    && let TypeBody::Alias(body) = &self.p.ty(*id).body
                {
                    if !seen.insert(*id) {
                        return None;
                    }
                    let map = self
                        .p
                        .ty(*id)
                        .params
                        .iter()
                        .map(|(_, v)| *v)
                        .zip(args)
                        .collect();
                    let shown = self.alias_shown(body, &map, seen);
                    seen.remove(id);
                    return shown;
                }
                if *ty == TypeRef::Builtin(BuiltinType::Never) {
                    ShownType::Never
                } else {
                    ShownType::Named(*ty, args)
                }
            }
            TypeKind::Tuple(ts) => ShownType::Tuple(
                ts.iter()
                    .map(|t| self.alias_shown(t, map, seen))
                    .collect::<Option<_>>()?,
            ),
            TypeKind::Union(ts) => ShownType::Union(
                ts.iter()
                    .map(|t| self.alias_shown(t, map, seen))
                    .collect::<Option<_>>()?,
            ),
            TypeKind::Inter(ts) => ShownType::Inter(
                ts.iter()
                    .map(|t| self.alias_shown(t, map, seen))
                    .collect::<Option<_>>()?,
            ),
            TypeKind::Diff(ts) => {
                let mut iter = ts.iter();
                let first = self.alias_shown(iter.next()?, map, seen)?;
                ShownType::Diff(
                    Box::new(first),
                    iter.map(|t| self.alias_shown(t, map, seen))
                        .collect::<Option<_>>()?,
                )
            }
            TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => ShownType::Fn(Box::new(ShownFn {
                params: params
                    .iter()
                    .map(|p| {
                        Some((
                            p.label.as_ref().map(|n| n.name.clone()),
                            self.alias_shown(&p.ty, map, seen)?,
                        ))
                    })
                    .collect::<Option<_>>()?,
                ret: self.alias_shown(ret, map, seen)?,
                err: match err {
                    Some(t) => Some(self.alias_shown(t, map, seen)?),
                    None => None,
                },
                env: match env {
                    Some(t) => Some(self.alias_shown(t, map, seen)?),
                    None => None,
                },
            })),
        })
    }

    pub(super) fn instantiate_alias(&mut self, id: TypeId, args: Vec<Ty>) -> Ty {
        if !matches!(self.reads.aliases.get(&id), Some(Some(()))) {
            if !self.recovery && !self.reads.aliases.contains_key(&id) {
                panic!("pending value-dependent alias {id:?}");
            }
            return Ty::Never;
        }
        let def = self.p.ty(id);
        let TypeBody::Alias(body) = &def.body else {
            unreachable!()
        };
        let map = def.params.iter().map(|(_, v)| *v).zip(args).collect();
        // Only written formals and holes are instantiated here. Every Of
        // reads its immutable completed site result, never its source again.
        self.conv(body, &map)
    }
}

pub(super) fn replace_vars(t: &Ty, map: &HashMap<V, Ty>) -> Ty {
    match t {
        Ty::Var(v) => map.get(v).cloned().unwrap_or_else(|| t.clone()),
        Ty::Con(h, ts) => Ty::Con(*h, ts.iter().map(|t| replace_vars(t, map)).collect()),
        Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
            params: f
                .params
                .iter()
                .map(|(n, t)| (n.clone(), replace_vars(t, map)))
                .collect(),
            ret: replace_vars(&f.ret, map),
            err: replace_vars(&f.err, map),
            env: replace_vars(&f.env, map),
        })),
        Ty::Union(ts) => Ty::Union(ts.iter().map(|t| replace_vars(t, map)).collect()),
        Ty::Inter(ts) => Ty::Inter(ts.iter().map(|t| replace_vars(t, map)).collect()),
        Ty::Diff(t, ts) => Ty::Diff(
            Box::new(replace_vars(t, map)),
            ts.iter().map(|t| replace_vars(t, map)).collect(),
        ),
        Ty::Both(a, b) => Ty::Both(
            Box::new(replace_vars(a, map)),
            Box::new(replace_vars(b, map)),
        ),
        Ty::Top | Ty::Never => t.clone(),
    }
}

impl Checker<'_> {
    /// Whole-annotation checking precedes instantiation and never reads a live
    /// variable bound (K2). Only the stored principal structural scheme counts.
    pub(super) fn check_value_places(&mut self, t: &Type, pol: super::solve::Pol) {
        fn written(
            c: &Checker<'_>,
            t: &Type,
            map: &HashMap<TyVarId, Ty>,
            seen: &mut HashSet<TypeId>,
        ) -> Ty {
            match &t.kind {
                TypeKind::Of { value, site, .. } => {
                    if let Some(Read::Ready(t)) = c.reads.sites.get(site) {
                        return t.clone();
                    }
                    let s = match value {
                        ValueRef::Local(v) => c.locals.get(v),
                        ValueRef::Def(d) => c.defs.get(d),
                        ValueRef::Ctor(_) => None,
                    };
                    s.map_or(Ty::Top, |scheme| {
                        let principal = if scheme.level == u32::MAX {
                            scheme.ty.clone()
                        } else {
                            super::display::simplify(&c.s, &scheme.ty, Some(scheme.level))
                                .unwrap_or_else(|()| scheme.ty.clone())
                        };
                        replace_vars(
                            &principal,
                            &c.reads
                                .fixed
                                .iter()
                                .map(|(v, f)| (*v, f.ty.clone()))
                                .collect(),
                        )
                    })
                }
                TypeKind::Var(v) => map
                    .get(v)
                    .or_else(|| c.tyvars.get(v))
                    .cloned()
                    // Pure place identities do not allocate solver variables.
                    .unwrap_or(Ty::Var(V(u32::MAX - v.0))),
                TypeKind::Named { ty, args } => {
                    let args = args
                        .iter()
                        .map(|t| written(c, t, map, seen))
                        .collect::<Vec<_>>();
                    if let TypeRef::Custom(id) = ty
                        && let TypeBody::Alias(body) = &c.p.ty(*id).body
                    {
                        if !seen.insert(*id) {
                            return Ty::Never;
                        }
                        let map =
                            c.p.ty(*id)
                                .params
                                .iter()
                                .map(|(_, v)| *v)
                                .zip(args)
                                .collect();
                        let ty = written(c, body, &map, seen);
                        seen.remove(id);
                        return ty;
                    }
                    Ty::named(*ty, args)
                }
                TypeKind::Tuple(ts) => {
                    Ty::tuple(ts.iter().map(|t| written(c, t, map, seen)).collect())
                }
                TypeKind::Union(ts) => {
                    Ty::Union(ts.iter().map(|t| written(c, t, map, seen)).collect())
                }
                TypeKind::Inter(ts) => {
                    Ty::Inter(ts.iter().map(|t| written(c, t, map, seen)).collect())
                }
                TypeKind::Diff(ts) => {
                    let mut it = ts.iter();
                    let t = it.next().map_or(Ty::Never, |t| written(c, t, map, seen));
                    Ty::Diff(Box::new(t), it.map(|t| written(c, t, map, seen)).collect())
                }
                TypeKind::Fn {
                    params,
                    ret,
                    err,
                    env,
                } => Ty::Fn(Box::new(FnTy {
                    params: params
                        .iter()
                        .map(|p| {
                            (
                                p.label.as_ref().map(|l| l.name.clone()),
                                written(c, &p.ty, map, seen),
                            )
                        })
                        .collect(),
                    ret: written(c, ret, map, seen),
                    err: err.as_ref().map_or(Ty::Never, |t| written(c, t, map, seen)),
                    env: env.as_ref().map_or(Ty::Never, |t| written(c, t, map, seen)),
                })),
                TypeKind::Hole | TypeKind::Error => Ty::Top,
            }
        }
        fn invalid(s: &Solver, t: &Ty, pol: super::solve::Pol) -> bool {
            use super::solve::Pol;
            match t {
                Ty::Union(ts) => {
                    let variables = ts
                        .iter()
                        .filter_map(|t| if let Ty::Var(v) = t { Some(v) } else { None })
                        .collect::<HashSet<_>>();
                    (variables.len() > 1 && pol != Pol::Pos)
                        || ts.iter().any(|t| invalid(s, t, pol))
                }
                Ty::Inter(ts) => {
                    (pol != Pol::Neg && ts.iter().any(|t| matches!(t, Ty::Var(_))))
                        || ts.iter().any(|t| invalid(s, t, pol))
                }
                Ty::Con(h, ts) => ts
                    .iter()
                    .zip(s.variances(*h, ts.len()))
                    .any(|(t, v)| invalid(s, t, pol.by(v))),
                Ty::Fn(f) => {
                    f.params.iter().any(|(_, t)| invalid(s, t, pol.flip()))
                        || [&f.ret, &f.err, &f.env]
                            .into_iter()
                            .any(|t| invalid(s, t, pol))
                }
                Ty::Diff(t, ts) => {
                    invalid(s, t, pol) || ts.iter().any(|t| invalid(s, t, Pol::Both))
                }
                Ty::Both(a, b) => invalid(s, a, Pol::Pos) || invalid(s, b, Pol::Neg),
                Ty::Var(_) | Ty::Top | Ty::Never => false,
            }
        }
        let expanded = written(self, t, &HashMap::new(), &mut HashSet::new());
        if invalid(&self.s, &expanded, pol) {
            self.error(code::TYPE_PLACE,t.span,"the whole expanded value-type annotation has a variable union or intersection in an illegal place (§4.4)");
        }
    }
}

/// The only mutation boundary for permanent reads. Cloning every Solver field
/// makes rollback include caches, reverse edges, levels, observations and errors.
fn checked_batch(
    snapshot: &Solver,
    equations: &HashMap<V, Ty>,
    shapes: &[(V, Ty)],
    boundary: usize,
    fixed: &HashMap<V, Fixed>,
) -> Option<Solver> {
    let mut trial = snapshot.clone();
    trial.observation = Some((
        boundary,
        fixed.keys().copied().collect(),
        Default::default(),
    ));
    for (v, t) in equations {
        trial.constrain(&Ty::Var(*v), t);
        trial.constrain(t, &Ty::Var(*v));
    }
    for (v, t) in shapes {
        trial.constrain(&Ty::Var(*v), t);
    }
    let roots = equations
        .keys()
        .chain(shapes.iter().map(|(v, _)| v))
        .copied()
        .collect();
    let occurs = trial.occurs(roots);
    let earlier = trial
        .observation
        .take()
        .is_some_and(|(_, _, seen)| !seen.borrow().is_empty());
    (trial.failures.is_empty() && !occurs && !earlier).then_some(trial)
}

impl Checker<'_> {
    pub(super) fn fixed_reached(&self, l: &Ty, r: &Ty) -> Vec<V> {
        fn collect(t: &Ty, out: &mut Vec<V>) {
            match t {
                Ty::Var(v) => out.push(*v),
                Ty::Con(_, ts) | Ty::Union(ts) | Ty::Inter(ts) => {
                    for t in ts {
                        collect(t, out);
                    }
                }
                Ty::Fn(f) => {
                    for (_, t) in &f.params {
                        collect(t, out);
                    }
                    for t in [&f.ret, &f.err, &f.env] {
                        collect(t, out);
                    }
                }
                Ty::Diff(t, ts) => {
                    collect(t, out);
                    for t in ts {
                        collect(t, out);
                    }
                }
                Ty::Both(a, b) => {
                    collect(a, out);
                    collect(b, out);
                }
                Ty::Top | Ty::Never => {}
            }
        }
        if self.reads.fixed.is_empty() {
            return Vec::new();
        }
        let mut stack = Vec::new();
        collect(l, &mut stack);
        collect(r, &mut stack);
        let mut seen = HashSet::new();
        let mut found = Vec::new();
        while let Some(v) = stack.pop() {
            if !seen.insert(v) {
                continue;
            }
            if self.reads.fixed.contains_key(&v) {
                found.push(v);
            }
            let st = &self.s.vars[v.0 as usize];
            for t in st.lower.iter().chain(&st.upper) {
                collect(t, &mut stack);
            }
            stack.extend(st.from.iter().chain(&st.into).copied());
        }
        found.sort();
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn solver() -> Solver {
        Solver::new(HashMap::new(), Vec::new())
    }
    fn int() -> Ty {
        Ty::named(TypeRef::Builtin(BuiltinType::Int), Vec::new())
    }
    fn string() -> Ty {
        Ty::named(TypeRef::Builtin(BuiltinType::String), Vec::new())
    }
    #[test]
    fn closure_follows_reverse_edges_and_collapses_pure_cycles() {
        // R4.1: the only concrete predecessor is visible through `from`.
        let mut s = solver();
        let v = s.fresh_at(0);
        let w = s.fresh_at(0);
        s.constrain(&int(), &Ty::Var(v));
        s.constrain(&Ty::Var(v), &Ty::Var(w));
        let fixed = HashMap::new();
        let mut c = Closure::new(&s, &fixed, 0, u32::MAX, false);
        assert_eq!(c.close(&Ty::Var(w)), Ok(int()));
        s.constrain(&Ty::Var(w), &Ty::Var(v));
        let mut c = Closure::new(&s, &fixed, 0, u32::MAX, false);
        assert_eq!(c.close(&Ty::Var(w)), Ok(int()));
        let mut empty = solver();
        let a = empty.fresh_at(0);
        let b = empty.fresh_at(0);
        empty.constrain(&Ty::Var(a), &Ty::Var(b));
        empty.constrain(&Ty::Var(b), &Ty::Var(a));
        let mut c = Closure::new(&empty, &fixed, 0, u32::MAX, false);
        assert_eq!(c.close(&Ty::Var(a)), Err(code::VALUE_TYPE_NOT_FIXED));
    }
    #[test]
    fn closure_does_not_equate_a_predecessor_with_its_wider_successor() {
        // R5: v <: w is retained, and never becomes w <: v.
        let mut s = solver();
        let v = s.fresh_at(0);
        let w = s.fresh_at(0);
        s.constrain(&int(), &Ty::Var(v));
        s.constrain(&Ty::Var(v), &Ty::Var(w));
        s.constrain(&string(), &Ty::Var(w));
        let fixed = HashMap::new();
        let mut c = Closure::new(&s, &fixed, 0, u32::MAX, false);
        let t = c.close(&Ty::Var(w)).unwrap();
        assert!(matches!(t, Ty::Union(_)));
        assert_eq!(c.equations[&v], int());
        let mut trial = checked_batch(&s, &HashMap::from([(v, int())]), &[], 0, &fixed).unwrap();
        trial.constrain(&string(), &Ty::Var(w));
        assert!(trial.failures.is_empty());
        trial.constrain(&string(), &Ty::Var(v));
        assert!(!trial.failures.is_empty());
    }
    #[test]
    fn failed_trial_restores_every_solver_field() {
        // R4 rollback includes reverse links/caches and trial extrusion.
        let mut s = solver();
        let v = s.fresh_at(0);
        let w = s.fresh_at(1);
        s.constrain(&Ty::Var(v), &int());
        s.constrain(&Ty::Var(w), &Ty::Var(v));
        let before = s.clone();
        let trial = checked_batch(&s, &HashMap::from([(v, string())]), &[], 0, &HashMap::new());
        assert!(trial.is_none());
        assert_eq!(s, before);
        assert!(checked_batch(&s, &HashMap::from([(v, int())]), &[], 0, &HashMap::new()).is_some());
    }
    #[test]
    fn closure_rejects_structural_cycles_and_earlier_unfixed_parts() {
        let mut s = solver();
        let v = s.fresh_at(0);
        s.constrain(
            &Ty::named(TypeRef::Builtin(BuiltinType::List), vec![Ty::Var(v)]),
            &Ty::Var(v),
        );
        let fixed = HashMap::new();
        let mut c = Closure::new(&s, &fixed, 0, u32::MAX, false);
        assert_eq!(c.close(&Ty::Var(v)), Err(code::VALUE_TYPE_NOT_FIXED));
        let mut s = solver();
        let old = s.fresh_at(0);
        s.constrain(&int(), &Ty::Var(old));
        let mut c = Closure::new(&s, &fixed, s.vars.len(), u32::MAX, false);
        assert_eq!(c.close(&Ty::Var(old)), Err(code::VALUE_TYPE_NOT_FIXED));
    }
    #[test]
    fn shape_checks_keep_skipped_identities_and_observe_earlier_links() {
        // P4: traversing a weak tuple checks its original skeleton. A skip
        // keeps the very same argument, even when it aliases a demanded part.
        let mut s = solver();
        let root = s.fresh_at(0);
        let part = s.fresh_at(0);
        let skeleton = Ty::Con(Head::Tuple(2), vec![Ty::Var(part), Ty::Var(part)]);
        s.constrain(&skeleton, &Ty::Var(root));
        s.constrain(&int(), &Ty::Var(part));
        let fixed = HashMap::new();
        let mut c = Closure::new(&s, &fixed, 0, u32::MAX, false);
        assert_eq!(c.expose(&Ty::Var(root)), Ok(skeleton.clone()));
        assert_eq!(c.close(&Ty::Var(part)), Ok(int()));
        let mut trial = checked_batch(&s, &c.equations, &c.shapes, 0, &fixed).unwrap();
        trial.constrain(&string(), &Ty::Var(part));
        assert!(!trial.failures.is_empty());
        let mut trial = checked_batch(&s, &c.equations, &c.shapes, 0, &fixed).unwrap();
        trial.constrain(&int(), &Ty::Var(root));
        assert!(!trial.failures.is_empty());

        // R3/R5: an old variable reached only through a function environment
        // is observed during trial propagation; failed trials change nothing.
        let mut s = solver();
        let old = s.fresh_at(0);
        let boundary = s.vars.len();
        let current = s.fresh_at(1);
        let fun = Ty::Fn(Box::new(FnTy {
            params: vec![],
            ret: int(),
            err: Ty::Never,
            env: Ty::Var(old),
        }));
        s.constrain(&Ty::Var(current), &fun);
        let before = s.clone();
        assert!(
            checked_batch(&s, &HashMap::from([(current, fun)]), &[], boundary, &fixed).is_none()
        );
        assert_eq!(before, s);
    }
}
