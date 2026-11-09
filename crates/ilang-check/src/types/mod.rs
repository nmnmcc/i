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
//! - `pattern.rs`: patterns, fitting, and `case` by places (rule 11).
//! - `display.rs`: coalescing, simplification, and the text of a type in
//!   the syntax of §3.
//!
//! TODO(M3b): narrowing (rule 12), the exact "may fail" of §7.3, coverage
//! (§7.4), and known constructors (§7.5).
//! TODO(M3d): known types and expected types (§4.8), fields and tuple parts
//! (rule 13) on values with no written type, and the disjointness check of
//! `==` (§6.9).

mod display;
mod infer;
mod pattern;
mod solve;

use crate::hir::{DefId, ModuleId, Program, TypeRef};
use ilang_package::{Diagnostic, Sources};
use std::collections::HashMap;

/// A type variable of the solver.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct V(pub u32);

/// The type constructor of a member (§4.4). All function types are one
/// head, and so are the tuples of one size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Head {
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

/// The result of type checking.
#[derive(Clone, Debug, Default)]
pub struct Typed {
    /// The type scheme of each top-level name, simplified and shown in the
    /// syntax of §3 (§4.7).
    pub schemes: HashMap<DefId, String>,
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
