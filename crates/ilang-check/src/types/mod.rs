//! Type inference (§4.7, §4.8) with effects (§9.2–§9.6): algebraic
//! subtyping in the style of Simple-sub (Parreaux, ICFP 2020; the theory is
//! MLsub, Dolan and Mycroft, POPL 2017), with the unions, intersections,
//! and differences of §4.4, the variance of §4.5, and the `^ E ! N` parts of
//! function types.
//!
//! - `solve.rs`: type variables with levels and bounds, `constrain`, and the
//!   smart constructors that merge unions by type constructor (§4.4).
//! - `infer.rs`: the typing rules for expressions, blocks, functions, and
//!   bindings (§4.8 rules 1–10 and 14).
//! - `pattern.rs`: patterns, fitting, `case` by places (rule 11),
//!   narrowing (rule 12), and known constructors (§7.5).
//! - `coverage.rs`: Maranget's usefulness algorithm, for "can not fail"
//!   (§7.3), "taken" (rule 12), coverage and "never used" (§7.4).
//! - `known.rs`: known types and expected types (§4.8), found from what
//!   is written, in the order of inference.
//! - `display.rs`: coalescing, simplification, and the text of a type in
//!   the syntax of §3.
//! - `shown.rs`: a shown type as a tree ([`ShownType`]), and its text.

mod alias;
mod codec;
mod coverage;
mod display;
mod infer;
mod known;
mod lits;
mod pattern;
mod prove;
mod provider;
mod scheme_wire;
mod shape;
mod shown;
mod solve;
mod value_types;

pub use lits::{LitValue, TestValues};
pub use shape::ShapeId;
pub use scheme_wire::{
    AliasArtifact, SchemeArtifact, SchemeComparison, SchemeEnvironment, compare_scheme_group,
};
pub use shown::{ShownFn, ShownType};

use crate::hir::{BindingRef, CtorRef, DefId, ModuleId, Program, TypeRef};
use ilang_package::{Diagnostic, Sources};
use ilang_syntax::span::Span;
use std::collections::HashMap;

/// A type variable of the solver.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct V(pub u32);

/// The type constructor of a member (§4.4). All function types are one
/// head, and so are the tuples of one size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Head {
    Named(TypeRef),
    /// Data of one shape: tuples, records and `()` (syntax-3 §6.2).
    Shape(ShapeId),
    Fn,
}

/// A runtime type test, including the values of an Int or String member.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestMember {
    pub head: Head,
    pub values: TestValues,
}

/// A type in inference. `Top` is only made when a type is shown: it is a
/// type variable that has no effect, in an input place.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Ty {
    Var(V),
    /// A named type or a tuple, with its arguments or parts.
    Con(Head, lits::ValueSet, Vec<Ty>),
    Fn(Box<FnTy>),
    Union(Vec<Ty>),
    Inter(Vec<Ty>),
    /// `A - H`: the members of `H` are not type variables (§4.4).
    Diff(Box<Ty>, Vec<Ty>),
    Never,
    Top,
    /// Only in a shown type: an argument that does not change, coalesced
    /// as an output and as an input.
    Both(Box<Ty>, Box<Ty>),
}

// The baseline Ty is 40 bytes on 64-bit targets. The set adds at most 8.
#[cfg(target_pointer_width = "64")]
const _: () = assert!(size_of::<Ty>() <= 48);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FnTy {
    /// Labels have no effect on subtyping (§4.6).
    pub params: Vec<(Option<String>, Ty)>,
    pub ret: Ty,
    pub err: Ty,
    pub env: Ty,
}

impl Ty {
    pub fn literal(v: &LitValue) -> Ty {
        let (b, text) = match v {
            LitValue::Int(n) => (crate::hir::BuiltinType::Int, n.to_string()),
            LitValue::Str(s) => (crate::hir::BuiltinType::String, s.clone()),
        };
        Ty::lit(b, lits::ValueSet::only([text.into()]).unwrap())
    }

    pub fn lit(b: crate::hir::BuiltinType, values: lits::ValueSet) -> Ty {
        debug_assert!(matches!(
            b,
            crate::hir::BuiltinType::Int | crate::hir::BuiltinType::String
        ));
        Ty::Con(Head::Named(TypeRef::Builtin(b)), values, Vec::new())
    }

    pub fn values(&self) -> &lits::ValueSet {
        match self {
            Ty::Con(_, values, _) => values,
            _ => &lits::ValueSet::ALL,
        }
    }

    /// Only these argumentless heads have a scalar value-set algebra.
    pub fn scalar(&self) -> Option<crate::hir::BuiltinType> {
        match self {
            Ty::Con(Head::Named(TypeRef::Builtin(b)), _, args)
                if args.is_empty()
                    && matches!(
                        b,
                        crate::hir::BuiltinType::Int | crate::hir::BuiltinType::String
                    ) =>
            {
                Some(*b)
            }
            _ => None,
        }
    }

    /// Replace a scalar's set, keeping empty distinct from All (§4.2).
    pub fn with_values(&self, values: Option<lits::ValueSet>) -> Ty {
        let b = self.scalar().expect("only Int and String have value sets");
        values.map_or(Ty::Never, |values| Ty::lit(b, values))
    }
    pub fn head(&self) -> Option<Head> {
        match self {
            Ty::Con(h, _, _) => Some(*h),
            Ty::Fn(_) => Some(Head::Fn),
            _ => None,
        }
    }

    pub fn named(t: TypeRef, args: Vec<Ty>) -> Ty {
        Ty::Con(Head::Named(t), lits::ValueSet::ALL, args)
    }

    pub fn tuple(parts: Vec<Ty>) -> Ty {
        Ty::data(ShapeId::positional(parts.len() as u32), parts)
    }

    /// Data of a shape, with its parts in canonical order.
    pub fn data(shape: ShapeId, parts: Vec<Ty>) -> Ty {
        debug_assert_eq!(shape.len(), parts.len());
        Ty::Con(Head::Shape(shape), lits::ValueSet::ALL, parts)
    }

    /// Data with these labelled parts, in any order.
    /// A repeated label has been reported; its first part is kept.
    pub fn record(mut fields: Vec<(String, Ty)>) -> Ty {
        fields.sort_by(|a, b| a.0.cmp(&b.0));
        fields.dedup_by(|a, b| a.0 == b.0);
        let shape = ShapeId::new(0, fields.iter().map(|(l, _)| l.clone()));
        Ty::data(shape, fields.into_iter().map(|(_, t)| t).collect())
    }

    /// A type variable, or a difference of one: a member that a value of
    /// any type constructor may go to.
    pub fn is_varlike(&self) -> bool {
        match self {
            Ty::Var(_) => true,
            Ty::Diff(a, _) => a.is_varlike(),
            _ => false,
        }
    }

    pub fn value_head(&self) -> Option<Head> {
        self.head()
    }

    pub fn is_headed(&self) -> bool {
        self.head().is_some()
    }
}

/// Normalize a closed written host type with the same algebra as inference.
/// Structural members may disappear during scalar intersection or difference;
/// their presence in the source must not decide scalar ABI admissibility.
pub(crate) fn boundary_type(
    t: &crate::hir::Type,
    p: &Program,
    expand: &impl Fn(&crate::hir::Type) -> crate::hir::Type,
) -> Option<Ty> {
    fn convert(
        s: &mut solve::Solver,
        t: &crate::hir::Type,
        p: &Program,
        expand: &impl Fn(&crate::hir::Type) -> crate::hir::Type,
    ) -> Option<Ty> {
        use crate::hir::{BuiltinType, TypeKind};
        let t = expand(t);
        Some(match &t.kind {
            TypeKind::Lit(v) => Ty::literal(v),
            TypeKind::Named { ty, args } => {
                if *ty == TypeRef::Builtin(BuiltinType::Never) {
                    Ty::Never
                } else {
                    if matches!(ty, TypeRef::Custom(_)) && !args.is_empty() && s.variance.is_empty()
                    {
                        s.variance = infer::variances(p);
                    }
                    Ty::named(
                        *ty,
                        args.iter()
                            .map(|t| convert(s, t, p, expand))
                            .collect::<Option<_>>()?,
                    )
                }
            }
            TypeKind::Tuple(ts) => Ty::tuple(
                ts.iter()
                    .map(|t| convert(s, t, p, expand))
                    .collect::<Option<_>>()?,
            ),
            TypeKind::Record { fields } => Ty::record(
                fields
                    .iter()
                    .map(|(name, ty)| Some((name.name.clone(), convert(s, ty, p, expand)?)))
                    .collect::<Option<_>>()?,
            ),
            TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => Ty::Fn(Box::new(FnTy {
                params: params
                    .iter()
                    .map(|part| {
                        Some((
                            part.label.as_ref().map(|l| l.name.clone()),
                            convert(s, &part.ty, p, expand)?,
                        ))
                    })
                    .collect::<Option<_>>()?,
                ret: convert(s, ret, p, expand)?,
                err: match err {
                    Some(t) => convert(s, t, p, expand)?,
                    None => Ty::Never,
                },
                env: match env {
                    Some(t) => convert(s, t, p, expand)?,
                    None => Ty::Never,
                },
            })),
            TypeKind::Union(ts) | TypeKind::Inter(ts) => {
                let ms = ts
                    .iter()
                    .map(|t| convert(s, t, p, expand))
                    .collect::<Option<Vec<_>>>()?;
                if matches!(t.kind, TypeKind::Union(_)) {
                    s.union(ms)
                } else {
                    s.inter(ms)
                }
            }
            TypeKind::Diff(ts) => {
                let mut parts = ts.iter();
                let mut a = convert(s, parts.next()?, p, expand)?;
                for part in parts {
                    let h = convert(s, part, p, expand)?;
                    a = s.diff(a, vec![h]);
                }
                a
            }
            TypeKind::Of { .. } | TypeKind::Var(_) | TypeKind::Hole | TypeKind::Error => {
                return None;
            }
        })
    }
    let mut s = solve::Solver::new(HashMap::new(), Vec::new());
    let t = convert(&mut s, t, p, expand)?;
    s.failures.is_empty().then_some(t)
}

#[cfg(test)]
mod literal_representation_tests {
    use super::{
        Head, Ty,
        lits::ValueSet,
        solve::{Pol, Solver},
    };
    use crate::hir::{BuiltinType, TypeRef};
    use std::sync::Arc;

    #[test]
    fn scalar_sets_survive_child_mapping() {
        // literal-types §4: "No helper rebuilds a Con with a default set."
        let set = ValueSet::except([Arc::from("a")]);
        let literal = Ty::lit(BuiltinType::String, set.clone());
        let mut solver = Solver::new(Default::default(), Vec::new());
        assert_eq!(literal.values(), &set);
        let mapped = solver.map_children(&literal, Pol::Pos, &mut |_, _, _| {
            panic!("a literal has no children")
        });
        assert_eq!(mapped, literal);
        let list = Ty::named(TypeRef::Builtin(BuiltinType::List), vec![literal.clone()]);
        assert_eq!(
            solver.map_children(&list, Pol::Pos, &mut |_, t, _| t.clone()),
            list
        );
        assert_eq!(Ty::tuple(vec![literal]).values(), &ValueSet::ALL);
        assert_eq!(solver.fresh().values(), &ValueSet::ALL);
        assert_eq!(
            mapped.head(),
            Some(Head::Named(TypeRef::Builtin(BuiltinType::String)))
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn only_int_and_string_carry_sets() {
        // literal-types decision 3: Float has no literal types.
        Ty::lit(
            BuiltinType::Float,
            ValueSet::only([Arc::from("1")]).unwrap(),
        );
    }
}

/// How a type parameter changes with its type (§4.5 rules 6 and 7).
/// `Bi` is a parameter that is not used, only while variances are found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Variance {
    Bi,
    Co,
    Contra,
    Inv,
}

impl Variance {
    pub fn join(self, o: Variance) -> Variance {
        match (self, o) {
            (Variance::Bi, x) | (x, Variance::Bi) => x,
            (a, b) if a == b => a,
            _ => Variance::Inv,
        }
    }

    pub fn flip(self) -> Variance {
        match self {
            Variance::Co => Variance::Contra,
            Variance::Contra => Variance::Co,
            x => x,
        }
    }
}

/// How `v.f` or `v.k` reads a part of its value (§4.8 rule 13).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Access {
    /// `v.f`: each constructor that may have made `v` (§6.7), with the
    /// index of its field with the label `f`. At run time, the constructor
    /// of the value picks one.
    Field(Vec<(CtorRef, u32)>),
    /// `v.k` or `v.f` on data: the part at index `k` in canonical order,
    /// whatever the shape (syntax-3 §2.3).
    Part(u32),
}

/// Conservative, post-inference representation facts. Missing entries mean
/// Value; an unconstrained or recursive solver variable never licenses a cast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericFact {
    FiniteF64,
    SmallInt { min: i32, max: i32 },
}

/// The result of type checking: the shown types, and what the code
/// generator needs. Places are keyed by the module and the span of the
/// node in the HIR, which is unique among the nodes of one kind.
#[derive(Clone, Debug, Default)]
pub struct Typed {
    /// Numeric expression facts, frozen after completion and deferred checks.
    pub numeric: HashMap<(ModuleId, Span), NumericFact>,
    /// Parameter-pattern facts from the same completed inference lifecycle.
    pub numeric_params: HashMap<(ModuleId, Span), NumericFact>,
    /// Spans of empty record expressions reached by type inference.
    pub empty_records: std::collections::HashSet<(ModuleId, Span)>,
    /// Each data expression `(f: e, …)` or `(..b, f: e, …)` that type checks,
    /// by its span: its shape, and the shape of its spread base.
    pub data_shapes: HashMap<(ModuleId, Span), (ShapeId, Option<ShapeId>)>,
    /// Diagnostics retained when coalescing could not produce a finite
    /// type. IDEs may use the partial result; lowering must reject it.
    pub(crate) coalescing_errors: Vec<Diagnostic>,
    /// Missing schemes are allowed only in a diagnosed partial result.
    pub(crate) diagnosed: bool,
    /// Full completed value-dependent alias bodies, with written formal names.
    pub alias_types: HashMap<crate::hir::TypeId, ShownType>,
    /// Exact alias graphs for semantic interface comparison.
    pub alias_artifacts: HashMap<crate::hir::TypeId, AliasArtifact>,
    /// The type scheme of each top-level name, simplified and shown in the
    /// syntax of §3 (§4.7).
    pub schemes: HashMap<DefId, String>,
    /// Human-facing text; full scheme trees and interfaces stay exact.
    pub scheme_hovers: HashMap<DefId, String>,
    /// The type scheme of each exported top-level name and of each
    /// declaration (§10.1), whether exported or not: the type of
    /// `schemes`, simplified and named in the same way, as a tree whose
    /// named types are known by their `TypeRef`, not by the names that
    /// a module gives them. `schemes[d]` is
    /// `scheme_types[d].text(&type_names(program, d.module))`.
    pub scheme_types: HashMap<DefId, ShownType>,
    /// Inferred schemes for all top-level names, including private bindings
    /// that may be named by their own package's provider plan.
    pub provider_scheme_types: HashMap<DefId, ShownType>,
    /// Complete schemes for plan-only private factories and adapters.
    pub provider_scheme_artifacts: HashMap<DefId, SchemeArtifact>,
    /// Full graph schemes, including their retained bounds and obligations.
    pub scheme_artifacts: HashMap<DefId, SchemeArtifact>,
    /// §7.5: each use of a name that is known to be made by a constructor,
    /// by the span of the `Name` expression.
    pub known_ctors: HashMap<(ModuleId, Span), CtorRef>,
    /// Each `Field` and `TupleIndex` expression that type checks, by its
    /// span: what it reads.
    pub access: HashMap<(ModuleId, Span), Access>,
    /// Each call whose arguments could be put in their places, by the span
    /// of its `Call` expression, or
    /// of the `Binary` `|>` expression for `a |> f(…)` and `a |> f`: for
    /// each parameter in order, the index of the argument that goes to it
    /// (§6.6). Arguments count in the order they are written, with the
    /// left side of `|>` first; they are worked out in that order (§6.2).
    pub args: HashMap<(ModuleId, Span), Vec<u32>>,
    /// Each type test `x: T` or `_: T` in a `case` clause pattern, by the
    /// span of the `Typed` pattern: the type constructors of the members of
    /// `T`, which are tested at run time (§7.1).
    pub type_tests: HashMap<(ModuleId, Span), Vec<TestMember>>,
    /// The type constructor of the service of each environment parameter,
    /// by the span of its name, and of each environment statement, by the
    /// span of the statement (§9.2). Aliases are expanded.
    pub services: HashMap<(ModuleId, Span), TypeRef>,
    /// Whether each top-level binding was generalized (§4.7): its value is
    /// a direct value, as are the values of its group.
    pub generalized: HashMap<BindingRef, bool>,
    /// Each constructor pattern `C(p1, …)`, by the span of the pattern: for
    /// each part, in the order written, the index of the field it is
    /// matched to (§7.1). A part that matches no field has `u32::MAX`.
    pub pattern_fields: HashMap<(ModuleId, Span), Vec<u32>>,
    /// Each pattern of a binding and each parameter pattern, by the span of
    /// the pattern: whether it may fail for the type of its value (§7.3),
    /// so that it reports to `Assert` (§6.10, §6.11).
    pub may_fail: HashMap<(ModuleId, Span), bool>,
    /// The signature of each host export (§10.2, B.2) whose types the host
    /// can take.
    pub host_exports: HashMap<DefId, HostSignature>,
    /// Service constructors in each entry's inferred environment (§10.2),
    /// used to bind the chosen body instances during lowering.
    pub entry_services: HashMap<DefId, Vec<crate::hir::TypeId>>,
    /// The complete inferred environment of each entry, including ordinary
    /// services and their type arguments. Provider planning must use this
    /// rather than the constructor-only list used to bind entry slots.
    pub entry_provider_needs: HashMap<DefId, ShownType>,
    /// The types of the local names, only from [`check_keeping_locals`]:
    /// for editors (`crate::query`).
    pub locals: Option<std::sync::Arc<LocalTypes>>,
}

/// The solver after inference, with the type and the known type (§4.8) of
/// every name bound by a pattern or a parameter, so that the type of a
/// local name can be shown when it is asked for.
pub struct LocalTypes {
    diagnosed: bool,
    /// In a lock, because the names of the types are set for the module
    /// where a type is shown.
    s: std::sync::Mutex<solve::Solver>,
    types: HashMap<crate::hir::VarId, Ty>,
    known: HashMap<crate::hir::VarId, Ty>,
    /// The constructors that may have made `v` in each `v.f`, by the span
    /// of `v.f`.
    fields: HashMap<(ModuleId, Span), Vec<CtorRef>>,
}

impl std::fmt::Debug for LocalTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LocalTypes({} names)", self.types.len())
    }
}

impl LocalTypes {
    /// The type of a name, simplified and shown in the syntax of §3: the
    /// types of the values that come to it (its output side). When no value
    /// is known to come to it, as for a parameter, what its values must be
    /// for its uses (its input side).
    pub fn show(
        &self,
        program: &Program,
        module: ModuleId,
        v: crate::hir::VarId,
    ) -> Option<String> {
        self.show_in(program, module, v, &[])
    }

    /// [`LocalTypes::show`], in the context of the names of the top-level
    /// binding that the name is in: the type variables that the name
    /// shares with them, as a parameter `f` of `(f, x) { f(x) }` shares the
    /// type of `x`, keep their names, as in the type scheme of the binding.
    pub fn show_in(
        &self,
        program: &Program,
        module: ModuleId,
        v: crate::hir::VarId,
        context: &[crate::hir::VarId],
    ) -> Option<String> {
        let t = self.types.get(&v)?;
        let context: Vec<Ty> = context
            .iter()
            .filter(|c| **c != v)
            .filter_map(|c| self.types.get(c).cloned())
            .collect();
        let mut s = self.s.lock().unwrap_or_else(|e| e.into_inner());
        s.type_names = infer::type_names(program, module);
        let side = |out: bool, context: &[Ty]| {
            let (a, b) = if out {
                (t.clone(), Ty::Top)
            } else {
                (Ty::Top, t.clone())
            };
            let both = Ty::Both(Box::new(a), Box::new(b));
            match display::scheme_in(&s, &both, context, display::Opts::default()) {
                Ok(t) => Some(t),
                Err(()) => {
                    assert!(self.diagnosed, "accepted local types must coalesce");
                    None
                }
            }
        };
        // The side is chosen from the name alone: in the context, a
        // parameter is also an input of the binding, so it would not be
        // `Never` as an output.
        let out = side(true, &[]);
        let out_side = matches!(&out, Some(s) if s != "Never");
        if context.is_empty() {
            return if out_side { out } else { side(false, &[]) };
        }
        side(out_side, &context).or_else(|| side(out_side, &[]))
    }

    /// The constructors that may have made the value before the dot of the
    /// field access `v.f` with this span, from its known type (§4.8, §6.7),
    /// whatever `v` is: the fields that may be written after the dot.
    pub fn field_ctors(&self, module: ModuleId, span: Span) -> Option<&[CtorRef]> {
        self.fields.get(&(module, span)).map(Vec::as_slice)
    }

    /// The named types that are members of the known type of a name (§4.8),
    /// or else of its inferred type: where its fields come from (§6.7).
    pub fn named_types(&self, v: crate::hir::VarId) -> Vec<TypeRef> {
        let heads = |t: &Ty| -> Vec<TypeRef> {
            known::members(t)
                .iter()
                .filter_map(|m| match m.head() {
                    Some(Head::Named(r)) => Some(r),
                    _ => None,
                })
                .collect()
        };
        if let Some(k) = self.known.get(&v) {
            let found = heads(&display::both(k));
            if !found.is_empty() {
                return found;
            }
        }
        let Some(t) = self.types.get(&v) else {
            return Vec::new();
        };
        let both = Ty::Both(Box::new(t.clone()), Box::new(t.clone()));
        let s = self.s.lock().unwrap_or_else(|e| e.into_inner());
        match display::shown(&s, &both, display::Opts::default()) {
            Ok(t) => heads(&display::both(&t)),
            Err(()) => {
                assert!(self.diagnosed, "accepted local types must coalesce");
                Vec::new()
            }
        }
    }
}

/// A type that the host can take (§10.3), as one part of a host export.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HostType {
    Nil,
    Bool,
    Int,
    Float,
    String,
    BitArray,
    /// A custom type with no constructors: an opaque reference.
    Opaque(crate::hir::TypeId),
}

/// The types of a host export (B.2): its parameters, its value, and the
/// members of its errors in the canonical order (`Nil`, `Bool`, `Int`,
/// `Float`, `String`, `BitArray`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostSignature {
    pub params: Vec<HostType>,
    pub result: HostType,
    pub errors: Vec<HostType>,
}

impl Typed {
    /// Errors that prevent lowering an incomplete coalesced type result.
    pub fn coalescing_errors(&self) -> &[Diagnostic] {
        &self.coalescing_errors
    }

    /// `module/path.name : type` for each top-level name of the module, in
    /// source order.
    pub fn lines(&self, program: &Program, module: ModuleId) -> Vec<String> {
        let m = program.module(module);
        (0..m.defs.len() as u32)
            .filter_map(|index| {
                let d = DefId { module, index };
                let Some(t) = self.schemes.get(&d) else {
                    assert!(
                        self.diagnosed,
                        "accepted definitions must have type schemes"
                    );
                    return None;
                };
                Some(format!("{}.{} : {t}", m.path, m.defs[index as usize].name))
            })
            .collect()
    }
}

/// The name of each custom type, by `TypeId`, as the types of the names of
/// the module `module` are shown in [`Typed::schemes`]: the name alone for
/// a type of the module, of the prelude, or put in by an import (§2.3);
/// `alias.Name` for a type of an imported module; else `module/path.Name`.
pub fn type_names(program: &Program, module: ModuleId) -> Vec<String> {
    infer::type_names(program, module)
}

/// Infers the type of every binding of the program, and reports the type
/// errors.
pub fn check(program: &Program, sources: &Sources) -> (Typed, Vec<Diagnostic>) {
    infer::Checker::new(program, sources).run()
}

pub fn check_selected(
    program: &Program,
    sources: &Sources,
    selected: &[(DefId, bool)],
) -> (Typed, Vec<Diagnostic>) {
    infer::Checker::new(program, sources).run_selected(selected)
}

/// `check`, keeping the types of the local names in `Typed::locals`.
pub fn check_keeping_locals(program: &Program, sources: &Sources) -> (Typed, Vec<Diagnostic>) {
    check_keeping_locals_with_errors(program, sources, false)
}

/// Query recovery is explicit; an empty inference diagnostic list is not readiness.
pub fn check_keeping_locals_with_errors(
    program: &Program,
    sources: &Sources,
    prior_errors: bool,
) -> (Typed, Vec<Diagnostic>) {
    let mut c = infer::Checker::new(program, sources);
    c.keep_locals = true;
    c.recovery = prior_errors;
    c.run()
}
