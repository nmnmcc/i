//! Frozen inference graph observations and typed expansion transactions.
use super::{
    FnTy, Head, Ty, V,
    infer::{Checker, Scheme},
    solve::{Pol, Solver},
};
use crate::hir::*;
use ilang_syntax::{
    lisp::{
        Datum, DatumKind,
        syntax::{Node, Scope, ScopeIdentity, Syntax, TypeHandle},
    },
    span::Span,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU32, AtomicU64, Ordering},
    },
};

static NEXT_SNAPSHOT: AtomicU64 = AtomicU64::new(1);
#[derive(Clone)]
pub(super) struct HandleArena {
    id: u64,
    next: Arc<AtomicU32>,
    pub types: BTreeMap<u32, Ty>,
}
impl HandleArena {
    pub fn new() -> Self {
        Self {
            id: NEXT_SNAPSHOT.fetch_add(1, Ordering::Relaxed),
            next: Arc::new(AtomicU32::new(0)),
            types: BTreeMap::new(),
        }
    }
    fn intern(&mut self, t: &Ty) -> u32 {
        if let Some((index, _)) = self.types.iter().find(|(_, old)| *old == t) {
            return *index;
        }
        let index = self.next.fetch_add(1, Ordering::Relaxed);
        assert!(index < 0x8000_0000, "typed handle identity space exhausted");
        self.types.insert(index, t.clone());
        index
    }
}
type Parser = dyn Fn(&Syntax) -> Result<Type, String> + Send + Sync;
pub struct TypedMacroQuery {
    pub(super) state: Mutex<QueryState>,
    parser: OnceLock<Arc<Parser>>,
    id: u64,
}
pub(super) struct QueryState {
    pub solver: Solver,
    pub signature: Ty,
    pub expressions: HashMap<Span, Ty>,
    pub inputs: Vec<(Syntax, Span)>,
    pub argument_order: Vec<u32>,
    pub substitution: HashMap<V, Ty>,
    observed: HashSet<V>,
    arena: HandleArena,
    answers: HashMap<u32, Syntax>,
    pub program: Arc<Program>,
    pub reads: super::value_types::Reads,
    pub tyvars: HashMap<TyVarId, Ty>,
    pub module: ModuleId,
    fixed: HashSet<V>,
    effects: HashMap<Span, (Ty, Ty)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MacroTruth {
    Yes,
    No,
    NotKnown,
}

pub(super) fn variables(t: &Ty) -> Vec<V> {
    let mut vs = HashSet::new();
    super::alias::map_vars(t, &mut |v| {
        vs.insert(v);
        Ty::Var(v)
    });
    vs.into_iter().collect()
}
fn occurrences(s: &Solver, t: &Ty, pol: Pol, out: &mut HashMap<V, Pol>) {
    match t {
        Ty::Var(v) => {
            out.entry(*v)
                .and_modify(|p| {
                    if *p != pol {
                        *p = Pol::Both;
                    }
                })
                .or_insert(pol);
        }
        Ty::Graph(g) => {
            for v in g.variables() {
                out.insert(v, Pol::Both);
            }
        }
        Ty::Con(h, _, xs) => {
            for (x, variance) in xs.iter().zip(s.variances(*h, xs.len())) {
                occurrences(s, x, pol.by(variance), out);
            }
        }
        Ty::Fn(f) => {
            for (_, p) in &f.params {
                occurrences(s, p, pol.flip(), out);
            }
            for t in [&f.ret, &f.err] {
                occurrences(s, t, pol, out);
            }
            occurrences_env(s, &f.env, pol, out);
        }
        Ty::Union(xs) | Ty::Inter(xs) => {
            for x in xs {
                occurrences(s, x, pol, out);
            }
        }
        Ty::Diff(x, _) => occurrences(s, x, pol, out),
        Ty::Both(a, b) => {
            occurrences(s, a, pol, out);
            occurrences(s, b, pol.flip(), out);
        }
        Ty::Never | Ty::Top => {}
    }
}
fn occurrences_env(s: &Solver, t: &Ty, pol: Pol, out: &mut HashMap<V, Pol>) {
    match t {
        Ty::Con(_, _, xs) => {
            for x in xs {
                occurrences(s, x, Pol::Both, out);
            }
        }
        Ty::Union(xs) | Ty::Inter(xs) => {
            for x in xs {
                occurrences_env(s, x, pol, out);
            }
        }
        Ty::Diff(base, _) => occurrences_env(s, base, pol, out),
        _ => occurrences(s, t, pol, out),
    }
}
pub(super) fn reachable(s: &Solver, roots: impl IntoIterator<Item = V>) -> HashSet<V> {
    let mut seen = HashSet::new();
    let mut work: Vec<_> = roots.into_iter().collect();
    // Obligations connect variables in both directions, including variables
    // occurring together in a structured bound.
    let mut adjacency: HashMap<V, Vec<V>> = HashMap::new();
    for (i, state) in s.vars.iter().enumerate() {
        let v = V(i as u32);
        let neighbors: Vec<_> = state
            .lower
            .iter()
            .chain(&state.upper)
            .chain(&state.handled)
            .flat_map(variables)
            .chain(state.from.iter().chain(&state.into).copied())
            .collect();
        for w in neighbors {
            adjacency.entry(v).or_default().push(w);
            adjacency.entry(w).or_default().push(v);
        }
    }
    while let Some(v) = work.pop() {
        if seen.insert(v) {
            work.extend(adjacency.get(&v).into_iter().flatten().copied());
        }
    }
    seen
}
fn observe(s: &Solver, t: &Ty, fixed: &HashSet<V>, parts: Option<&FnTy>) -> HashMap<V, Ty> {
    let mut ps = HashMap::new();
    if let Some(f) = parts {
        for t in f.params.iter().map(|(_, t)| t).chain([&f.ret, &f.err]) {
            occurrences(s, t, Pol::Pos, &mut ps);
        }
        occurrences_env(s, &f.env, Pol::Pos, &mut ps);
    } else {
        occurrences(s, t, Pol::Pos, &mut ps);
    }
    let mut result = HashMap::new();
    for (v, pol) in ps {
        if fixed.contains(&v) {
            continue;
        }
        let state = &s.vars[v.0 as usize];
        let value = match pol {
            Pol::Pos => {
                let mut trial = s.clone();
                Some(trial.union(state.lower.clone()))
            }
            Pol::Neg if !state.upper.is_empty() => {
                let mut trial = s.clone();
                Some(trial.inter(state.upper.clone()))
            }
            _ => None,
        };
        if let Some(value) = value
            && !variables(&value).contains(&v)
        {
            result.insert(v, value);
        }
    }
    result
}
impl TypedMacroQuery {
    pub(super) fn new(
        c: &Checker<'_>,
        signature: Ty,
        expressions: HashMap<Span, Ty>,
        fixed: HashSet<V>,
        span: Span,
        argument_types: &[Ty],
        input_spans: &HashSet<Span>,
    ) -> Self {
        let mut observation = if let Ty::Fn(f) = &signature {
            f.as_ref().clone()
        } else {
            unreachable!("typed signature is a function")
        };
        observation
            .params
            .extend(argument_types.iter().cloned().map(|t| (None, t)));
        let substitution = observe(&c.s, &signature, &fixed, Some(&observation));
        Self {
            id: c.typed_handles.id,
            parser: OnceLock::new(),
            state: Mutex::new(QueryState {
                solver: c.s.clone(),
                signature,
                expressions: expressions
                    .into_iter()
                    .filter(|(s, _)| input_spans.contains(s))
                    .collect(),
                inputs: Vec::new(),
                observed: variables(&Ty::Fn(Box::new(observation)))
                    .into_iter()
                    .collect(),
                substitution,
                arena: c.typed_handles.clone(),
                answers: HashMap::new(),
                program: Arc::new(c.p.clone()),
                fixed,
                effects: c
                    .expression_effects
                    .iter()
                    .filter(|((m, _), _)| *m == c.module)
                    .map(|((_, s), t)| (*s, t.clone()))
                    .collect(),
                reads: c.reads.clone(),
                tyvars: c.tyvars.clone(),
                module: c.module,
                argument_order: c
                    .out
                    .args
                    .get(&(c.module, span))
                    .cloned()
                    .expect("typed call has placed arguments"),
            }),
        }
    }
    pub fn input_span(&self, syntax: &Syntax) -> Option<Span> {
        self.state
            .lock()
            .expect("query snapshot")
            .inputs
            .iter()
            .find(|(x, _)| x.same_input(syntax))
            .map(|(_, span)| *span)
    }
    pub fn argument_is_effectful(&self, span: Span) -> bool {
        let s = self.state.lock().expect("query snapshot");
        let Some((err, env)) = s.effects.get(&span) else {
            return false;
        };
        let mut trial = s.solver.clone();
        trial.freeze();
        trial.constrain(err, &Ty::Never);
        trial.constrain_env(env, &Ty::Never);
        !trial.failures.is_empty()
    }
    pub fn argument_order(&self) -> Vec<u32> {
        self.state
            .lock()
            .expect("query snapshot")
            .argument_order
            .clone()
    }
    pub fn set_parser(&self, parser: Arc<Parser>) {
        assert!(self.parser.set(parser).is_ok(), "one parser per typed run");
    }
    pub fn set_arguments(&self, arguments: &[Syntax]) {
        let mut s = self.state.lock().expect("type query snapshot");
        let mut work: Vec<_> = arguments.to_vec();
        while let Some(x) = work.pop() {
            match x.node() {
                Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                    work.extend(xs.iter().cloned())
                }
                Node::Quote(x) | Node::Unquote(x) | Node::Splice(x) => work.push(x.clone()),
                _ => {}
            }
            let mut candidates: Vec<_> = s.expressions.keys().copied().collect();
            candidates.sort_by_key(|s| (s.start, s.end));
            let occurrence = candidates.into_iter().find(|span| {
                if let Some(origin) = s
                    .program
                    .module(s.module)
                    .scoped
                    .as_ref()
                    .and_then(|sc| sc.origin(*span))
                {
                    origin.syntax.same_input(&x)
                } else {
                    *span == x.span()
                }
            });
            if let Some(span) = occurrence {
                s.inputs.push((x, span));
            }
        }
    }
    pub fn type_of(&self, syntax: &Syntax) -> Result<Syntax, String> {
        let mut s = self.state.lock().expect("type query snapshot");
        let Some(span) = s
            .inputs
            .iter()
            .find(|(x, _)| x.same_input(syntax))
            .map(|(_, span)| *span)
        else {
            return Err(
                "type_of needs an original argument expression or pattern-bound name".into(),
            );
        };
        let ty = s
            .expressions
            .get(&span)
            .expect("registered input has a type")
            .clone();
        let extra = observe(&s.solver, &ty, &s.fixed, None);
        let signature_vars = s.observed.clone();
        for (v, t) in extra {
            if !signature_vars.contains(&v) {
                s.substitution.entry(v).or_insert(t);
            }
        }
        let ty = substitute(&ty, &s.substitution);
        Ok(self.syntax(&mut s, &ty))
    }
    pub fn signature(&self) -> Syntax {
        let mut s = self.state.lock().expect("type query snapshot");
        let ty = substitute(&s.signature, &s.substitution);
        self.syntax(&mut s, &ty)
    }
    pub fn subtype(&self, a: &Syntax, b: &Syntax) -> Result<MacroTruth, String> {
        let mut s = self.state.lock().expect("type query snapshot");
        let a = self.read(&mut s, a)?;
        let b = self.read(&mut s, b)?;
        let mut trial = s.solver.clone();
        trial.freeze();
        trial.constrain(&a, &b);
        if trial
            .failures
            .iter()
            .any(|f| f.kind == super::solve::FailKind::ResourceLimit)
        {
            return Err("E0546: type proof resource limit exceeded".into());
        }
        Ok(if trial.failures.is_empty() {
            MacroTruth::Yes
        } else if variables(&a).is_empty() && variables(&b).is_empty() {
            MacroTruth::No
        } else {
            MacroTruth::NotKnown
        })
    }
    pub fn members(&self, syntax: &Syntax) -> Result<Vec<Syntax>, String> {
        let mut s = self.state.lock().expect("type query snapshot");
        let mut t = self.read(&mut s, syntax)?;
        if variables(&t).is_empty() && !matches!(t, Ty::Graph(_)) {
            let mut scratch = s.solver.clone();
            scratch.freeze();
            t = scratch.normalize_mode(t, super::solve::Mode::Value);
            if !scratch.failures.is_empty() {
                return Err("members needs a valid type".into());
            }
        }
        fn split(t: Ty, out: &mut Vec<Ty>) {
            match t {
                Ty::Never => {}
                Ty::Graph(g) if !g.record_only() => {
                    if let Some(view) = g.view() {
                        split(view, out);
                    } else {
                        out.push(Ty::Graph(g));
                    }
                }
                Ty::Union(ts) => {
                    for t in ts {
                        split(t, out);
                    }
                }
                Ty::Con(h, values, args) if values.is_finite() => {
                    for v in values.items() {
                        out.push(Ty::Con(
                            h,
                            super::lits::ValueSet::only([v.clone()]).expect("one value"),
                            args.clone(),
                        ));
                    }
                }
                t => out.push(t),
            }
        }
        let mut ts = Vec::new();
        split(t, &mut ts);
        let mut answer: Vec<_> = ts.iter().map(|t| self.syntax(&mut s, t)).collect();
        answer.sort_by_key(Syntax::show);
        Ok(answer)
    }
    fn read(&self, s: &mut QueryState, syntax: &Syntax) -> Result<Ty, String> {
        let mut work = vec![syntax];
        while let Some(node) = work.pop() {
            let stale = match node.node() {
                Node::TypeView(h) => h.snapshot != self.id || !s.arena.types.contains_key(&h.index),
                _ => {
                    matches!(node.scope_identity(),Some(ScopeIdentity::TypeVariable{variable,..}) if *variable>>32!=self.id || !s.arena.types.contains_key(&(*variable as u32)))
                }
            };
            if stale {
                return Err("type handle belongs to a different module-check snapshot".into());
            }
            match node.node() {
                Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                    work.extend(xs)
                }
                _ => {}
            }
        }
        if let Node::TypeView(h) = syntax.node() {
            return Ok(s.arena.types[&h.index].clone());
        }
        let ty = self
            .parser
            .get()
            .expect("typed runner installs its type parser")(syntax)?;
        convert_query(s, &ty, &HashMap::new())
    }

    fn handle(&self, s: &mut QueryState, t: &Ty) -> u32 {
        s.arena.intern(t)
    }
    fn syntax(&self, s: &mut QueryState, t: &Ty) -> Syntax {
        let scope = Scope {
            module: s.program.module(s.module).source.0,
            marks: Default::default(),
        };
        self.syntax_rows(
            s,
            t,
            &scope,
            ilang_syntax::lisp::definitions::NEW_ROWS,
        )
    }
    /// A type as syntax, in the rows before syntax three or (`new_rows`) in
    /// the rows of syntax-3.md §6.8. The rows differ for function types, for
    /// records with named fields and for the empty tuple.
    fn syntax_rows(&self, s: &mut QueryState, t: &Ty, scope: &Scope, new_rows: bool) -> Syntax {
        let atom = |kind| {
            Syntax::from_datum(
                &Datum {
                    kind,
                    span: Span::new(0, 0),
                    docs: Vec::new(),
                },
                scope,
            )
            .as_answer()
        };
        let round = |xs| {
            Syntax::make(Node::Round(xs), &atom(DatumKind::Symbol("->".into())))
                .expect("query form")
                .as_answer()
        };
        let form = |name: &str, mut xs: Vec<Syntax>| {
            xs.insert(0, atom(DatumKind::Symbol(name.into())));
            round(xs)
        };
        if let Ty::Graph(g) = t {
            if matches!(
                g.node(),
                super::records::Node::Never
                    | super::records::Node::Var(_)
                    | super::records::Node::Con(..)
                    | super::records::Node::Fn(_)
            ) {
                return self.syntax_rows(
                    s,
                    &g.view().expect("ordinary graph head has a surface view"),
                    scope,
                    new_rows,
                );
            }
            if let Some(fields) = g.fields()
                && fields
                    .iter()
                    .all(|(label, _)| matches!(label, super::records::Label::Name(_)))
            {
                if fields.is_empty() {
                    return atom(DatumKind::UpName("Record".into())).with_scope_identity(
                        ScopeIdentity::Builtin {
                            name: "Record".into(),
                        },
                    );
                }
                let fields = fields
                    .iter()
                    .map(|(label, t)| {
                        let super::records::Label::Name(name) = label else {
                            unreachable!("surface record label")
                        };
                        let t = self.syntax_rows(s, t, scope, new_rows);
                        if new_rows {
                            // A labelled item of a data type (§6.8).
                            form("=", vec![atom(DatumKind::Name(name.to_string())), t])
                        } else {
                            form(":", vec![atom(DatumKind::String(name.to_string())), t])
                        }
                    })
                    .collect();
                return if new_rows {
                    form(",", fields)
                } else {
                    round(fields)
                };
            }
        }
        match t {
            Ty::Graph(_) | Ty::Top | Ty::Both(..) => {
                let index = self.handle(s, t);
                if let Some(answer) = s.answers.get(&index) {
                    return answer.clone();
                }
                let answer = Syntax::type_view(
                    TypeHandle {
                        snapshot: self.id,
                        index,
                    },
                    &atom(DatumKind::UpName("TypeView".into())),
                );
                s.answers.insert(index, answer.clone());
                answer
            }
            Ty::Var(_) => {
                let index = self.handle(s, t);
                if let Some(answer) = s.answers.get(&index) {
                    return answer.clone();
                }
                let name = format!("A{index}");
                let answer = atom(DatumKind::UpName(name.clone())).with_scope_identity(
                    ScopeIdentity::TypeVariable {
                        variable: (self.id << 32) | u64::from(index),
                        name,
                    },
                );
                s.answers.insert(index, answer.clone());
                answer
            }
            Ty::Never => atom(DatumKind::UpName("Never".into())).with_scope_identity(
                ScopeIdentity::Builtin {
                    name: "Never".into(),
                },
            ),
            Ty::Con(h, values, args) => {
                if values.is_finite() && args.is_empty() {
                    let b = if matches!(h, Head::Named(TypeRef::Builtin(BuiltinType::Int))) {
                        BuiltinType::Int
                    } else {
                        BuiltinType::String
                    };
                    let xs: Vec<_> = values
                        .items()
                        .map(|v| {
                            let lit = if b == BuiltinType::Int {
                                ilang_syntax::ast::LitValue::Int(v.parse().expect("integer type"))
                            } else {
                                ilang_syntax::ast::LitValue::Str(v.to_string())
                            };
                            let text = lit.text();
                            let d = ilang_syntax::lisp::parse(&text).expect("literal type prints");
                            Syntax::from_datum(&d.datums[0], scope).as_answer()
                        })
                        .collect();
                    return if xs.len() == 1 {
                        xs[0].clone()
                    } else {
                        form("|", xs)
                    };
                }
                let name = match h {
                    Head::Named(TypeRef::Builtin(b)) => atom(DatumKind::UpName(b.name().into()))
                        .with_scope_identity(ScopeIdentity::Builtin {
                            name: b.name().into(),
                        }),
                    Head::Named(TypeRef::Custom(id)) => {
                        let def = s.program.ty(*id);
                        let module = s.program.module(def.module);
                        if Some(def.module) == s.program.prelude {
                            atom(DatumKind::UpName(def.name.clone())).with_scope_identity(
                                ScopeIdentity::Builtin {
                                    name: def.name.clone(),
                                },
                            )
                        } else {
                            let def_scope = module
                                .scoped
                                .as_ref()
                                .and_then(|sc| sc.origin(def.span))
                                .map(|o| o.syntax.scope().clone())
                                .unwrap_or_else(|| Scope {
                                    module: module.source.0,
                                    marks: Default::default(),
                                });
                            Syntax::from_datum(
                                &Datum {
                                    kind: DatumKind::UpName(def.name.clone()),
                                    span: def.span,
                                    docs: Vec::new(),
                                },
                                &def_scope,
                            )
                            .as_answer()
                        }
                    }
                    Head::Tuple(_) => atom(DatumKind::Symbol(",".into())),
                    _ => unreachable!("type constructor"),
                };
                let base = if args.is_empty() && !(new_rows && matches!(h, Head::Tuple(_))) {
                    name
                } else {
                    // In the rows of §6.8 the empty data type is `(,)`.
                    let mut xs = vec![name];
                    xs.extend(args.iter().map(|t| self.syntax_rows(s, t, scope, new_rows)));
                    round(xs)
                };
                if !values.is_all() {
                    let mut xs = vec![base];
                    for v in values.items() {
                        let lit = if t.scalar() == Some(BuiltinType::Int) {
                            ilang_syntax::ast::LitValue::Int(v.parse().expect("integer"))
                        } else {
                            ilang_syntax::ast::LitValue::Str(v.to_string())
                        };
                        let d = ilang_syntax::lisp::parse(&lit.text()).expect("literal");
                        xs.push(Syntax::from_datum(&d.datums[0], scope));
                    }
                    form("-", xs)
                } else {
                    base
                }
            }
            Ty::Fn(f) => {
                let params: Vec<_> = f
                    .params
                    .iter()
                    .map(|(label, t)| {
                        let t = self.syntax_rows(s, t, scope, new_rows);
                        if let Some(l) = label {
                            let head = if new_rows { "=" } else { ":" };
                            form(head, vec![atom(DatumKind::Name(l.clone())), t])
                        } else {
                            t
                        }
                    })
                    .collect();
                let mut ret = self.syntax_rows(s, &f.ret, scope, new_rows);
                if new_rows {
                    // `(-> (, T…) (<- R (, (Fail E) N… .. V)))` (§6.8): the
                    // reverse of `Cx::needs_row` in `syn3/lower.rs`. The
                    // error row is the entry `(Fail E)`; each member of the
                    // environment is an entry, and a member that is a type
                    // variable is a row variable, `.. V`. Needs that are one
                    // row variable and nothing else are written `(<- R V)`.
                    let mut needs = Vec::new();
                    if f.err != Ty::Never {
                        let err = self.syntax_rows(s, &f.err, scope, new_rows);
                        needs.push(round(vec![atom(DatumKind::UpName("Fail".into())), err]));
                    }
                    let members: &[Ty] = match &f.env {
                        Ty::Never => &[],
                        Ty::Union(members) => members,
                        one => std::slice::from_ref(one),
                    };
                    let only_row =
                        needs.is_empty() && matches!(members, [Ty::Var(_)]);
                    for member in members {
                        if matches!(member, Ty::Var(_)) && !only_row {
                            needs.push(atom(DatumKind::Symbol("..".into())));
                        }
                        needs.push(self.syntax_rows(s, member, scope, new_rows));
                    }
                    if only_row {
                        let row = needs.pop().expect("the row variable");
                        ret = form("<-", vec![ret, row]);
                    } else if !needs.is_empty() {
                        ret = form("<-", vec![ret, form(",", needs)]);
                    }
                    form("->", vec![form(",", params), ret])
                } else {
                    if f.err != Ty::Never {
                        let err = self.syntax_rows(s, &f.err, scope, new_rows);
                        ret = form("^", vec![ret, err]);
                    }
                    if f.env != Ty::Never {
                        let env = self.syntax_rows(s, &f.env, scope, new_rows);
                        ret = form("!", vec![ret, env]);
                    }
                    form("->", vec![round(params), ret])
                }
            }
            Ty::Union(xs) | Ty::Inter(xs) => {
                let name = if matches!(t, Ty::Union(_)) { "|" } else { "&" };
                form(name, xs.iter().map(|t| self.syntax_rows(s, t, scope, new_rows)).collect())
            }
            Ty::Diff(a, bs) => {
                let mut xs = vec![self.syntax_rows(s, a, scope, new_rows)];
                xs.extend(bs.iter().map(|t| self.syntax_rows(s, t, scope, new_rows)));
                form("-", xs)
            }
        }
    }
    /// A cache key contains the exact graph plus bounds and obligations.
    pub fn graph_bundle(&self) -> String {
        let s = self.state.lock().expect("type query snapshot");
        let mut expressions: Vec<_> = s.expressions.iter().collect();
        expressions.sort_by_key(|(span, _)| (span.start, span.end));
        let mut roots = vec![s.signature.clone()];
        roots.extend(expressions.into_iter().map(|(_, ty)| ty.clone()));
        roots.extend(s.arena.types.values().cloned());
        let mut substitutions: Vec<_> = s.substitution.iter().collect();
        substitutions.sort_by_key(|(v, _)| v.0);
        roots.extend(substitutions.iter().map(|(_, t)| (*t).clone()));
        roots.extend((0..s.solver.vars.len()).map(|i| Ty::Var(V(i as u32))));
        let scheme = Scheme::mono(Ty::tuple(roots));
        let artifact = super::scheme_wire::SchemeArtifact {
            catalog: Arc::new(super::scheme_wire::SchemeCatalog {
                solver: Mutex::new(s.solver.scheme_snapshot()),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        };
        let graph = artifact.text_with(&mut |t| match t {
            TypeRef::Builtin(b) => b.name().into(),
            TypeRef::Custom(id) => format!(
                "{}:{}",
                s.program.module(s.program.ty(id).module).path,
                s.program.ty(id).name
            ),
        });
        let mut fixed: Vec<_> = s.fixed.iter().map(|v| v.0).collect();
        fixed.sort_unstable();
        let mut observed: Vec<_> = s.observed.iter().map(|v| v.0).collect();
        observed.sort_unstable();
        serde_json::json!({"version":1,"graph_bundle":serde_json::from_str::<serde_json::Value>(&graph).expect("scheme bundle JSON"),"fixed":fixed,"observed":observed,"substitutions":substitutions.iter().map(|(v,_)|v.0).collect::<Vec<_>>(),"handle_indices":s.arena.types.keys().copied().collect::<Vec<_>>(),"variable_count":s.solver.vars.len()}).to_string()
    }
    pub(super) fn arena(&self) -> HandleArena {
        self.state.lock().expect("query snapshot").arena.clone()
    }
}
// Type queries denote sets. They do not create inference obligations or
// residual variables while reading an intersection or difference.
fn convert_query(s: &QueryState, t: &Type, local: &HashMap<TyVarId, Ty>) -> Result<Ty, String> {
    let go = |t| convert_query(s, t, local);
    Ok(match &t.kind {
        TypeKind::Lit(lit) => Ty::literal(lit),
        TypeKind::Var(v) => local
            .get(v)
            .or_else(|| s.tyvars.get(v))
            .cloned()
            .or_else(|| {
                (v.0 & 0x8000_0000 != 0)
                    .then(|| s.arena.types.get(&(v.0 & 0x7fff_ffff)).cloned())
                    .flatten()
            })
            .ok_or_else(|| "type query refers to an unknown frozen variable".to_string())?,
        TypeKind::Named { ty, args } => {
            let args: Vec<_> = args.iter().map(go).collect::<Result<_, _>>()?;
            if let TypeRef::Custom(id) = ty
                && matches!(s.program.ty(*id).body, TypeBody::Alias(_))
            {
                let template = s
                    .reads
                    .templates
                    .get(id)
                    .ok_or_else(|| "type query needs an already completed alias".to_string())?;
                if template.slots != template.params {
                    return Err("type query alias contains an unspecified _".into());
                }
                template.apply(args, || {
                    unreachable!("fully specified alias has no hole slots")
                })
            } else if *ty == TypeRef::Builtin(BuiltinType::Never) {
                Ty::Never
            } else {
                Ty::named(*ty, args)
            }
        }
        TypeKind::Tuple(parts) => Ty::tuple(parts.iter().map(go).collect::<Result<_, _>>()?),
        TypeKind::Record {
            base,
            fields,
            removed,
        } => {
            let labels = fields
                .iter()
                .map(|(label, _)| label)
                .chain(removed)
                .map(|label| super::records::written_label(label, s.solver.level))
                .collect();
            let fields = fields
                .iter()
                .map(|(label, t)| {
                    Ok((super::records::written_label(label, s.solver.level), go(t)?))
                })
                .collect::<Result<_, String>>()?;
            let record = s.solver.records.record(fields);
            if let Some(base) = base {
                let rest = s.solver.records.without(&go(base)?, labels);
                s.solver.records.inter(&[rest, record])
            } else {
                record
            }
        }
        TypeKind::Fn {
            params,
            ret,
            err,
            env,
        } => Ty::Fn(Box::new(FnTy {
            params: params
                .iter()
                .map(|p| Ok((p.label.as_ref().map(|l| l.name.clone()), go(&p.ty)?)))
                .collect::<Result<_, String>>()?,
            ret: go(ret)?,
            err: err
                .as_ref()
                .map(|t| go(t))
                .transpose()?
                .unwrap_or(Ty::Never),
            env: env
                .as_ref()
                .map(|t| go(t))
                .transpose()?
                .unwrap_or(Ty::Never),
        })),
        TypeKind::Union(parts) => {
            let mut flat = Vec::new();
            for part in parts {
                let part = go(part)?;
                super::solve::flatten_union(&[part], &mut flat);
            }
            let mut joined = super::solve::join_literals(flat);
            joined.retain(|t| *t != Ty::Never);
            if joined.iter().any(|t| matches!(t, Ty::Graph(_))) {
                s.solver.records.union(&joined)
            } else if joined.len() == 1 {
                joined.pop().expect("one union member")
            } else if joined.is_empty() {
                Ty::Never
            } else {
                Ty::Union(joined)
            }
        }
        TypeKind::Inter(parts) => Ty::Inter(parts.iter().map(go).collect::<Result<_, _>>()?),
        TypeKind::Diff(parts) => {
            let base = go(&parts[0])?;
            let heads = parts[1..].iter().map(go).collect::<Result<Vec<_>, _>>()?;
            let mut flattened = Vec::new();
            super::solve::flatten_union(&heads, &mut flattened);
            if flattened
                .iter()
                .any(|t| t.head().is_none() && *t != Ty::Never)
            {
                return Err("the right side of - needs members with a type constructor".into());
            }
            Ty::Diff(Box::new(base), flattened)
        }
        TypeKind::Hole | TypeKind::Error | TypeKind::Of { .. } => {
            return Err("type query needs a correct type without _ or value-type reads".into());
        }
    })
}
pub(super) fn substitute(t: &Ty, sub: &HashMap<V, Ty>) -> Ty {
    super::alias::map_vars(t, &mut |v| sub.get(&v).cloned().unwrap_or(Ty::Var(v)))
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
    fn prove(s: &Solver, a: &Ty, b: &Ty) -> bool {
        let mut t = s.clone();
        t.freeze();
        t.constrain(a, b);
        t.failures.is_empty()
    }
    #[test]
    fn typed_macro_rigid_variable_cannot_receive_guessed_lower_bound() {
        let mut s = solver();
        let a = s.fresh();
        assert!(!prove(&s, &int(), &a));
    }
    #[test]
    fn typed_macro_rigid_variable_has_no_fabricated_top() {
        let mut s = solver();
        let a = s.fresh();
        assert!(!prove(&s, &a, &int()));
    }
    #[test]
    fn typed_macro_rigid_lower_evidence_is_read_only() {
        let mut s = solver();
        let a = s.fresh();
        s.constrain(&int(), &a);
        let before = s.vars.clone();
        assert!(prove(&s, &int(), &a));
        assert_eq!(s.vars, before);
    }
    #[test]
    fn typed_macro_rigid_upper_evidence_is_read_only() {
        let mut s = solver();
        let a = s.fresh();
        s.constrain(&a, &int());
        assert!(prove(&s, &a, &int()));
        assert!(!prove(&s, &a, &string()));
    }
    #[test]
    fn typed_macro_rigid_transitive_frozen_edges() {
        let mut s = solver();
        let a = s.fresh();
        let b = s.fresh();
        let c = s.fresh();
        s.constrain(&a, &b);
        s.constrain(&b, &c);
        assert!(prove(&s, &a, &c));
        assert!(!prove(&s, &c, &a));
    }
    #[test]
    fn typed_macro_fresh_edges_do_not_change_frozen_reverse_links() {
        let mut s = solver();
        let a = s.fresh();
        s.freeze();
        let snapshot = s.vars.clone();
        let fresh = s.fresh();
        s.constrain(&a, &fresh);
        s.constrain(&fresh, &a);
        assert!(s.failures.is_empty());
        assert_eq!(&s.vars[..snapshot.len()], snapshot);
    }
    #[test]
    fn typed_macro_unguarded_cycles_are_not_proofs() {
        let mut s = solver();
        let a = s.fresh();
        let b = s.fresh();
        let (Ty::Var(a), Ty::Var(b)) = (&a, &b) else {
            unreachable!()
        };
        s.vars[a.0 as usize].upper.push(Ty::Var(*b));
        s.vars[b.0 as usize].upper.push(Ty::Var(*a));
        assert!(!prove(&s, &Ty::Var(*a), &int()));
    }
    #[test]
    fn typed_macro_difference_needs_disjointness() {
        let mut s = solver();
        let a = s.fresh();
        s.constrain(&int(), &a);
        let difference = Ty::Diff(Box::new(a.clone()), vec![int()]);
        assert!(!prove(&s, &a, &difference));
    }
    #[test]
    fn typed_macro_proof_cache_does_not_turn_failure_into_success() {
        let mut s = solver();
        let a = s.fresh();
        s.freeze();
        for _ in 0..3 {
            s.constrain(&a, &int());
            assert!(!s.failures.is_empty());
            s.failures.clear();
        }
        s.constrain(&a, &a);
        assert!(s.failures.is_empty());
    }
    #[test]
    fn typed_macro_proof_cache_replay_has_same_result() {
        let mut s = solver();
        let a = s.fresh();
        s.constrain(&a, &int());
        s.freeze();
        s.constrain(&a, &int());
        s.constrain(&a, &int());
        assert!(s.failures.is_empty());
        let mut replay = s.scheme_snapshot();
        replay.freeze();
        replay.constrain(&a, &int());
        assert!(replay.failures.is_empty());
        replay.constrain(&a, &string());
        assert!(!replay.failures.is_empty());
    }
    #[test]
    fn typed_macro_observation_keeps_shared_invariant_identity() {
        let mut s = solver();
        let a = s.fresh();
        s.constrain(&int(), &a);
        let Ty::Var(v) = a else { unreachable!() };
        let signature = FnTy {
            params: vec![(None, Ty::Var(v))],
            ret: Ty::Var(v),
            err: Ty::Never,
            env: Ty::Never,
        };
        let t = Ty::Fn(Box::new(signature.clone()));
        let sub = observe(&s, &t, &HashSet::new(), Some(&signature));
        assert_eq!(sub[&v], int());
        let fixed = HashSet::from([v]);
        assert!(observe(&s, &t, &fixed, Some(&signature)).is_empty());
        let mut ps = HashMap::new();
        let record = s.record(vec![(super::super::records::Label::name("x"), Ty::Var(v))]);
        occurrences(&s, &record, Pol::Pos, &mut ps);
        assert_eq!(ps[&v], Pol::Both);
    }
    #[test]
    fn typed_macro_graph_bundle_gate_preserves_sharing_and_obligations() {
        use super::super::scheme_wire::{SchemeArtifact, SchemeCatalog};
        let mut s = solver();
        let a = s.fresh();
        let Ty::Var(v) = a else { unreachable!() };
        s.constrain(&int(), &Ty::Var(v));
        s.vars[v.0 as usize].difference = true;
        s.vars[v.0 as usize].handled.push(string());
        let left = s.record(vec![
            (super::super::records::Label::name("x"), Ty::Var(v)),
            (super::super::records::Label::name("y"), Ty::Var(v)),
        ]);
        let right = s.record(vec![
            (super::super::records::Label::name("x"), string()),
            (super::super::records::Label::name("y"), int()),
        ]);
        let graph = s.records.union(&[left.clone(), right]);
        let scheme = Scheme::mono(Ty::tuple(vec![graph, Ty::Var(v)]));
        let artifact = SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(s.scheme_snapshot()),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        };
        let bundle = artifact.text_with(&mut |t| match t {
            TypeRef::Builtin(b) => b.name().into(),
            _ => unreachable!(),
        });
        assert!(bundle.contains("\"graph\""));
        assert!(artifact.needs_semantic_comparison());
        let restored = SchemeArtifact::from_bundle(&bundle, &mut |name| match name {
            "Int" => Some(TypeRef::Builtin(BuiltinType::Int)),
            "String" => Some(TypeRef::Builtin(BuiltinType::String)),
            _ => None,
        })
        .unwrap();
        let snapshot = restored.catalog.solver.lock().unwrap();
        assert!(
            snapshot
                .vars
                .iter()
                .any(|v| v.difference && !v.handled.is_empty())
        );
        let decoded = match &restored.scheme.ty {
            Ty::Graph(g) => g.view().expect("bundle tuple view"),
            t => t.clone(),
        };
        let Ty::Con(_, _, roots) = &decoded else {
            panic!("bundle tuple")
        };
        assert!(prove(&snapshot, &int(), &roots[1]));
        assert!(!prove(&snapshot, &string(), &roots[1]));
        drop(snapshot);
        let again = restored.text_with(&mut |t| match t {
            TypeRef::Builtin(b) => b.name().into(),
            _ => unreachable!(),
        });
        assert_eq!(bundle, again);
    }
    fn rows_query(signature: Ty) -> TypedMacroQuery {
        TypedMacroQuery {
            id: 1,
            parser: OnceLock::new(),
            state: Mutex::new(QueryState {
                solver: solver(),
                signature,
                expressions: HashMap::new(),
                inputs: Vec::new(),
                argument_order: Vec::new(),
                substitution: HashMap::new(),
                observed: HashSet::new(),
                arena: HandleArena::new(),
                answers: HashMap::new(),
                program: Arc::new(Program::default()),
                reads: super::super::value_types::Reads::default(),
                tyvars: HashMap::new(),
                module: ModuleId(0),
                fixed: HashSet::new(),
                effects: HashMap::new(),
            }),
        }
    }
    /// The type as a typed macro sees it, in the old and in the new rows.
    fn rows(t: &Ty) -> (String, String) {
        let query = rows_query(t.clone());
        let scope = Scope {
            module: 0,
            marks: Default::default(),
        };
        let mut s = query.state.lock().unwrap();
        let old = query.syntax_rows(&mut s, t, &scope, false).show();
        let new = query.syntax_rows(&mut s, t, &scope, true).show();
        (old, new)
    }
    fn function(params: Vec<(Option<&str>, Ty)>, ret: Ty, err: Ty, env: Ty) -> Ty {
        Ty::Fn(Box::new(FnTy {
            params: params
                .into_iter()
                .map(|(label, t)| (label.map(Into::into), t))
                .collect(),
            ret,
            err,
            env,
        }))
    }
    fn builtin(b: BuiltinType) -> Ty {
        Ty::named(TypeRef::Builtin(b), Vec::new())
    }
    #[test]
    fn typed_macro_function_type_rows_without_errors_or_needs() {
        let t = function(vec![], int(), Ty::Never, Ty::Never);
        assert_eq!(rows(&t), ("(-> () Int)".into(), "(-> (,) Int)".into()));
        let t = function(
            vec![(None, int()), (Some("x"), string())],
            int(),
            Ty::Never,
            Ty::Never,
        );
        assert_eq!(
            rows(&t),
            (
                "(-> (Int (: x String)) Int)".into(),
                "(-> (, Int (= x String)) Int)".into()
            )
        );
        // A function type as a parameter and as a result.
        let inner = function(vec![(None, int())], string(), Ty::Never, Ty::Never);
        let t = function(vec![(None, inner.clone())], inner, Ty::Never, Ty::Never);
        assert_eq!(
            rows(&t),
            (
                "(-> ((-> (Int) String)) (-> (Int) String))".into(),
                "(-> (, (-> (, Int) String)) (-> (, Int) String))".into()
            )
        );
    }
    #[test]
    fn typed_macro_function_type_rows_with_errors_and_needs() {
        let float = builtin(BuiltinType::Float);
        // Errors only.
        let t = function(vec![(None, int())], int(), string(), Ty::Never);
        assert_eq!(
            rows(&t),
            (
                "(-> (Int) (^ Int String))".into(),
                "(-> (, Int) (<- Int (, (Fail String))))".into()
            )
        );
        // One need.
        let t = function(vec![(None, int())], int(), Ty::Never, float.clone());
        assert_eq!(
            rows(&t),
            (
                "(-> (Int) (! Int Float))".into(),
                "(-> (, Int) (<- Int (, Float)))".into()
            )
        );
        // Several needs.
        let both = Ty::Union(vec![float.clone(), string()]);
        let t = function(vec![(None, int())], int(), Ty::Never, both.clone());
        assert_eq!(
            rows(&t),
            (
                "(-> (Int) (! Int (| Float String)))".into(),
                "(-> (, Int) (<- Int (, Float String)))".into()
            )
        );
        // Errors and needs: the `Fail` entry is first.
        let failure = Ty::Union(vec![int(), string()]);
        let t = function(vec![(Some("x"), int())], int(), failure, both);
        assert_eq!(
            rows(&t),
            (
                "(-> ((: x Int)) (! (^ Int (| Int String)) (| Float String)))".into(),
                "(-> (, (= x Int)) (<- Int (, (Fail (| Int String)) Float String)))".into()
            )
        );
    }
    #[test]
    fn typed_macro_function_type_rows_with_row_variables() {
        let mut solver = solver();
        let row = solver.fresh();
        let error = solver.fresh();
        let float = builtin(BuiltinType::Float);
        // Needs that are one row variable.
        let t = function(vec![(None, int())], int(), Ty::Never, row.clone());
        let (old, new) = rows(&t);
        assert_eq!(old, "(-> (Int) (! Int A0))");
        assert_eq!(new, "(-> (, Int) (<- Int A0))");
        // A row variable beside a service, and beside `Fail`.
        let env = Ty::Union(vec![row.clone(), float]);
        let t = function(vec![(None, int())], int(), Ty::Never, env.clone());
        assert_eq!(rows(&t).1, "(-> (, Int) (<- Int (, .. A0 Float)))");
        let t = function(vec![(None, int())], int(), string(), row.clone());
        assert_eq!(rows(&t).1, "(-> (, Int) (<- Int (, (Fail String) .. A0)))");
        // An error variable is the argument of `Fail`, not a row.
        let t = function(vec![(None, int())], int(), error, env);
        let (old, new) = rows(&t);
        assert_eq!(old, "(-> (Int) (! (^ Int A0) (| A1 Float)))");
        assert_eq!(new, "(-> (, Int) (<- Int (, (Fail A0) .. A1 Float)))");
    }
    /// The new rows are types of the Lisp surface, and the needs read back
    /// as the two rows they were made from: `Cx::needs_row` is the reverse.
    #[test]
    fn typed_macro_function_type_rows_are_read_back_by_the_lowering() {
        use ilang_syntax::ast::{TypeBody, TypeKind as Old};
        let float = builtin(BuiltinType::Float);
        let cases = [
            (Ty::Never, Ty::Never, None, None),
            (string(), Ty::Never, Some("String"), None),
            (Ty::Never, float.clone(), None, Some("Float")),
            (string(), float.clone(), Some("String"), Some("Float")),
            (
                Ty::Union(vec![int(), string()]),
                Ty::Union(vec![float, string()]),
                Some("(| Int String)"),
                Some("(| Float String)"),
            ),
        ];
        for (err, env, want_err, want_env) in cases {
            let t = function(vec![(None, int()), (Some("x"), string())], int(), err, env);
            let (_, new) = rows(&t);
            let source = format!("(= T {new})");
            let (module, errors) = ilang_syntax::syn3::parse_lisp_lowered(&source);
            assert!(errors.is_empty(), "{source}: {errors:?}");
            let ilang_syntax::ast::ItemKind::TypeDef(def) = &module.items[0].kind else {
                panic!("{source}: a type definition")
            };
            let TypeBody::Alias(alias) = &def.body else {
                panic!("{source}: a name")
            };
            let Old::Fn {
                params,
                ret,
                err,
                env,
            } = &alias.kind
            else {
                panic!("{source}: a function type")
            };
            assert_eq!(params.len(), 2, "{source}");
            assert!(matches!(&ret.kind, Old::Named { name, .. } if name.name == "Int"));
            let show = |t: &Option<Box<ilang_syntax::ast::TypeExpr>>| {
                t.as_ref().map(|t| match &t.kind {
                    Old::Named { name, .. } => name.name.clone(),
                    Old::Union(members) => {
                        let names: Vec<_> = members
                            .iter()
                            .map(|m| match &m.kind {
                                Old::Named { name, .. } => name.name.clone(),
                                _ => panic!("a named member"),
                            })
                            .collect();
                        format!("(| {})", names.join(" "))
                    }
                    _ => panic!("{source}: a named type or a union"),
                })
            };
            assert_eq!(show(err).as_deref(), want_err, "{source}");
            assert_eq!(show(env).as_deref(), want_env, "{source}");
        }
    }
    #[test]
    fn typed_macro_data_type_rows() {
        let pair = Ty::tuple(vec![int(), string()]);
        assert_eq!(rows(&pair), ("(, Int String)".into(), "(, Int String)".into()));
        let mut solver = solver();
        let record = solver.record(vec![
            (super::super::records::Label::name("x"), int()),
            (super::super::records::Label::name("y"), string()),
        ]);
        assert_eq!(
            rows(&record),
            (
                "((: \"x\" Int) (: \"y\" String))".into(),
                "(, (= x Int) (= y String))".into()
            )
        );
    }
    #[test]
    fn typed_macro_query_cache_uses_graph_bundle_and_fixation_state() {
        let mut solver = solver();
        let variable = solver.fresh();
        let Ty::Var(v) = variable else { unreachable!() };
        let graph = solver.record(vec![
            (super::super::records::Label::name("x"), variable.clone()),
            (super::super::records::Label::name("y"), variable.clone()),
        ]);
        let query = TypedMacroQuery {
            id: 1,
            parser: OnceLock::new(),
            state: Mutex::new(QueryState {
                solver,
                signature: graph,
                expressions: HashMap::new(),
                inputs: Vec::new(),
                argument_order: Vec::new(),
                substitution: HashMap::new(),
                observed: HashSet::from([v]),
                arena: {
                    let mut arena = HandleArena::new();
                    arena.intern(&variable);
                    arena
                },
                answers: HashMap::new(),
                program: Arc::new(Program::default()),
                reads: super::super::value_types::Reads::default(),
                tyvars: HashMap::new(),
                module: ModuleId(0),
                fixed: HashSet::from([v]),
                effects: HashMap::new(),
            }),
        };
        let wire: serde_json::Value = serde_json::from_str(&query.graph_bundle()).unwrap();
        assert_eq!(wire["fixed"], serde_json::json!([v.0]));
        assert!(wire["graph_bundle"]["graph"].is_string());
        let graph = super::super::SchemeArtifact::from_bundle(
            &wire["graph_bundle"].to_string(),
            &mut |_| None,
        )
        .unwrap();
        assert!(graph.needs_semantic_comparison());
        let before = query.graph_bundle();
        query.state.lock().unwrap().fixed.clear();
        assert_ne!(before, query.graph_bundle());
    }
}
