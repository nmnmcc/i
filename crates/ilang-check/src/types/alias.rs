//! Dependency-driven alias templates. Template variables are private slots,
//! never solver variables; instantiation allocates each written hole once.
use super::infer::Checker;
use super::value_types::Read;
use super::{FnTy, ShownType, Ty, V};
use crate::code;
use crate::hir::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub(super) struct Template {
    pub ty: Ty,
    pub params: usize,
    pub slots: usize,
    pub has_algebra: bool,
}

fn has_algebra(t: &Ty) -> bool {
    match t {
        Ty::Graph(_) => true,
        Ty::Union(_) | Ty::Inter(_) | Ty::Diff(_, _) => true,
        Ty::Con(_, _, ts) => ts.iter().any(has_algebra),
        Ty::Fn(f) => {
            f.params.iter().any(|(_, t)| has_algebra(t))
                || [&f.ret, &f.err, &f.env].into_iter().any(has_algebra)
        }
        Ty::Both(a, b) => has_algebra(a) || has_algebra(b),
        Ty::Var(_) | Ty::Never | Ty::Top => false,
    }
}

fn has_record(t: &Ty) -> bool {
    match t {
        Ty::Graph(g) => g.nodes().into_iter().any(|(_, node)| {
            matches!(
                node,
                super::records::Node::Rec(_) | super::records::Node::Without(..)
            )
        }),
        Ty::Con(_, _, parts) | Ty::Union(parts) | Ty::Inter(parts) => parts.iter().any(has_record),
        Ty::Fn(f) => {
            f.params.iter().any(|(_, t)| has_record(t))
                || [&f.ret, &f.err, &f.env].into_iter().any(has_record)
        }
        Ty::Diff(base, heads) => has_record(base) || heads.iter().any(has_record),
        Ty::Both(a, b) => has_record(a) || has_record(b),
        Ty::Var(_) | Ty::Never | Ty::Top => false,
    }
}

impl Template {
    pub fn apply(&self, args: Vec<Ty>, mut hole: impl FnMut() -> Ty) -> Ty {
        let mut slots = args;
        slots.extend((self.params..self.slots).map(|_| hole()));
        map_vars(&self.ty, &mut |v| slots[v.0 as usize].clone())
    }
}

pub(super) fn map_vars(t: &Ty, f: &mut impl FnMut(V) -> Ty) -> Ty {
    match t {
        Ty::Graph(g) => g.map_vars(f),
        Ty::Var(v) => f(*v),
        Ty::Con(h, values, ts) => Ty::Con(
            *h,
            values.clone(),
            ts.iter().map(|t| map_vars(t, f)).collect(),
        ),
        Ty::Fn(g) => Ty::Fn(Box::new(FnTy {
            params: g
                .params
                .iter()
                .map(|(n, t)| (n.clone(), map_vars(t, f)))
                .collect(),
            ret: map_vars(&g.ret, f),
            err: map_vars(&g.err, f),
            env: map_vars(&g.env, f),
        })),
        Ty::Union(ts) => Ty::Union(ts.iter().map(|t| map_vars(t, f)).collect()),
        Ty::Inter(ts) => Ty::Inter(ts.iter().map(|t| map_vars(t, f)).collect()),
        Ty::Diff(t, ts) => Ty::Diff(
            Box::new(map_vars(t, f)),
            ts.iter().map(|t| map_vars(t, f)).collect(),
        ),
        Ty::Both(a, b) => Ty::Both(Box::new(map_vars(a, f)), Box::new(map_vars(b, f))),
        Ty::Never | Ty::Top => t.clone(),
    }
}

impl Checker<'_> {
    pub(super) fn prepare_aliases(&mut self) {
        if self.reads.prepared {
            return;
        }
        self.reads.prepared = true;
        self.reads
            .alias_waiters
            .resize(self.p.types.len(), Vec::new());
        self.reads.remaining.resize(self.p.types.len(), 0);
        for (i, def) in self.p.types.iter().enumerate() {
            let TypeBody::Alias(body) = &def.body else {
                continue;
            };
            let id = TypeId(i as u32);
            let mut reads = HashSet::new();
            walk_type(body, &mut |t| {
                if let TypeKind::Of { site, .. } = t.kind {
                    reads.insert(site);
                }
            });
            let deps = &self.p.alias_dependencies[i];
            self.reads.remaining[i] = deps.len() + reads.len();
            for dep in deps {
                self.reads.alias_waiters[dep.0 as usize].push(id);
            }
            for site in reads {
                self.reads.read_waiters.entry(site).or_default().push(id);
            }
            if self.reads.remaining[i] == 0 {
                self.reads.queue.insert(id);
            }
        }
        self.assemble_aliases();
    }

    pub(super) fn assemble_aliases(&mut self) {
        let active = self.module;
        while let Some(id) = self.reads.queue.pop_first() {
            let def = self.p.ty(id);
            let TypeBody::Alias(body) = &def.body else {
                unreachable!()
            };
            let params = def
                .params
                .iter()
                .enumerate()
                .map(|(i, (_, v))| (*v, Ty::Var(V(i as u32))))
                .collect();
            let mut slots = def.params.len();
            let template = self.template(body, &params, &mut slots).map(|ty| Template {
                has_algebra: has_algebra(&ty),
                ty,
                params: def.params.len(),
                slots,
            });
            if let Some(template) = template {
                if def.exported && has_record(&template.ty) {
                    self.out.alias_artifacts.insert(
                        id,
                        super::scheme_wire::AliasArtifact::new(template.clone(), &self.s, id),
                    );
                }
                if self
                    .p
                    .value_aliases
                    .as_ref()
                    .is_some_and(|ids| ids.contains(&id))
                {
                    let shown = super::display::tree_with(&template.ty, &mut |v| {
                        def.params
                            .get(v.0 as usize)
                            .map_or(ShownType::Hole, |(n, _)| ShownType::Var(n.clone()))
                    });
                    self.set_module(def.module);
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
                }
                self.reads.templates.insert(id, template);
                self.reads.aliases.insert(id, Some(()));
            } else {
                self.reads.aliases.insert(id, None);
            }
            let next = std::mem::take(&mut self.reads.alias_waiters[id.0 as usize]);
            self.reads.release_aliases(next);
        }
        self.set_module(active);
    }

    fn template(&self, t: &Type, params: &HashMap<TyVarId, Ty>, slots: &mut usize) -> Option<Ty> {
        let mut go = |t: &Type| self.template(t, params, slots);
        Some(match &t.kind {
            TypeKind::Lit(v) => Ty::literal(v),
            TypeKind::Var(v) => params.get(v)?.clone(),
            TypeKind::Hole => {
                let v = V(*slots as u32);
                *slots += 1;
                Ty::Var(v)
            }
            TypeKind::Error => return None,
            TypeKind::Of { site, .. } => {
                let Read::Ready(t) = self.reads.sites.get(site)? else {
                    return None;
                };
                t.clone()
            }
            TypeKind::Named { ty, args } => {
                let args = args.iter().map(&mut go).collect::<Option<Vec<_>>>()?;
                if let TypeRef::Custom(id) = ty
                    && matches!(self.p.ty(*id).body, TypeBody::Alias(_))
                {
                    let dep = self.reads.templates.get(id)?;
                    dep.apply(args, || {
                        let v = V(*slots as u32);
                        *slots += 1;
                        Ty::Var(v)
                    })
                } else if *ty == TypeRef::Builtin(BuiltinType::Never) {
                    Ty::Never
                } else {
                    Ty::named(*ty, args)
                }
            }
            TypeKind::Tuple(ts) => Ty::tuple(ts.iter().map(&mut go).collect::<Option<_>>()?),
            TypeKind::Record {
                base,
                fields,
                removed,
            } => {
                let pool = super::records::Pool::default();
                let mut labels = Vec::new();
                let mut resolved = Vec::new();
                for (name, ty) in fields {
                    let label = super::records::written_label(name, self.s.level);
                    labels.push(label.clone());
                    resolved.push((label, go(ty)?));
                }
                labels.extend(
                    removed
                        .iter()
                        .map(|name| super::records::written_label(name, self.s.level)),
                );
                let record = pool.record(resolved);
                match base {
                    Some(base) => {
                        let base = go(base)?;
                        let rest = pool.without(&base, labels);
                        pool.inter(&[rest, record])
                    }
                    None => record,
                }
            }
            TypeKind::Union(ts) => Ty::Union(ts.iter().map(&mut go).collect::<Option<_>>()?),
            TypeKind::Inter(ts) => Ty::Inter(ts.iter().map(&mut go).collect::<Option<_>>()?),
            TypeKind::Diff(ts) => {
                let mut ts = ts.iter();
                let t = go(ts.next()?)?;
                Ty::Diff(Box::new(t), ts.map(&mut go).collect::<Option<_>>()?)
            }
            TypeKind::Fn {
                params: ps,
                ret,
                err,
                env,
            } => Ty::Fn(Box::new(FnTy {
                params: ps
                    .iter()
                    .map(|p| Some((p.label.as_ref().map(|l| l.name.clone()), go(&p.ty)?)))
                    .collect::<Option<_>>()?,
                ret: go(ret)?,
                err: err.as_ref().map_or(Some(Ty::Never), |t| go(t))?,
                env: env.as_ref().map_or(Some(Ty::Never), |t| go(t))?,
            })),
        })
    }

    pub(super) fn instantiate_alias(
        &mut self,
        id: TypeId,
        args: Vec<Ty>,
        mode: super::solve::Mode,
    ) -> Ty {
        for (arg, mode) in args.iter().zip(
            self.s
                .rows(super::Head::Named(TypeRef::Custom(id)), args.len()),
        ) {
            self.s.mark_mode(arg, mode);
        }
        let Some(template) = self.reads.templates.get(&id) else {
            if !self.recovery && !self.reads.aliases.contains_key(&id) {
                panic!("pending alias {id:?}");
            }
            return Ty::Never;
        };
        let has_algebra = template.has_algebra || args.iter().any(has_algebra);
        let ty = template.apply(args, || self.s.fresh_input());
        // With no operators in either the template or its arguments,
        // normalization would only copy constructors and variables unchanged.
        if has_algebra {
            self.s.normalize_mode(ty, mode)
        } else {
            self.s.mark_mode(&ty, mode);
            ty
        }
    }

    pub(super) fn known_alias(&self, id: TypeId, args: Vec<Ty>) -> Option<Ty> {
        let template = self.reads.templates.get(&id)?;
        let has_algebra = template.has_algebra || args.iter().any(has_algebra);
        let t = template.apply(args, || Ty::Top);
        Some(if has_algebra {
            self.normalize_known_alias(t)
        } else {
            t
        })
    }
    fn normalize_known_alias(&self, t: Ty) -> Ty {
        let go = |t| self.normalize_known_alias(t);
        match t {
            Ty::Graph(_) => t,
            Ty::Con(h, values, ts) => Ty::Con(h, values, ts.into_iter().map(go).collect()),
            Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
                params: f.params.into_iter().map(|(n, t)| (n, go(t))).collect(),
                ret: go(f.ret),
                err: go(f.err),
                env: go(f.env),
            })),
            Ty::Union(ts) => self.kunion(ts.into_iter().map(go).collect()),
            Ty::Inter(ts) => super::display::known_inter(&self.s, ts.into_iter().map(go).collect()),
            Ty::Diff(t, ts) => {
                super::display::known_diff(&self.s, go(*t), ts.into_iter().map(go).collect())
            }
            Ty::Both(a, b) => Ty::Both(Box::new(go(*a)), Box::new(go(*b))),
            Ty::Var(_) | Ty::Never | Ty::Top => t,
        }
    }

    pub(super) fn raw_alias(&self, id: TypeId, args: Vec<Ty>) -> Option<Ty> {
        self.reads
            .templates
            .get(&id)
            .map(|t| t.apply(args, || Ty::Top))
    }
}

pub(super) fn normalize(s: &mut super::solve::Solver, t: Ty) -> Ty {
    s.normalize_mode(t, super::solve::Mode::Value)
}
