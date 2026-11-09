//! Type inference (§4.7, §4.8) with effects (§9.2–§9.6): algebraic
//! subtyping in the style of Simple-sub (Parreaux, ICFP 2020; the theory is
//! MLsub, Dolan and Mycroft, POPL 2017), with the unions, intersections,
//! and differences of §4.4, the variance of §4.5, and the `? E ! N` parts of
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

mod coverage;
mod display;
mod infer;
mod known;
mod pattern;
mod solve;

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
    Tuple(u32),
    Fn,
}

/// A type in inference. `Top` is only made when a type is shown: it is a
/// type variable that has no effect, in an input place.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Ty {
    Var(V),
    /// A named type or a tuple, with its arguments or parts.
    Con(Head, Vec<Ty>),
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FnTy {
    /// Labels have no effect on subtyping (§4.6).
    pub params: Vec<(Option<String>, Ty)>,
    pub ret: Ty,
    pub err: Ty,
    pub env: Ty,
}

impl Ty {
    pub fn head(&self) -> Option<Head> {
        match self {
            Ty::Con(h, _) => Some(*h),
            Ty::Fn(_) => Some(Head::Fn),
            _ => None,
        }
    }

    pub fn named(t: TypeRef, args: Vec<Ty>) -> Ty {
        Ty::Con(Head::Named(t), args)
    }

    pub fn tuple(parts: Vec<Ty>) -> Ty {
        Ty::Con(Head::Tuple(parts.len() as u32), parts)
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
    /// `v.k`: part `k` of a tuple, whatever its size.
    Part(u32),
}

/// The result of type checking: the shown types, and what the code
/// generator needs. Places are keyed by the module and the span of the
/// node in the HIR, which is unique among the nodes of one kind.
#[derive(Clone, Debug, Default)]
pub struct Typed {
    /// The type scheme of each top-level name, simplified and shown in the
    /// syntax of §3 (§4.7).
    pub schemes: HashMap<DefId, String>,
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
    pub type_tests: HashMap<(ModuleId, Span), Vec<Head>>,
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
    /// The host services in the environment of each entry point (§10.2),
    /// which the host gives (§9.7).
    pub entry_services: HashMap<DefId, Vec<crate::hir::TypeId>>,
    /// The types of the local names, only from [`check_keeping_locals`]:
    /// for editors (`crate::query`).
    pub locals: Option<std::sync::Arc<LocalTypes>>,
}

/// The solver after inference, with the type and the known type (§4.8) of
/// every name bound by a pattern or a parameter, so that the type of a
/// local name can be shown when it is asked for.
pub struct LocalTypes {
    /// In a lock, because the names of the types are set for the module
    /// where a type is shown.
    s: std::sync::Mutex<solve::Solver>,
    types: HashMap<crate::hir::VarId, Ty>,
    known: HashMap<crate::hir::VarId, Ty>,
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
        let t = self.types.get(&v)?;
        let mut s = self.s.lock().unwrap_or_else(|e| e.into_inner());
        s.type_names = infer::type_names(program, module);
        let side = |out: bool| {
            let (a, b) = if out {
                (t.clone(), Ty::Top)
            } else {
                (Ty::Top, t.clone())
            };
            let both = Ty::Both(Box::new(a), Box::new(b));
            display::scheme(&s, &both, display::Opts::default()).ok()
        };
        match side(true) {
            Some(s) if s != "Never" => Some(s),
            _ => side(false),
        }
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
            Err(()) => Vec::new(),
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
    /// `module/path.name : type` for each top-level name of the module, in
    /// source order.
    pub fn lines(&self, program: &Program, module: ModuleId) -> Vec<String> {
        let m = program.module(module);
        (0..m.defs.len() as u32)
            .filter_map(|index| {
                let d = DefId { module, index };
                let t = self.schemes.get(&d)?;
                Some(format!("{}.{} : {t}", m.path, m.defs[index as usize].name))
            })
            .collect()
    }
}

/// Infers the type of every binding of the program, and reports the type
/// errors.
pub fn check(program: &Program, sources: &Sources) -> (Typed, Vec<Diagnostic>) {
    infer::Checker::new(program, sources).run()
}

/// `check`, keeping the types of the local names in `Typed::locals`.
pub fn check_keeping_locals(program: &Program, sources: &Sources) -> (Typed, Vec<Diagnostic>) {
    let mut c = infer::Checker::new(program, sources);
    c.keep_locals = true;
    c.run()
}
