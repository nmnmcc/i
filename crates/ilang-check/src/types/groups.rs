//! Shape groups of functions (syntax three §2.5, §2.6).
//!
//! A function takes one data value. Until `FnTy` holds that data type as
//! its one parameter, this module is the only place that knows how a
//! parameter list stands for it:
//!
//! - an ordinary `FnTy` with `n` parameters has the parameter data type of
//!   shape "`n` positions" with the parameter types as its parts
//!   ([`param_data`]);
//! - a **multi-shape** `FnTy` has exactly one parameter, marked with the
//!   reserved label [`DATA`], whose type is the parameter data type itself:
//!   a union of data types of different shapes, or one data type whose
//!   shape has labels ([`multi`]).
//!
//! With one data parameter in `FnTy`, `param_data(f)` is `f.param` for
//! every function, the mark goes away, and the two cases of
//! [`Solver::constrain_fns`] become the one rule
//! `constrain(g.param, f.param)`.

use super::solve::{FailKind, Solver};
use super::{FnTy, ShapeId, Ty};

/// The label that marks the one parameter of a multi-shape function type.
/// It cannot be written in source.
pub(crate) const DATA: &str = "%data";

/// The function type takes data of several shapes, or of a shape with
/// labels.
pub(crate) fn is_multi(f: &FnTy) -> bool {
    matches!(f.params.as_slice(), [(Some(l), _)] if l == DATA)
}

/// The parameter data type of a function type.
pub(crate) fn param_data(f: &FnTy) -> Ty {
    if is_multi(f) {
        f.params[0].1.clone()
    } else {
        Ty::tuple(f.params.iter().map(|p| p.1.clone()).collect())
    }
}

/// The function type whose parameter data type is `param`.
pub(crate) fn with_data(param: Ty, ret: Ty, err: Ty, env: Ty) -> FnTy {
    FnTy {
        params: vec![(Some(DATA.to_string()), param)],
        ret,
        err,
        env,
    }
}

/// A function type in the multi-shape form.
pub(crate) fn as_multi(f: FnTy) -> FnTy {
    if is_multi(&f) {
        f
    } else {
        let param = param_data(&f);
        with_data(param, f.ret, f.err, f.env)
    }
}

/// The parameter data type of one shape group: the group's function takes
/// the parts of `shape` in canonical order.
pub(crate) fn group_data(shape: ShapeId, f: &FnTy) -> Ty {
    assert_eq!(
        shape.len(),
        f.params.len(),
        "a group function takes the parts of its shape"
    );
    Ty::data(shape, f.params.iter().map(|p| p.1.clone()).collect())
}

impl Solver {
    /// `f <: g` for two function types of which one at least is
    /// multi-shape: the parameter data of `g` goes to `f`. The results and
    /// the needs are constrained by the caller.
    pub(super) fn constrain_fn_data(&mut self, f: &FnTy, g: &FnTy, l: &Ty, r: &Ty) {
        if is_multi(f) && !is_multi(g) {
            // syn3-stage10: a multi-shape value is called, or goes where a
            // function of one shape is wanted. The run time cannot call it
            // yet; delete this failure when it can.
            self.fail(FailKind::MultiCall, l, r);
        }
        self.constrain(&param_data(g), &param_data(f));
    }
}
