//! The typing rules of §4.8 for expressions, blocks, functions, and
//! bindings, with the errors `E` and the environment `N` of each expression
//! (§9.2, §9.3).
//!
//! The `E` and `N` of the expression being typed are gathered in a stack of
//! `Effects`: a call adds the `E` and `N` of its callee, and a function or
//! the rest of a block after an environment statement starts a new entry.

use super::display;
use super::known;
use super::solve::{FailKind, Pol, Solver, builtin_variance};
use super::{FnTy, Head, Ty, Typed, Variance};
use crate::code;
use crate::hir::*;
use crate::resolve::subst;
use ilang_package::{Diagnostic, Sources};
use ilang_syntax::ast::{BinOp, Ident, UnOp};
use ilang_syntax::span::{LineIndex, Span};
use num_traits::ToPrimitive;
use std::collections::{HashMap, HashSet};

/// `∀ (variables deeper than level). ty`. A name that is not generalized
/// has the level `u32::MAX`.
#[derive(Clone, Debug)]
pub(super) struct Scheme {
    pub level: u32,
    pub ty: Ty,
}

impl Scheme {
    pub fn mono(ty: Ty) -> Scheme {
        Scheme {
            level: u32::MAX,
            ty,
        }
    }
}

#[derive(Default)]
struct Effects {
    err: Vec<Ty>,
    env: Vec<Ty>,
}

/// The custom types of §4.2 and §9.6 that the rules use.
pub(super) struct Builtins {
    pub bool_: TypeRef,
    pub nil: TypeRef,
    pub assert: TypeRef,
}

pub(crate) struct Checker<'a> {
    pub p: &'a Program,
    sources: &'a Sources,
    pub s: Solver,
    pub diags: Vec<Diagnostic>,
    /// The module whose code is being typed, for the place of problems.
    pub module: ModuleId,
    pub locals: HashMap<VarId, Scheme>,
    defs: HashMap<DefId, Scheme>,
    /// The solver variable of each type variable written in an annotation.
    pub tyvars: HashMap<TyVarId, Ty>,
    /// The type written at the top of the pattern that binds a name: its
    /// type is shown as written (§2.4).
    pub written: HashMap<VarId, Type>,
    /// The known type of each name (§4.8), where it is used now.
    pub kvars: HashMap<VarId, Ty>,
    effects: Vec<Effects>,
    pub b: Builtins,
    alias_depth: u32,
    /// §7.5: the names known to be made by a constructor, where they are
    /// used now.
    pub known: HashMap<VarId, CtorRef>,
    /// Names bound by choices that give them different constructors.
    pub known_conflict: HashSet<VarId>,
    /// What the code generator needs, gathered while typing.
    pub out: Typed,
    /// The `case` expressions of the group being typed, whose coverage is
    /// checked with the types that inference finds (§4.8 rule 11).
    pub cases: Vec<super::pattern::PendingCase>,
}

impl<'a> Checker<'a> {
    pub fn new(p: &'a Program, sources: &'a Sources) -> Checker<'a> {
        let find = |name: &str| {
            p.types
                .iter()
                .position(|t| Some(t.module) == p.prelude && t.name == name)
                .map(|i| TypeRef::Custom(TypeId(i as u32)))
                .unwrap_or(TypeRef::Builtin(BuiltinType::Never))
        };
        let b = Builtins {
            bool_: find("Bool"),
            nil: find("Nil"),
            assert: find("Assert"),
        };
        Checker {
            p,
            sources,
            s: Solver::new(variances(p), Vec::new()),
            diags: Vec::new(),
            module: ModuleId(0),
            locals: HashMap::new(),
            defs: HashMap::new(),
            tyvars: HashMap::new(),
            written: HashMap::new(),
            kvars: HashMap::new(),
            effects: Vec::new(),
            b,
            alias_depth: 0,
            known: HashMap::new(),
            known_conflict: HashSet::new(),
            out: Typed::default(),
            cases: Vec::new(),
        }
    }

    // ---- The program ----

    pub fn run(mut self) -> (Typed, Vec<Diagnostic>) {
        let p = self.p;
        // Declarations have their written types (§10.1), generalized.
        for (mi, m) in p.modules.iter().enumerate() {
            self.set_module(ModuleId(mi as u32));
            // Field types are output places (§4.5 rule 6).
            for t in &m.types {
                if let TypeBody::Custom(ctors) = &p.ty(*t).body {
                    for c in ctors {
                        for f in &c.fields {
                            self.check_places(&f.ty, Pol::Pos, 0);
                        }
                    }
                }
            }
            for (di, d) in m.defs.iter().enumerate() {
                if let DefKind::External { ty, ty_vars, .. } = &d.kind {
                    self.check_places(ty, Pol::Pos, 0);
                    self.s.level = 1;
                    self.new_tyvars(ty_vars);
                    let t = self.conv(ty, &HashMap::new());
                    self.s.level = 0;
                    let id = DefId {
                        module: ModuleId(mi as u32),
                        index: di as u32,
                    };
                    self.defs.insert(id, Scheme { level: 0, ty: t });
                }
            }
            for b in &m.bindings {
                self.note_binding(&b.binding);
            }
        }
        // Groups, in the order of their uses (§4.7).
        for group in &p.order.groups {
            self.group(group);
        }
        self.entry_points();
        // The shown type schemes (§4.7).
        let mut typed = std::mem::take(&mut self.out);
        for (mi, m) in p.modules.iter().enumerate() {
            self.set_module(ModuleId(mi as u32));
            for (di, d) in m.defs.iter().enumerate() {
                let id = DefId {
                    module: ModuleId(mi as u32),
                    index: di as u32,
                };
                let Some(scheme) = self.defs.get(&id) else {
                    continue;
                };
                let opts = display::Opts {
                    mono: true,
                    written: matches!(d.kind, DefKind::External { .. }),
                };
                // A type with itself inside it has been reported by the
                // occurs check, and is not shown.
                if let Ok(text) = display::scheme(&self.s, &scheme.ty, opts) {
                    typed.schemes.insert(id, text);
                }
                self.exported_type(d, &scheme.ty.clone(), opts);
            }
        }
        (typed, self.diags)
    }

    /// One group of top-level bindings (§4.7): typed together, then
    /// generalized together when every value is a direct value.
    fn group(&mut self, group: &[BindingRef]) {
        let p = self.p;
        let direct = group.iter().all(|b| p.binding(*b).binding.direct);
        self.s.level = u32::from(direct);
        let mut holders = Vec::new();
        for b in group {
            for d in &p.binding(*b).defs {
                let v = self.s.fresh();
                self.defs.insert(*d, Scheme::mono(v.clone()));
                holders.push((*d, v));
            }
        }
        // §4.8: inside the group, its names are known only from what is
        // written.
        for b in group {
            self.set_module(b.module);
            let binding = &p.binding(*b).binding;
            self.new_tyvars(&binding.ty_vars);
            self.own_known(binding);
        }
        let mut knowns = Vec::new();
        for b in group {
            self.set_module(b.module);
            let top = p.binding(*b);
            let binding = &top.binding;
            let mark = self.occurs_mark();
            self.check_pattern_places(&binding.pattern, Pol::Pos);
            self.effects.push(Effects::default());
            let expected = self.binding_expected(binding);
            let (a, k) = self.infer_k(&binding.value, &expected);
            knowns.push(k);
            self.fit(&binding.pattern, &a);
            if self.may_fail(&binding.pattern, &a) {
                self.add_env(Ty::named(self.b.assert, Vec::new()));
            }
            let (err, env) = self.pop_effects(binding.span);
            // §2.5: a top-level binding may not give errors or need an
            // environment.
            for (t, what) in [(err, "give errors"), (env, "need an environment")] {
                self.s.constrain(&t, &Ty::Never);
                if !self.s.failures.is_empty() {
                    let f = self.s.failures.remove(0);
                    self.s.failures.clear();
                    self.error(
                        code::TOP_LEVEL_EFFECT,
                        binding.span,
                        format!(
                            "a top-level binding may not {what} (§2.5), but this one has `{}`",
                            display::show(&self.s, &f.lhs)
                        ),
                    );
                }
            }
            self.check_occurs(mark, binding.span);
        }
        self.s.level = 0;
        // §4.7: the names of the group are linked to their values, all of
        // them, before any is generalized.
        let mark = self.occurs_mark();
        let mut types = Vec::new();
        for (d, holder) in &holders {
            let DefKind::Binding { var, binding } = p.def(*d).kind else {
                continue;
            };
            let span = p.module(d.module).bindings[binding as usize].binding.span;
            self.set_module(d.module);
            let t = self
                .locals
                .get(&var)
                .map(|s| s.ty.clone())
                .unwrap_or(Ty::Never);
            self.sub(&t, holder, span);
            types.push((*d, var, t, span));
        }
        if let Some((d, _, _, span)) = types.first() {
            self.set_module(d.module);
            self.check_occurs(mark, *span);
        }
        for (d, var, t, _) in types {
            self.set_module(d.module);
            let scheme = if direct {
                let s = Scheme {
                    level: 0,
                    ty: self.generalize(&t, 0),
                };
                // §4.8: outside its group, a generalized name has its type
                // scheme as its known type.
                let k = display::known(&self.s, &s.ty, 0);
                self.kvars.insert(var, k);
                s
            } else {
                Scheme::mono(t)
            };
            self.defs.insert(d, scheme);
        }
        for (b, k) in group.iter().zip(knowns) {
            self.out.generalized.insert(*b, direct);
            if !direct {
                self.fit_known(&p.binding(*b).binding.pattern, &k, false);
            }
        }
        // §7.4: coverage, with the types of the group.
        for c in std::mem::take(&mut self.cases) {
            self.check_case(c);
        }
    }

    /// §10.2: the entry points of the compiled package: the program entry,
    /// the tests, and the host exports.
    fn entry_points(&mut self) {
        let p = self.p;
        let Some(root) = p.root else {
            return;
        };
        let mut entries = Vec::new();
        for m in p.modules.iter().filter(|m| m.package == root) {
            for b in m.bindings.iter().filter(|b| b.main || b.test) {
                entries.extend(b.defs.first().map(|d| (*d, b.test, false)));
            }
        }
        entries.extend(p.host_exports.iter().map(|d| (*d, false, true)));
        for (d, test, export) in entries {
            self.entry_point(d, test, export);
        }
    }

    /// §10.2 and §10.3 for one entry point, with its inferred type: a test
    /// gives `Nil`; a host export takes and gives types that the host can
    /// take; and the environment has only host services and `Assert`.
    fn entry_point(&mut self, d: DefId, test: bool, export: bool) {
        self.set_module(d.module);
        let def = self.p.def(d);
        let (name, span) = (def.name.clone(), def.span);
        let Some(scheme) = self.defs.get(&d) else {
            return;
        };
        let opts = display::Opts {
            mono: true,
            written: false,
        };
        let Ok(t) = display::shown(&self.s, &scheme.ty, opts) else {
            return;
        };
        let show = |c: &Self, t: &Ty| display::show_known(&c.s, t);
        let Ty::Fn(f) = display::both(&t) else {
            if export {
                let t = show(self, &t);
                self.error(
                    code::ENTRY_TYPE,
                    span,
                    format!("`{name}` is a host export, so it is a function (§10.2), but its type is `{t}`"),
                );
            }
            return;
        };
        for m in known::members(&f.env) {
            let ok = match &m {
                Ty::Con(Head::Named(TypeRef::Custom(id)), _) => {
                    self.p.ty(*id).host.is_some() || TypeRef::Custom(*id) == self.b.assert
                }
                _ => false,
            };
            if !ok {
                let m = show(self, &m);
                self.error(
                    code::ENTRY_ENV,
                    span,
                    format!(
                        "`{name}` is an entry point, so its environment may only have host services and `Assert` (§10.2), but it needs `{m}`, which must be given before it gets to the entry point"
                    ),
                );
            }
        }
        let nil = Ty::named(self.b.nil, Vec::new());
        if test && f.ret != nil && f.ret != Ty::Never {
            let r = show(self, &f.ret);
            self.error(
                code::ENTRY_TYPE,
                span,
                format!("`{name}` is a test, so it gives `Nil` (§10.2), but it gives `{r}`"),
            );
        }
        if !export {
            return;
        }
        let parts = f
            .params
            .iter()
            .map(|p| ("a parameter", &p.1))
            .chain([("the value", &f.ret)]);
        for (what, t) in parts {
            if !self.host_type(t) {
                let t = show(self, t);
                self.error(
                    code::ENTRY_TYPE,
                    span,
                    format!(
                        "`{name}` is a host export, so {what} must be a type that the host can take: `Nil`, `Bool`, `Int`, `Float`, `String`, `BitArray`, or a custom type with no constructors (§10.3), but it is `{t}`"
                    ),
                );
            }
        }
        let errors_ok = known::members(&f.err).iter().all(|m| {
            matches!(m, Ty::Con(Head::Named(r), xs) if xs.is_empty() && (*r == self.b.nil || *r == self.b.bool_ || matches!(r, TypeRef::Builtin(b) if host_builtin(*b))))
        });
        if !errors_ok {
            let e = show(self, &f.err);
            self.error(
                code::ENTRY_TYPE,
                span,
                format!(
                    "`{name}` is a host export, so its errors must be a union of `Nil`, `Bool`, `Int`, `Float`, `String`, and `BitArray` (§10.3), but they are `{e}`"
                ),
            );
        }
    }

    /// §10.3: a type that the host can take. `Never` has no values, so it
    /// is one.
    fn host_type(&self, t: &Ty) -> bool {
        match t {
            Ty::Never => true,
            Ty::Con(Head::Named(TypeRef::Builtin(b)), _) => host_builtin(*b),
            Ty::Con(Head::Named(r @ TypeRef::Custom(id)), _) => {
                *r == self.b.nil
                    || *r == self.b.bool_
                    || matches!(&self.p.ty(*id).body, TypeBody::Custom(cs) if cs.is_empty())
            }
            _ => false,
        }
    }

    pub fn new_tyvars(&mut self, ids: &[TyVarId]) {
        for id in ids {
            let v = self.s.fresh_at(self.s.level);
            self.s.vars[v.0 as usize].input = true;
            self.s.vars[v.0 as usize].name = self.p.ty_var_names.get(id.0 as usize).cloned();
            self.tyvars.insert(*id, Ty::Var(v));
        }
    }

    // ---- Problems ----

    pub fn error(&mut self, code: &'static str, span: Span, message: impl Into<String>) {
        let source = self.p.module(self.module).source;
        let d = Diagnostic::error(code, Some(source), span, message);
        // One problem found two times at one place is reported once.
        if !self.diags.contains(&d) {
            self.diags.push(d);
        }
    }

    pub fn warning(&mut self, code: &'static str, span: Span, message: impl Into<String>) {
        let source = self.p.module(self.module).source;
        self.diags
            .push(Diagnostic::warning(code, Some(source), span, message));
    }

    /// §2.4: the inferred type of an exported binding may not use a type
    /// of its module that is not exported. A written type at the top of
    /// its pattern is checked by name resolution.
    fn exported_type(&mut self, d: &Def, t: &Ty, opts: display::Opts) {
        let DefKind::Binding { var, .. } = d.kind else {
            return;
        };
        if !d.exported || self.written.contains_key(&var) {
            return;
        }
        let Ok(shown) = display::shown(&self.s, t, opts) else {
            return;
        };
        let mut found = Vec::new();
        uses_types(&shown, &mut found);
        let mut names: Vec<String> = Vec::new();
        for id in found {
            let def = self.p.ty(id);
            if def.module == self.module && !def.exported && !names.contains(&def.name) {
                names.push(def.name.clone());
            }
        }
        for name in names {
            self.error(
                code::EXPORT_HIDDEN_TYPE,
                d.span,
                format!(
                    "`{}` is exported, but its inferred type uses the type `{name}` of this module, which is not exported (§2.4)",
                    d.name
                ),
            );
        }
    }

    /// §4.4: in a written type, a union with two or more type variables as
    /// members may only be in an output place, and an intersection with a
    /// type variable only in an input place. `pol` is the place of `t`.
    fn check_places(&mut self, t: &Type, pol: Pol, depth: u32) {
        if depth > 64 {
            return;
        }
        match &t.kind {
            TypeKind::Named { ty, args } => {
                if let TypeRef::Custom(id) = ty
                    && matches!(self.p.ty(*id).body, TypeBody::Alias(_))
                {
                    let e = self.expand(t);
                    self.check_places(&e, pol, depth + 1);
                    return;
                }
                let vs = match ty {
                    TypeRef::Custom(id) => self.s.variance.get(id).cloned().unwrap_or_default(),
                    TypeRef::Builtin(b) => builtin_variance(*b),
                };
                for (i, a) in args.iter().enumerate() {
                    let v = vs.get(i).copied().unwrap_or(Variance::Inv);
                    self.check_places(a, pol.by(v), depth);
                }
            }
            TypeKind::Tuple(ts) => {
                for a in ts {
                    self.check_places(a, pol, depth);
                }
            }
            TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => {
                for p in params {
                    self.check_places(&p.ty, pol.flip(), depth);
                }
                for x in [Some(&**ret), err.as_deref(), env.as_deref()]
                    .into_iter()
                    .flatten()
                {
                    self.check_places(x, pol, depth);
                }
            }
            TypeKind::Union(ts) => {
                let vars = ts
                    .iter()
                    .filter(|m| matches!(self.expand(m).kind, TypeKind::Var(_)))
                    .count();
                if vars >= 2 && pol != Pol::Pos {
                    self.error(
                        code::TYPE_PLACE,
                        t.span,
                        "a union with two or more type variables as members may only be in an output place (§4.4): inference could not choose the variable for a value that comes here",
                    );
                }
                for a in ts {
                    self.check_places(a, pol, depth);
                }
            }
            TypeKind::Inter(ts) => {
                if pol != Pol::Neg
                    && ts
                        .iter()
                        .any(|m| matches!(self.expand(m).kind, TypeKind::Var(_)))
                {
                    self.error(
                        code::TYPE_PLACE,
                        t.span,
                        "an intersection with a type variable may only be in an input place (§4.4): inference could not give a value that is in both",
                    );
                }
                for a in ts {
                    self.check_places(a, pol, depth);
                }
            }
            TypeKind::Diff(ts) => {
                if let Some((first, rest)) = ts.split_first() {
                    self.check_places(first, pol, depth);
                    for a in rest {
                        self.check_places(a, Pol::Both, depth);
                    }
                }
            }
            TypeKind::Var(_) | TypeKind::Hole | TypeKind::Error => {}
        }
    }

    /// `check_places` for the types written in a pattern of a binding
    /// (`Pos`) or a parameter (`Neg`).
    fn check_pattern_places(&mut self, q: &Pattern, pol: Pol) {
        match &q.kind {
            PatternKind::Typed { ty, .. } => self.check_places(ty, pol, 0),
            PatternKind::Named { pattern, .. } => self.check_pattern_places(pattern, pol),
            PatternKind::Constructor { args: ps, .. }
            | PatternKind::Tuple(ps)
            | PatternKind::Error(ps)
            | PatternKind::List { items: ps, .. } => {
                for x in ps {
                    self.check_pattern_places(x, pol);
                }
            }
            _ => {}
        }
    }

    /// Sets the module whose code is typed or shown: types of other modules
    /// are shown with the name of their module (§3).
    pub fn set_module(&mut self, m: ModuleId) {
        if self.module == m && !self.s.type_names.is_empty() {
            return;
        }
        self.module = m;
        let p = self.p;
        let here = p.module(m);
        self.s.type_names = p
            .types
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let id = TypeId(i as u32);
                if t.module == m || Some(t.module) == p.prelude || here.imported_types.contains(&id)
                {
                    t.name.clone()
                } else if let Some((alias, _)) = here.import_names.iter().find(|x| x.1 == t.module)
                {
                    format!("{alias}.{}", t.name)
                } else {
                    format!("{}.{}", p.module(t.module).path, t.name)
                }
            })
            .collect();
    }

    /// Where the occurs check of a binding starts: the number of variables
    /// and of bounds given so far.
    pub fn occurs_mark(&self) -> (usize, usize) {
        (self.s.vars.len(), self.s.touched.len())
    }

    /// §4.7: reports a type that has itself inside it, among the variables
    /// made or given bounds since `mark`.
    pub fn check_occurs(&mut self, mark: (usize, usize), span: Span) {
        let mut roots: Vec<super::V> = (mark.0..self.s.vars.len())
            .map(|i| super::V(i as u32))
            .collect();
        roots.extend(self.s.touched[mark.1..].iter().copied());
        if self.s.occurs(roots) {
            self.error(
                code::TYPE_OCCURS,
                span,
                "this type has itself inside it, through a type variable (§4.7)",
            );
        }
    }

    fn position(&self, span: Span) -> String {
        let text = &self.sources.get(self.p.module(self.module).source).text;
        let (line, col) = LineIndex::new(text).line_col(span.start);
        format!("{line}:{col}")
    }

    /// `l <: r`, with problems reported at `span`.
    pub fn sub(&mut self, l: &Ty, r: &Ty, span: Span) {
        self.s.constrain(l, r);
        self.flush(span, None);
    }

    /// `l <: r`, where `r` comes from the written type at `other`.
    pub fn sub2(&mut self, l: &Ty, r: &Ty, span: Span, other: Span) {
        self.s.constrain(l, r);
        self.flush(span, Some(other));
    }

    /// Reports the first problem the solver found since the last flush.
    pub fn flush(&mut self, span: Span, other: Option<Span>) {
        if self.s.failures.is_empty() {
            return;
        }
        let f = self.s.failures.remove(0);
        self.s.failures.clear();
        let (l, r) = (
            display::show(&self.s, &f.lhs),
            display::show(&self.s, &f.rhs),
        );
        let (code, mut message) = match f.kind {
            FailKind::Mismatch => (
                code::TYPE_MISMATCH,
                format!("`{l}` is not a subtype of `{r}`"),
            ),
            FailKind::Arity => (
                code::FN_ARITY,
                format!("the function types `{l}` and `{r}` have different numbers of parameters"),
            ),
            FailKind::Invariant => (
                code::TYPE_INVARIANT,
                format!(
                    "`{l}` and `{r}` must be equal: they are arguments of two forms of one type constructor whose parameter does not change, which are put together (§4.4)"
                ),
            ),
            FailKind::EnvForms => (
                code::ENV_FORMS,
                format!(
                    "`{l}` and `{r}` must be equal: services in environments are compared with equal arguments, and an environment may not have two forms of one type constructor with different arguments (§4.5 rule 4, §9.2)"
                ),
            ),
            FailKind::Ambiguous => (
                code::TYPE_AMBIGUOUS,
                format!("a value of `{l}` could go to more than one type variable of `{r}` (§4.4)"),
            ),
        };
        if let Some(o) = other {
            message.push_str(&format!(" (the type at {} needs it)", self.position(o)));
        }
        self.error(code, span, message);
    }

    pub fn union(&mut self, ms: Vec<Ty>, span: Span) -> Ty {
        let t = self.s.union(ms);
        self.flush(span, None);
        t
    }

    /// The union of environments (§9.2).
    fn env_union(&mut self, ms: Vec<Ty>, span: Span) -> Ty {
        let t = self.s.env_union(ms);
        self.flush(span, None);
        t
    }

    /// `N` in `M` for two environments (§4.5 rule 4), with problems reported
    /// at `span`, and `other` the place of the written type.
    fn sub_env(&mut self, l: &Ty, r: &Ty, span: Span, other: Option<Span>) {
        self.s.constrain_env(l, r);
        self.flush(span, other);
    }

    // ---- Effects ----

    fn add_err(&mut self, t: Ty) {
        if t != Ty::Never
            && let Some(e) = self.effects.last_mut()
        {
            e.err.push(t);
        }
    }

    pub fn add_env(&mut self, t: Ty) {
        if t != Ty::Never
            && let Some(e) = self.effects.last_mut()
        {
            e.env.push(t);
        }
    }

    /// The errors and the environment of an entry of the effect stack.
    fn pop_effects(&mut self, span: Span) -> (Ty, Ty) {
        let e = self.effects.pop().unwrap_or_default();
        let err = self.union(e.err, span);
        let env = self.env_union(e.env, span);
        (err, env)
    }

    // ---- Written types ----

    /// A written type in inference (§4.7): `_` is a new variable, and a
    /// type variable is the variable of its binding. `local` gives the type
    /// parameters of a definition.
    pub fn conv(&mut self, t: &Type, local: &HashMap<TyVarId, Ty>) -> Ty {
        match &t.kind {
            TypeKind::Named { ty, args } => {
                if let TypeRef::Custom(id) = ty
                    && let TypeBody::Alias(body) = &self.p.ty(*id).body
                {
                    if self.alias_depth > 64 {
                        return self.s.fresh();
                    }
                    let params = self.p.ty(*id).params.clone();
                    let mut map = HashMap::new();
                    for ((_, tv), a) in params.iter().zip(args) {
                        let a = self.conv(a, local);
                        map.insert(*tv, a);
                    }
                    self.alias_depth += 1;
                    let r = self.conv(body, &map);
                    self.alias_depth -= 1;
                    return r;
                }
                if *ty == TypeRef::Builtin(BuiltinType::Never) {
                    return Ty::Never;
                }
                let args = args.iter().map(|a| self.conv(a, local)).collect();
                Ty::named(*ty, args)
            }
            TypeKind::Var(tv) => {
                if let Some(t) = local.get(tv).or_else(|| self.tyvars.get(tv)) {
                    return t.clone();
                }
                let v = self.s.fresh_input();
                if let Ty::Var(x) = v {
                    self.s.vars[x.0 as usize].name =
                        self.p.ty_var_names.get(tv.0 as usize).cloned();
                }
                self.tyvars.insert(*tv, v.clone());
                v
            }
            TypeKind::Hole | TypeKind::Error => self.s.fresh_input(),
            TypeKind::Tuple(ts) => Ty::tuple(ts.iter().map(|a| self.conv(a, local)).collect()),
            TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => {
                let params = params
                    .iter()
                    .map(|p| {
                        (
                            p.label.as_ref().map(|l| l.name.clone()),
                            self.conv(&p.ty, local),
                        )
                    })
                    .collect();
                let ret = self.conv(ret, local);
                let err = err.as_ref().map_or(Ty::Never, |e| self.conv(e, local));
                let env = env.as_ref().map_or(Ty::Never, |e| self.conv(e, local));
                Ty::Fn(Box::new(FnTy {
                    params,
                    ret,
                    err,
                    env,
                }))
            }
            TypeKind::Union(ts) => {
                let ms = ts.iter().map(|a| self.conv(a, local)).collect();
                self.union(ms, t.span)
            }
            TypeKind::Inter(ts) => {
                let ms = ts.iter().map(|a| self.conv(a, local)).collect();
                let r = self.s.inter(ms);
                self.flush(t.span, None);
                r
            }
            TypeKind::Diff(ts) => {
                let mut a = self.conv(&ts[0], local);
                for part in &ts[1..] {
                    let h = self.conv(part, local);
                    let mut members = Vec::new();
                    super::solve::flatten_union(std::slice::from_ref(&h), &mut members);
                    if members
                        .iter()
                        .any(|m| m.head().is_none() && *m != Ty::Never)
                    {
                        self.error(
                            code::TYPE_DIFF_VAR,
                            part.span,
                            "the right side of `-` may only have members with a type constructor (§4.4)",
                        );
                        continue;
                    }
                    a = self.s.diff(a, members);
                    self.flush(t.span, None);
                }
                a
            }
        }
    }

    /// `t` with the aliases at its top expanded (§4.3).
    pub fn expand(&self, t: &Type) -> Type {
        let mut cur = t.clone();
        for _ in 0..64 {
            let TypeKind::Named {
                ty: TypeRef::Custom(id),
                args,
            } = &cur.kind
            else {
                break;
            };
            let def = self.p.ty(*id);
            let TypeBody::Alias(body) = &def.body else {
                break;
            };
            let map: HashMap<TyVarId, &Type> = def.params.iter().map(|p| p.1).zip(args).collect();
            cur = subst(body, &map);
        }
        cur
    }

    /// Notes the type written at the top of the pattern of a binding.
    fn note_binding(&mut self, b: &Binding) {
        let mut p = &b.pattern;
        let mut names = Vec::new();
        loop {
            match &p.kind {
                PatternKind::Named { binder, pattern } => {
                    names.push(binder.var);
                    p = pattern;
                }
                PatternKind::Typed {
                    binder: Some(binder),
                    ty,
                } => {
                    names.push(binder.var);
                    for v in names {
                        self.written.insert(v, ty.clone());
                    }
                    break;
                }
                _ => break,
            }
        }
    }

    // ---- Names and constructors ----

    fn instantiate(&mut self, s: &Scheme) -> Ty {
        if s.level == u32::MAX {
            return s.ty.clone();
        }
        self.s.freshen(&s.ty, s.level, &mut HashMap::new())
    }

    /// The field types of a constructor, with `targs` for the type
    /// parameters of its type.
    pub fn ctor_fields(&mut self, c: CtorRef, targs: &[Ty]) -> Vec<(Option<String>, Ty)> {
        let def = self.p.ty(c.ty);
        let map: HashMap<TyVarId, Ty> = def
            .params
            .iter()
            .map(|p| p.1)
            .zip(targs.iter().cloned())
            .collect();
        let TypeBody::Custom(ctors) = &def.body else {
            return Vec::new();
        };
        let Some(ctor) = ctors.get(c.index as usize) else {
            return Vec::new();
        };
        ctor.fields
            .iter()
            .map(|f| {
                (
                    f.label.as_ref().map(|l| l.name.clone()),
                    self.conv(&f.ty, &map),
                )
            })
            .collect()
    }

    pub fn fresh_args(&mut self, t: TypeId) -> Vec<Ty> {
        (0..self.p.ty(t).params.len())
            .map(|_| self.s.fresh())
            .collect()
    }

    /// Rule 7: a constructor is a function from its fields to its type, with
    /// the field labels; with no fields it is a value.
    fn ctor_value(&mut self, c: CtorRef) -> Ty {
        let targs = self.fresh_args(c.ty);
        let fields = self.ctor_fields(c, &targs);
        let t = Ty::named(TypeRef::Custom(c.ty), targs);
        if fields.is_empty() {
            return t;
        }
        Ty::Fn(Box::new(FnTy {
            params: fields,
            ret: t,
            err: Ty::Never,
            env: Ty::Never,
        }))
    }

    fn prim(&self, b: BuiltinType) -> Ty {
        Ty::named(TypeRef::Builtin(b), Vec::new())
    }

    pub fn bool_ty(&self) -> Ty {
        Ty::named(self.b.bool_, Vec::new())
    }

    // ---- Expressions ----

    pub fn infer(&mut self, e: &Expr) -> Ty {
        self.infer_k(e, &Ty::Top).0
    }

    /// The type of `e`, and its known type (§4.8), `Top` when it has none.
    /// `expected` is the expected type of its place, `Top` for none.
    pub fn infer_k(&mut self, e: &Expr, expected: &Ty) -> (Ty, Ty) {
        let same = |t: Ty| (t.clone(), t);
        match &e.kind {
            ExprKind::Int(_) => same(self.prim(BuiltinType::Int)),
            ExprKind::Float(_) => same(self.prim(BuiltinType::Float)),
            ExprKind::String(_) => same(self.prim(BuiltinType::String)),
            ExprKind::Name(ValueRef::Local(v)) => {
                self.note_use(*v, e.span);
                let t = match self.locals.get(v).cloned() {
                    Some(s) => self.instantiate(&s),
                    None => self.s.fresh_input(),
                };
                (t, self.kvars.get(v).cloned().unwrap_or(Ty::Top))
            }
            ExprKind::Name(ValueRef::Def(d)) => {
                let k = match self.p.def(*d).kind {
                    DefKind::Binding { var, .. } => {
                        self.note_use(var, e.span);
                        self.kvars.get(&var).cloned().unwrap_or(Ty::Top)
                    }
                    DefKind::External { .. } => self
                        .defs
                        .get(d)
                        .map_or(Ty::Top, |s| display::known(&self.s, &s.ty, s.level)),
                };
                let t = match self.defs.get(d).cloned() {
                    Some(s) => self.instantiate(&s),
                    None => self.s.fresh_input(),
                };
                (t, k)
            }
            ExprKind::Name(ValueRef::Ctor(c)) => {
                let t = self.ctor_value(*c);
                (t, self.ctor_known(*c))
            }
            ExprKind::Tuple(items) => {
                // A part has the matching part of the expected type (§4.8).
                let parts = match known::members(expected).as_slice() {
                    [Ty::Con(Head::Tuple(n), xs)] if *n as usize == items.len() => xs.clone(),
                    _ => vec![Ty::Top; items.len()],
                };
                let (ts, ks) = items
                    .iter()
                    .zip(&parts)
                    .map(|(x, p)| self.infer_k(x, p))
                    .unzip();
                (Ty::tuple(ts), Ty::tuple(ks))
            }
            ExprKind::List { items, rest } => {
                let head = Some(Head::Named(TypeRef::Builtin(BuiltinType::List)));
                let (item_exp, list_exp) = match known::members(expected).as_slice() {
                    [m @ Ty::Con(_, xs)] if m.head() == head && xs.len() == 1 => {
                        (xs[0].clone(), m.clone())
                    }
                    _ => (Ty::Top, Ty::Top),
                };
                let t = self.s.fresh();
                let mut ks = Vec::new();
                for x in items {
                    let (xt, xk) = self.infer_k(x, &item_exp);
                    self.sub(&xt, &t, x.span);
                    ks.push(xk);
                }
                let list = Ty::named(TypeRef::Builtin(BuiltinType::List), vec![t]);
                if let Some(r) = rest {
                    let (rt, rk) = self.infer_k(r, &list_exp);
                    self.sub(&rt, &list, r.span);
                    ks.push(match known::member(&rk, head) {
                        Some(Ty::Con(_, xs)) if xs.len() == 1 => xs[0].clone(),
                        _ => Ty::Top,
                    });
                }
                let k = Ty::named(TypeRef::Builtin(BuiltinType::List), vec![self.kunion(ks)]);
                (list, k)
            }
            ExprKind::BitArray(segs) => {
                for s in segs {
                    let want = self.format(s.format.as_ref(), s.span);
                    let t = self.infer(&s.value);
                    self.sub(&t, &want, s.value.span);
                }
                same(self.prim(BuiltinType::BitArray))
            }
            ExprKind::Block(b) => self.block(&b.stmts, b.span, expected),
            ExprKind::Fn(f) => self.function(f, expected),
            ExprKind::Case { subject, clauses } => self.case(subject, clauses, e.span, expected),
            ExprKind::Call { callee, args } => self.call(callee, None, args, e.span),
            ExprKind::Field { target, name } => self.field(target, name, e.span),
            ExprKind::TupleIndex {
                target,
                index,
                index_span,
            } => self.tuple_index(
                target,
                index.to_u32().unwrap_or(u32::MAX),
                *index_span,
                e.span,
            ),
            ExprKind::RecordUpdate {
                ctor,
                ctor_span,
                base,
                fields,
            } => self.record_update(*ctor, *ctor_span, base, fields),
            ExprKind::Unary { op, expr } => {
                let want = self.prim(match op {
                    UnOp::Neg => BuiltinType::Int,
                    UnOp::NegFloat => BuiltinType::Float,
                });
                let t = self.infer(expr);
                self.sub(&t, &want, expr.span);
                same(want)
            }
            ExprKind::Binary { op, lhs, rhs } => self.binary(*op, lhs, rhs, e.span),
            ExprKind::Error => (self.s.fresh_input(), Ty::Top),
        }
    }

    /// Rule 8 and §6.9.
    fn binary(&mut self, op: BinOp, lhs: &Expr, rhs: &Expr, span: Span) -> (Ty, Ty) {
        use BinOp::*;
        if op == Pipe {
            // `a |> f(b)` is `f(a, b)`, and `a |> f` is `f(a)`.
            let (a, k) = self.infer_k(lhs, &Ty::Top);
            let first = Some((a, k, lhs.span));
            return match &rhs.kind {
                ExprKind::Call { callee, args } => self.call(callee, first, args, span),
                _ => self.call(rhs, first, &[], span),
            };
        }
        let (operand, result) = match op {
            Or | And => (Some(self.bool_ty()), self.bool_ty()),
            Eq | NotEq => (None, self.bool_ty()),
            Lt | LtEq | Gt | GtEq => (Some(self.prim(BuiltinType::Int)), self.bool_ty()),
            LtFloat | LtEqFloat | GtFloat | GtEqFloat => {
                (Some(self.prim(BuiltinType::Float)), self.bool_ty())
            }
            Concat => (
                Some(self.prim(BuiltinType::String)),
                self.prim(BuiltinType::String),
            ),
            Add | Sub | Mul | Div | Rem => (
                Some(self.prim(BuiltinType::Int)),
                self.prim(BuiltinType::Int),
            ),
            AddFloat | SubFloat | MulFloat | DivFloat => (
                Some(self.prim(BuiltinType::Float)),
                self.prim(BuiltinType::Float),
            ),
            Pipe => unreachable!(),
        };
        let (lt, lk) = self.infer_k(lhs, &Ty::Top);
        let (rt, rk) = self.infer_k(rhs, &Ty::Top);
        // §6.9: `==` on values of disjoint known types would always give the
        // same result.
        if matches!(op, Eq | NotEq) && lk != Ty::Top && rk != Ty::Top && known::disjoint(&lk, &rk) {
            let (l, r) = (
                display::show_known(&self.s, &lk),
                display::show_known(&self.s, &rk),
            );
            self.error(
                code::EQ_DISJOINT,
                span,
                format!(
                    "the two sides have the known types `{l}` and `{r}`, which are disjoint, so the result would always be the same (§6.9)"
                ),
            );
        }
        if let Some(want) = operand {
            self.sub(&lt, &want, lhs.span);
            self.sub(&rt, &want, rhs.span);
        }
        (result.clone(), result)
    }

    /// Rule 5: `f(a1, …)`. `first` is the left side of `|>`, with its known
    /// type.
    fn call(
        &mut self,
        callee: &Expr,
        first: Option<(Ty, Ty, Span)>,
        args: &[Arg],
        span: Span,
    ) -> (Ty, Ty) {
        let (f, kf) = self.infer_k(callee, &Ty::Top);
        let kfn = known::one_fn(&kf);
        let skip = usize::from(first.is_some());
        let n = skip + args.len();
        // §6.6: the parameter that each argument goes to.
        let to: Option<Vec<usize>> = if args.iter().any(|a| a.label.is_some()) {
            match &kfn {
                None => {
                    self.error(
                        code::NEEDS_KNOWN_TYPE,
                        callee.span,
                        "a call with an argument by label needs a known function type for the function (§6.6, §4.8); write the type of the function, or give the arguments by position",
                    );
                    None
                }
                Some(k) => {
                    let labels: Vec<Option<String>> =
                        k.params.iter().map(|p| p.0.clone()).collect();
                    let given: Vec<(Option<&Ident>, Span)> = first
                        .iter()
                        .map(|x| (None, x.2))
                        .chain(args.iter().map(|a| (a.label.as_ref(), a.value.span)))
                        .collect();
                    self.place(&labels, &given, span)
                }
            }
        } else {
            Some((0..n).collect())
        };
        // §4.8: an argument has the expected type of its parameter, with the
        // type variables that matching the other arguments fixes. Function
        // expressions with a parameter with no written type are typed after
        // the others, and are left out of that matching.
        let params: Option<Vec<Ty>> = match (&kfn, &to) {
            (Some(k), Some(to)) => Some(
                to.iter()
                    .map(|i| k.params.get(*i).map_or(Ty::Top, |p| p.1.clone()))
                    .collect(),
            ),
            _ => None,
        };
        let mut typed: Vec<Option<(Ty, Ty)>> = vec![None; n];
        let mut sigma = known::Sigma::new();
        if let Some((t, k, _)) = &first {
            if let Some(ps) = &params {
                self.match_k(&ps[0], k, &mut sigma, 0);
            }
            typed[0] = Some((t.clone(), k.clone()));
        }
        for later in [false, true] {
            for (j, a) in args.iter().enumerate() {
                let i = skip + j;
                if (params.is_some() && known::open_fn(&a.value)) != later {
                    continue;
                }
                let expected = match &params {
                    Some(ps) => self.ksubst(&ps[i], &sigma, true).unwrap_or(Ty::Top),
                    None => Ty::Top,
                };
                let (t, k) = self.infer_k(&a.value, &expected);
                if let (Some(ps), false) = (&params, later) {
                    self.match_k(&ps[i], &k, &mut sigma, 0);
                }
                typed[i] = Some((t, k));
            }
        }
        let typed: Vec<(Ty, Ty)> = typed.into_iter().flatten().collect();
        let spans: Vec<Span> = first
            .iter()
            .map(|x| x.2)
            .chain(args.iter().map(|a| a.value.span))
            .collect();
        let Some(to) = to else {
            return (self.s.fresh(), Ty::Top);
        };
        // `order[p]` is the argument that goes to the parameter `p`.
        let mut order = vec![0u32; n];
        for (g, p) in to.iter().enumerate() {
            order[*p] = g as u32;
        }
        self.out.args.insert((self.module, span), order.clone());
        let placed: Vec<(Ty, Span)> = order
            .iter()
            .map(|g| (typed[*g as usize].0.clone(), spans[*g as usize]))
            .collect();
        let known = match &kfn {
            Some(k) if k.params.len() == n => {
                let ks: Vec<Ty> = order.iter().map(|g| typed[*g as usize].1.clone()).collect();
                self.call_known(k, &ks)
            }
            _ => Ty::Top,
        };
        let t = match &f {
            Ty::Fn(ft) if ft.params.len() == placed.len() => {
                for ((t, s), (_, p)) in placed.iter().zip(&ft.params) {
                    self.sub2(t, p, *s, callee.span);
                }
                self.add_err(ft.err.clone());
                self.add_env(ft.env.clone());
                ft.ret.clone()
            }
            Ty::Fn(ft) => {
                let n = ft.params.len();
                self.error(
                    code::FN_ARITY,
                    span,
                    format!(
                        "this call gives {} argument{}, but the function has {n} parameter{} (§6.6)",
                        placed.len(),
                        if placed.len() == 1 { "" } else { "s" },
                        if n == 1 { "" } else { "s" },
                    ),
                );
                self.s.fresh()
            }
            _ => {
                // The function may be a parameter: what it gives back may
                // come from outside.
                let (r, e, n) = (
                    self.s.fresh_input(),
                    self.s.fresh_input(),
                    self.s.fresh_input(),
                );
                let want = Ty::Fn(Box::new(FnTy {
                    params: placed.into_iter().map(|(t, _)| (None, t)).collect(),
                    ret: r.clone(),
                    err: e.clone(),
                    env: n.clone(),
                }));
                self.sub(&f, &want, span);
                self.add_err(e);
                self.add_env(n);
                r
            }
        };
        (t, known)
    }

    /// §6.6: arguments by position go to the first parameters, and
    /// arguments by label to the parameter with that label. Gives the
    /// parameter of each argument.
    fn place(
        &mut self,
        labels: &[Option<String>],
        given: &[(Option<&Ident>, Span)],
        span: Span,
    ) -> Option<Vec<usize>> {
        let mut slots: Vec<bool> = vec![false; labels.len()];
        let mut to = Vec::new();
        let mut pos = 0;
        let mut ok = true;
        for (label, s) in given {
            let i = match label {
                None => {
                    pos += 1;
                    (pos <= labels.len()).then_some(pos - 1)
                }
                Some(l) => labels
                    .iter()
                    .position(|x| x.as_deref() == Some(l.name.as_str())),
            };
            match (i, label) {
                (Some(i), _) if !slots[i] => {
                    slots[i] = true;
                    to.push(i);
                }
                (Some(_), Some(l)) => {
                    ok = false;
                    self.error(
                        code::CALL_LABELS,
                        l.span,
                        format!("the parameter `{}` gets more than one argument", l.name),
                    );
                }
                (None, Some(l)) => {
                    ok = false;
                    self.error(
                        code::CALL_LABELS,
                        l.span,
                        format!("the function has no parameter with the label `{}`", l.name),
                    );
                }
                (_, None) => {
                    ok = false;
                    self.error(
                        code::FN_ARITY,
                        *s,
                        format!("the function has only {} parameters", labels.len()),
                    );
                }
            }
        }
        if ok && slots.iter().any(|s| !s) {
            let missing: Vec<String> = labels
                .iter()
                .zip(&slots)
                .enumerate()
                .filter(|(_, (_, s))| !**s)
                .map(|(i, (l, _))| match l {
                    Some(l) => format!("`{l}`"),
                    None => format!("{}", i + 1),
                })
                .collect();
            self.error(
                code::CALL_LABELS,
                span,
                format!(
                    "every parameter must get one argument (§6.6), but {} get{} none",
                    missing.join(", "),
                    if missing.len() == 1 { "s" } else { "" }
                ),
            );
            return None;
        }
        ok.then_some(to)
    }

    /// §7.5: records a use of a name that is known to be made by a
    /// constructor.
    fn note_use(&mut self, v: VarId, span: Span) {
        if let Some(c) = self.known.get(&v) {
            self.out.known_ctors.insert((self.module, span), *c);
        }
    }

    /// The constructor that the name `e` is known to be made by (§7.5).
    fn known_ctor(&self, e: &Expr) -> Option<CtorRef> {
        let var = match &e.kind {
            ExprKind::Name(ValueRef::Local(v)) => *v,
            ExprKind::Name(ValueRef::Def(d)) => match self.p.def(*d).kind {
                DefKind::Binding { var, .. } => var,
                DefKind::External { .. } => return None,
            },
            _ => return None,
        };
        self.known.get(&var).copied()
    }

    fn ctor_name(&self, c: CtorRef) -> String {
        match &self.p.ty(c.ty).body {
            TypeBody::Custom(cs) => cs[c.index as usize].name.clone(),
            TypeBody::Alias(_) => String::new(),
        }
    }

    /// Rule 13 and §6.7 for `v.f`: every constructor that may have made `v`
    /// has a field with the label `f`, and `v.f` has the union of their
    /// types.
    fn field(&mut self, target: &Expr, name: &Ident, span: Span) -> (Ty, Ty) {
        let (t, kv) = self.infer_k(target, &Ty::Top);
        // The constructors that may have made `v`, each with the arguments
        // of its type in the known type of `v`, when they are known.
        let mut ctors: Vec<(CtorRef, Option<Vec<Ty>>)> = Vec::new();
        let known_ctor = self.known_ctor(target);
        if let Some(c) = known_ctor {
            let args = match known::member(&kv, Some(Head::Named(TypeRef::Custom(c.ty)))) {
                Some(Ty::Con(_, xs)) => Some(xs),
                _ => None,
            };
            ctors.push((c, args));
        } else {
            if kv == Ty::Top {
                self.error(
                    code::NEEDS_KNOWN_TYPE,
                    name.span,
                    format!(
                        "`.{}` needs a known type for its value (§6.7, §4.8); write its type, such as `x: T`",
                        name.name
                    ),
                );
                return (self.s.fresh(), Ty::Top);
            }
            for m in known::members(&kv) {
                let custom = match &m {
                    Ty::Con(Head::Named(TypeRef::Custom(id)), xs) => match &self.p.ty(*id).body {
                        TypeBody::Custom(cs) => Some((*id, xs.clone(), cs.len())),
                        TypeBody::Alias(_) => None,
                    },
                    _ => None,
                };
                let Some((id, xs, count)) = custom else {
                    let (k, m) = (
                        display::show_known(&self.s, &kv),
                        display::show_known(&self.s, &m),
                    );
                    self.error(
                        code::NO_FIELD,
                        name.span,
                        format!(
                            "`.{}` needs every member of the known type `{k}` of its value to be a custom type (§6.7), but `{m}` is not one",
                            name.name
                        ),
                    );
                    return (self.s.fresh(), Ty::Top);
                };
                if count == 0 {
                    let tname = &self.p.ty(id).name;
                    self.error(
                        code::NO_FIELD,
                        name.span,
                        format!("`{tname}` has no constructors, so its values have no fields"),
                    );
                    return (self.s.fresh(), Ty::Top);
                }
                for i in 0..count {
                    let c = CtorRef {
                        ty: id,
                        index: i as u32,
                    };
                    ctors.push((c, Some(xs.clone())));
                }
            }
        }
        let mut found: Vec<(CtorRef, u32)> = Vec::new();
        for (c, _) in &ctors {
            let TypeBody::Custom(cs) = &self.p.ty(c.ty).body else {
                continue;
            };
            let at = cs[c.index as usize]
                .fields
                .iter()
                .position(|f| f.label.as_ref().is_some_and(|l| l.name == name.name));
            match at {
                Some(i) => found.push((*c, i as u32)),
                None => {
                    let message = if known_ctor.is_some() {
                        format!(
                            "the constructor `{}`, which made this value (§7.5), has no field with the label `{}` (§4.8 rule 13)",
                            self.ctor_name(*c),
                            name.name
                        )
                    } else {
                        format!(
                            "not every constructor of `{}` has a field with the label `{}` (§4.8 rule 13)",
                            self.p.ty(c.ty).name,
                            name.name
                        )
                    };
                    self.error(code::NO_FIELD, name.span, message);
                    return (self.s.fresh(), Ty::Top);
                }
            }
        }
        // The type of `v` is a subtype of its known type (rule 13), and of
        // the type of the constructor that made it. The field types are
        // those of the forms of these types in that bound.
        let mut forms: Vec<Ty> = match kv {
            Ty::Top => Vec::new(),
            _ => known::members(&kv)
                .iter()
                .map(|m| self.known_bound(m))
                .collect(),
        };
        let mut parts = Vec::new();
        for (c, i) in &found {
            let h = Some(Head::Named(TypeRef::Custom(c.ty)));
            let targs = match forms.iter().find(|f| f.head() == h) {
                Some(Ty::Con(_, xs)) => xs.clone(),
                _ => {
                    let a = self.fresh_args(c.ty);
                    forms.push(Ty::named(TypeRef::Custom(c.ty), a.clone()));
                    a
                }
            };
            parts.push(self.ctor_fields(*c, &targs)[*i as usize].1.clone());
        }
        let bound = self.union(forms, target.span);
        self.sub(&t, &bound, target.span);
        let ks = ctors
            .iter()
            .zip(&found)
            .map(|((c, args), (_, i))| self.field_known(*c, *i as usize, args.as_deref()))
            .collect();
        let k = self.kunion(ks);
        self.out
            .access
            .insert((self.module, span), super::Access::Field(found));
        (self.union(parts, name.span), k)
    }

    /// Rule 13 for `v.k`: every member of the known type of `v` is a tuple
    /// with more than k parts.
    fn tuple_index(&mut self, target: &Expr, k: u32, index_span: Span, span: Span) -> (Ty, Ty) {
        let (t, kv) = self.infer_k(target, &Ty::Top);
        if kv == Ty::Top {
            self.error(
                code::NEEDS_KNOWN_TYPE,
                index_span,
                "`.k` needs a known type for its value (§4.8 rule 13); write its type, such as `x: (A, B)`",
            );
            return (self.s.fresh(), Ty::Top);
        }
        let mut forms = Vec::new();
        let mut parts = Vec::new();
        let mut kparts = Vec::new();
        for m in known::members(&kv) {
            match &m {
                Ty::Con(Head::Tuple(n), xs) if k < *n => {
                    let form = self.known_bound(&m);
                    if let Ty::Con(_, ys) = &form {
                        parts.push(ys[k as usize].clone());
                    }
                    forms.push(form);
                    kparts.push(xs[k as usize].clone());
                }
                Ty::Con(Head::Tuple(n), _) => {
                    self.error(
                        code::NO_FIELD,
                        index_span,
                        format!(
                            "the tuple has {n} part{}, so it has no part {k} (§6.4)",
                            if *n == 1 { "" } else { "s" }
                        ),
                    );
                    return (self.s.fresh(), Ty::Top);
                }
                _ => {
                    let (whole, m) = (
                        display::show_known(&self.s, &kv),
                        display::show_known(&self.s, &m),
                    );
                    self.error(
                        code::NO_FIELD,
                        index_span,
                        format!(
                            "`.{k}` needs every member of the known type `{whole}` of its value to be a tuple (§4.8 rule 13), but `{m}` is not one"
                        ),
                    );
                    return (self.s.fresh(), Ty::Top);
                }
            }
        }
        let bound = self.union(forms, target.span);
        self.sub(&t, &bound, target.span);
        self.out
            .access
            .insert((self.module, span), super::Access::Part(k));
        (self.union(parts, index_span), self.kunion(kparts))
    }

    /// §6.7: `C(..base, f: e)`.
    fn record_update(
        &mut self,
        ctor: CtorRef,
        ctor_span: Span,
        base: &Expr,
        fields: &[(Ident, Expr)],
    ) -> (Ty, Ty) {
        let (bt, bk) = self.infer_k(base, &Ty::Top);
        let TypeBody::Custom(ctors) = &self.p.ty(ctor.ty).body else {
            return (self.s.fresh(), Ty::Top);
        };
        if ctors.len() != 1 && self.known_ctor(base) != Some(ctor) {
            let cname = ctors[ctor.index as usize].name.clone();
            self.error(
                code::NOT_KNOWN_CTOR,
                ctor_span,
                format!(
                    "`{cname}(..v, …)` needs `v` to be known to be made by `{cname}` (§6.7): its type has other constructors, so `v` must be a name matched with `{cname}(…)` (§7.5)"
                ),
            );
        }
        let old = self.fresh_args(ctor.ty);
        let new = self.fresh_args(ctor.ty);
        self.sub(
            &bt,
            &Ty::named(TypeRef::Custom(ctor.ty), old.clone()),
            base.span,
        );
        let old_fields = self.ctor_fields(ctor, &old);
        let new_fields = self.ctor_fields(ctor, &new);
        let mut given: Vec<Option<Ty>> = vec![None; new_fields.len()];
        for (label, e) in fields {
            let (t, k) = self.infer_k(e, &Ty::Top);
            match new_fields
                .iter()
                .position(|f| f.0.as_deref() == Some(&label.name))
            {
                Some(i) if given[i].is_none() => {
                    given[i] = Some(k);
                    self.sub(&t, &new_fields[i].1, e.span);
                }
                Some(_) => self.error(
                    code::CALL_LABELS,
                    label.span,
                    format!("the field `{}` is given more than once", label.name),
                ),
                None => self.error(
                    code::NO_FIELD,
                    label.span,
                    format!(
                        "the constructor has no field with the label `{}`",
                        label.name
                    ),
                ),
            }
        }
        for (i, g) in given.iter().enumerate() {
            if g.is_none() {
                self.sub(&old_fields[i].1, &new_fields[i].1, base.span);
            }
        }
        // §4.8: the known type of the call of `C` with the given fields and
        // the other fields of `v`.
        let base_args = match known::member(&bk, Some(Head::Named(TypeRef::Custom(ctor.ty)))) {
            Some(Ty::Con(_, xs)) => Some(xs),
            _ => None,
        };
        let ks: Vec<Ty> = given
            .into_iter()
            .enumerate()
            .map(|(i, g)| g.unwrap_or_else(|| self.field_known(ctor, i, base_args.as_deref())))
            .collect();
        let k = match known::one_fn(&self.ctor_known(ctor)) {
            Some(f) => self.call_known(&f, &ks),
            None => Ty::Top,
        };
        (Ty::named(TypeRef::Custom(ctor.ty), new), k)
    }

    /// §8.2: the type of the value of a segment.
    pub fn format(&mut self, f: Option<&Format>, span: Span) -> Ty {
        let Some(f) = f else {
            return self.prim(BuiltinType::Int);
        };
        let name = f.name.name.as_str();
        let Some(ty) = known::format_prim(name) else {
            self.error(
                code::BAD_FORMAT,
                f.name.span,
                format!("there is no bit array format `{name}` (§8.2)"),
            );
            return self.s.fresh();
        };
        // Whether the format needs a length, has none, or may have one.
        let arg = match name {
            "u" | "i" => Some(true),
            "bits" | "bytes" => None,
            _ => Some(false),
        };
        match (arg, &f.arg) {
            (Some(true), None) => self.error(
                code::BAD_FORMAT,
                f.name.span,
                format!("the format `{name}` needs a length: `{name}(n)` (§8.2)"),
            ),
            (Some(false), Some(_)) => self.error(
                code::BAD_FORMAT,
                f.name.span,
                format!("the format `{name}` has no length (§8.2)"),
            ),
            _ => {}
        }
        if let Some(a) = &f.arg {
            let t = self.infer(a);
            let int = self.prim(BuiltinType::Int);
            self.sub(&t, &int, a.span);
        }
        let _ = span;
        self.prim(ty)
    }

    // ---- Blocks, functions, and bindings ----

    /// Rule 9: `{ s1 … sn e }`. The last expression has the expected type
    /// of the block, and the block has its known type (§4.8).
    pub fn block(&mut self, stmts: &[Stmt], span: Span, expected: &Ty) -> (Ty, Ty) {
        for (i, s) in stmts.iter().enumerate() {
            match s {
                Stmt::Bind(b) => self.local_binding(b),
                Stmt::Expr(e) if i + 1 == stmts.len() => return self.infer_k(e, expected),
                Stmt::Expr(e) => {
                    self.infer(e);
                }
                Stmt::Env { ty, value, span } => {
                    // `! T = e1` and the rest `r`: `A ? E ! N1 | M - T`.
                    self.check_places(ty, Pol::Pos, 0);
                    let t = self.conv(ty, &HashMap::new());
                    if let Some(super::Head::Named(r)) = t.head() {
                        self.out.services.insert((self.module, *span), r);
                    }
                    let v = self.infer(value);
                    self.sub2(&v, &t, value.span, ty.span);
                    self.effects.push(Effects::default());
                    let a = self.block(&stmts[i + 1..], *span, expected);
                    let (err, env) = self.pop_effects(*span);
                    self.add_err(err);
                    let rest = self.s.env_diff(env, vec![t]);
                    self.flush(*span, None);
                    self.add_env(rest);
                    return a;
                }
            }
        }
        // The last statement is not an expression: reported by the parser.
        let _ = span;
        let nil = Ty::named(self.b.nil, Vec::new());
        (nil.clone(), nil)
    }

    /// Rule 4: a function expression, and its known type (§4.8). With an
    /// expected function type with as many parameters, a parameter with no
    /// type written at its top gets the known type of the matching
    /// parameter.
    fn function(&mut self, f: &Function, expected: &Ty) -> (Ty, Ty) {
        let count = f
            .params
            .iter()
            .filter(|p| matches!(p, Param::Pattern { .. }))
            .count();
        let expected: Vec<Ty> = match known::members(expected).as_slice() {
            [Ty::Fn(g)] if g.params.len() == count => {
                g.params.iter().map(|p| p.1.clone()).collect()
            }
            _ => Vec::new(),
        };
        let mut params = Vec::new();
        let mut env_tys = Vec::new();
        let mut fails = false;
        for p in &f.params {
            match p {
                Param::Pattern { pattern, label } => {
                    // The values of a parameter come from the calls.
                    let start = self.s.vars.len();
                    let t = self.param(pattern);
                    self.s.mark_inputs(start);
                    fails |= self.may_fail(pattern, &t);
                    let k = match known::top_written(pattern) {
                        Some(ty) => self.kconv(ty, &HashMap::new()),
                        None => expected.get(params.len()).cloned().unwrap_or(Ty::Top),
                    };
                    self.fit_known(pattern, &k, false);
                    params.push((label.clone(), t));
                }
                Param::Env { var, name, ty } => {
                    self.check_places(ty, Pol::Pos, 0);
                    let t = self.conv(ty, &HashMap::new());
                    if let Some(super::Head::Named(r)) = t.head() {
                        self.out.services.insert((self.module, name.span), r);
                    }
                    let k = self.kconv(ty, &HashMap::new());
                    self.kvars.insert(*var, k);
                    self.locals.insert(*var, Scheme::mono(t.clone()));
                    env_tys.push(t);
                }
            }
        }
        for p in &f.params {
            if let Param::Pattern { pattern, .. } = p {
                self.check_pattern_places(pattern, Pol::Neg);
            }
        }
        if let Some(r) = &f.ret {
            for t in [Some(&r.value), r.err.as_ref(), r.env.as_ref()]
                .into_iter()
                .flatten()
            {
                self.check_places(t, Pol::Pos, 0);
            }
        }
        let ret = f.ret.as_ref().map(|r| {
            let value = self.conv(&r.value, &HashMap::new());
            let err = r
                .err
                .as_ref()
                .map_or(Ty::Never, |t| self.conv(t, &HashMap::new()));
            let env = r
                .env
                .as_ref()
                .map_or(Ty::Never, |t| self.conv(t, &HashMap::new()));
            (value, err, env)
        });
        // §4.8: the body of a function with a written `->` part has that
        // type as its expected type.
        let body_expected = f
            .ret
            .as_ref()
            .map_or(Ty::Top, |r| self.kconv(&r.value, &HashMap::new()));
        self.effects.push(Effects::default());
        let (a, body_known) = self.block(&f.body.stmts, f.body.span, &body_expected);
        let known = self.fn_known(f, &expected, body_known);
        let (err, env) = self.pop_effects(f.span);
        let assert = Ty::named(self.b.assert, Vec::new());
        let t = match (ret, &f.ret) {
            (Some((value, rerr, renv)), Some(written)) => {
                // §6.8: the written `->` part is the type after the
                // parameters; `! N` has the environment parameters and
                // `Assert` when a parameter may fail.
                let body_span = match f.body.stmts.last() {
                    Some(Stmt::Expr(e)) => e.span,
                    _ => f.body.span,
                };
                let wspan = written.value.span;
                let espan = written.err.as_ref().map_or(wspan, |t| t.span);
                let nspan = written.env.as_ref().map_or(wspan, |t| t.span);
                self.sub2(&a, &value, body_span, wspan);
                self.sub2(&err, &rerr, f.body.span, espan);
                self.sub_env(&env, &renv, f.body.span, Some(nspan));
                for t in &env_tys {
                    self.sub_env(t, &renv, nspan, None);
                }
                if fails {
                    self.sub_env(&assert, &renv, nspan, None);
                }
                Ty::Fn(Box::new(FnTy {
                    params,
                    ret: value,
                    err: rerr,
                    env: renv,
                }))
            }
            _ => {
                let mut envs = vec![env];
                envs.extend(env_tys);
                if fails {
                    envs.push(assert);
                }
                let env = self.env_union(envs, f.span);
                Ty::Fn(Box::new(FnTy {
                    params,
                    ret: a,
                    err,
                    env,
                }))
            }
        };
        (t, known)
    }

    /// Rule 10 for a binding in a block, and rule 14.
    fn local_binding(&mut self, b: &Binding) {
        let mark = self.occurs_mark();
        let direct = b.direct;
        if direct {
            self.s.level += 1;
        }
        self.check_pattern_places(&b.pattern, Pol::Pos);
        self.new_tyvars(&b.ty_vars);
        let mut binders = Vec::new();
        super::pattern::binders(&b.pattern, &mut binders);
        let mut holders = Vec::new();
        if direct {
            // §6.10: inside its own value, a name is not general, and it is
            // known only from what is written (§4.8).
            for v in &binders {
                let h = self.s.fresh();
                self.locals.insert(*v, Scheme::mono(h.clone()));
                holders.push((*v, h));
            }
            self.own_known(b);
        }
        let expected = self.binding_expected(b);
        let (a, k) = self.infer_k(&b.value, &expected);
        self.fit(&b.pattern, &a);
        if self.may_fail(&b.pattern, &a) {
            self.add_env(Ty::named(self.b.assert, Vec::new()));
        }
        for (v, h) in holders {
            if let Some(s) = self.locals.get(&v).cloned() {
                self.sub(&s.ty, &h, b.pattern.span);
            }
        }
        self.check_occurs(mark, b.pattern.span);
        if direct {
            self.s.level -= 1;
            let level = self.s.level;
            for v in &binders {
                if let Some(s) = self.locals.get(v).cloned() {
                    let ty = self.generalize(&s.ty, level);
                    // §4.8: a generalized name has its type scheme.
                    let known = display::known(&self.s, &ty, level);
                    self.kvars.insert(*v, known);
                    self.locals.insert(*v, Scheme { level, ty });
                }
            }
        } else {
            self.fit_known(&b.pattern, &k, false);
        }
    }

    /// Rule 14: the type scheme of a generalized binding, in its simpler
    /// form (§4.7): the bounds of its variables are coalesced and simplified
    /// once, so that each use copies only the simplified type. Variables at
    /// `level` or below are not generalized and are kept as they are.
    fn generalize(&mut self, t: &Ty, level: u32) -> Ty {
        let simple = match display::simplify(&self.s, t, Some(level)) {
            Ok(simple) => simple,
            // Reported by the occurs check of the binding.
            Err(()) => return t.clone(),
        };
        if has_both(&simple) {
            // An argument that does not change, with different bounds as
            // an output and as an input: keep the bounds.
            return t.clone();
        }
        let mut map = HashMap::new();
        self.rebuild(&simple, level, &mut map, false)
    }

    /// A simplified type as a solver type: each variable that is
    /// generalized is a new variable with no bounds, and `Top` is a new
    /// variable.
    fn rebuild(&mut self, t: &Ty, level: u32, map: &mut HashMap<super::V, Ty>, in_h: bool) -> Ty {
        let mut go = |c: &mut Self, x: &Ty| c.rebuild(x, level, map, in_h);
        match t {
            Ty::Var(v)
                if self
                    .s
                    .vars
                    .get(v.0 as usize)
                    .is_some_and(|x| x.level <= level) =>
            {
                t.clone()
            }
            Ty::Var(v) => {
                if let Some(n) = map.get(v) {
                    return n.clone();
                }
                let n = self.s.fresh_at(level + 1);
                self.s.vars[n.0 as usize].name =
                    self.s.vars.get(v.0 as usize).and_then(|x| x.name.clone());
                map.insert(*v, Ty::Var(n));
                Ty::Var(n)
            }
            // Only the type constructors of `H` matter in `A - H`, so a part
            // of `H` with no effect stays `Top`.
            Ty::Top if in_h => Ty::Top,
            Ty::Top => Ty::Var(self.s.fresh_at(level + 1)),
            Ty::Diff(a, h) => {
                let a = go(self, a);
                let h: Vec<Ty> = h
                    .iter()
                    .map(|x| self.rebuild(x, level, map, true))
                    .collect();
                // The scheme keeps the constraint of the difference (§4.4):
                // a new variable has no bounds, so it is given again.
                if let Ty::Var(v) = &a
                    && self.s.vars[v.0 as usize].level > level
                {
                    let hs: Vec<Ty> = h.iter().map(|x| self.s.no_top(x, level + 1)).collect();
                    self.s.diff_bound(&a, &hs, false);
                }
                Ty::Diff(Box::new(a), h)
            }
            Ty::Con(h, args) => Ty::Con(*h, args.iter().map(|a| go(self, a)).collect()),
            Ty::Union(ms) => Ty::Union(ms.iter().map(|a| go(self, a)).collect()),
            Ty::Inter(ms) => Ty::Inter(ms.iter().map(|a| go(self, a)).collect()),
            Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
                params: f
                    .params
                    .iter()
                    .map(|(l, p)| (l.clone(), go(self, p)))
                    .collect(),
                ret: go(self, &f.ret),
                err: go(self, &f.err),
                env: go(self, &f.env),
            })),
            Ty::Never | Ty::Both(..) => t.clone(),
        }
    }
}

/// The custom types that a type uses.
fn uses_types(t: &Ty, out: &mut Vec<TypeId>) {
    match t {
        Ty::Con(h, xs) => {
            if let super::Head::Named(TypeRef::Custom(id)) = h
                && !out.contains(id)
            {
                out.push(*id);
            }
            xs.iter().for_each(|x| uses_types(x, out));
        }
        Ty::Union(xs) | Ty::Inter(xs) => xs.iter().for_each(|x| uses_types(x, out)),
        Ty::Fn(f) => {
            f.params.iter().for_each(|p| uses_types(&p.1, out));
            [&f.ret, &f.err, &f.env]
                .into_iter()
                .for_each(|x| uses_types(x, out));
        }
        Ty::Diff(a, h) => {
            uses_types(a, out);
            h.iter().for_each(|x| uses_types(x, out));
        }
        Ty::Both(a, b) => {
            uses_types(a, out);
            uses_types(b, out);
        }
        _ => {}
    }
}

fn has_both(t: &Ty) -> bool {
    match t {
        Ty::Both(..) => true,
        Ty::Con(_, xs) | Ty::Union(xs) | Ty::Inter(xs) => xs.iter().any(has_both),
        Ty::Fn(f) => {
            f.params.iter().any(|p| has_both(&p.1))
                || [&f.ret, &f.err, &f.env].into_iter().any(has_both)
        }
        Ty::Diff(a, _) => has_both(a),
        _ => false,
    }
}

/// §10.3: the built-in types that the host can take.
fn host_builtin(b: BuiltinType) -> bool {
    matches!(
        b,
        BuiltinType::Int | BuiltinType::Float | BuiltinType::String | BuiltinType::BitArray
    )
}

/// §4.5 rules 6 and 7: the variance of every parameter of every custom
/// type, from the definitions, by a fixpoint over all of them.
fn variances(p: &Program) -> HashMap<TypeId, Vec<Variance>> {
    let mut table: HashMap<TypeId, Vec<Variance>> = HashMap::new();
    for (i, t) in p.types.iter().enumerate() {
        table.insert(TypeId(i as u32), vec![Variance::Bi; t.params.len()]);
    }
    loop {
        let mut changed = false;
        for (i, t) in p.types.iter().enumerate() {
            let TypeBody::Custom(ctors) = &t.body else {
                continue;
            };
            let id = TypeId(i as u32);
            let mut acc = table[&id].clone();
            for c in ctors {
                for f in &c.fields {
                    walk_variance(p, &f.ty, Variance::Co, &t.params, &mut acc, &table, 0);
                }
            }
            if acc != table[&id] {
                table.insert(id, acc);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    table
}

fn walk_variance(
    p: &Program,
    t: &Type,
    v: Variance,
    params: &[(String, TyVarId)],
    acc: &mut [Variance],
    table: &HashMap<TypeId, Vec<Variance>>,
    depth: u32,
) {
    let go = |t: &Type, v: Variance, acc: &mut [Variance]| {
        walk_variance(p, t, v, params, acc, table, depth)
    };
    match &t.kind {
        TypeKind::Var(tv) => {
            if let Some(i) = params.iter().position(|x| x.1 == *tv) {
                acc[i] = acc[i].join(v);
            }
        }
        TypeKind::Named { ty, args } => {
            if let TypeRef::Custom(id) = ty
                && let TypeBody::Alias(body) = &p.ty(*id).body
            {
                if depth < 64 {
                    let def = p.ty(*id);
                    let map: HashMap<TyVarId, &Type> =
                        def.params.iter().map(|x| x.1).zip(args).collect();
                    let expanded = subst(body, &map);
                    walk_variance(p, &expanded, v, params, acc, table, depth + 1);
                }
                return;
            }
            let vs = match ty {
                TypeRef::Custom(id) => table[id].clone(),
                TypeRef::Builtin(b) => builtin_variance(*b),
            };
            for (a, w) in args.iter().zip(vs) {
                let pv = match w {
                    Variance::Bi => continue,
                    Variance::Co => v,
                    Variance::Contra => v.flip(),
                    Variance::Inv => Variance::Inv,
                };
                go(a, pv, acc);
            }
        }
        TypeKind::Tuple(ts) | TypeKind::Union(ts) | TypeKind::Inter(ts) => {
            ts.iter().for_each(|a| go(a, v, acc))
        }
        TypeKind::Diff(ts) => {
            go(&ts[0], v, acc);
            ts[1..].iter().for_each(|a| go(a, Variance::Inv, acc));
        }
        TypeKind::Fn {
            params: ps,
            ret,
            err,
            env,
        } => {
            ps.iter().for_each(|x| go(&x.ty, v.flip(), acc));
            go(ret, v, acc);
            err.iter().chain(env).for_each(|x| go(x, v, acc));
        }
        TypeKind::Hole | TypeKind::Error => {}
    }
}
