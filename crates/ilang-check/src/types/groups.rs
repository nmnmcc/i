//! Shape groups of functions (syntax three §2.5, §2.6).
//!
//! A function takes one data value: `FnTy::param` is its parameter data
//! type. A function of one shape with no labels has data of that shape
//! there. A **multi-shape** function type has a union of data types of
//! different shapes, or one data type whose shape has labels ([`with_data`]).

use super::solve::{FailKind, Solver};
use super::{FnTy, Head, ShapeId, Ty};

/// The label of the one parameter of a multi-shape function type in the
/// transition syntax tree (`syn3/lower.rs`): its type is the parameter data
/// type itself.
pub(crate) const DATA: &str = "%data";

/// The function type takes data of several shapes, or of a shape with
/// labels.
pub(crate) fn is_multi(f: &FnTy) -> bool {
    match &f.param {
        Ty::Con(Head::Shape(s), _, _) => s.has_labels(),
        Ty::Union(ms) => ms.iter().filter(|m| matches!(m, Ty::Con(Head::Shape(_), _, _))).count() > 1,
        _ => false,
    }
}

/// The function type whose parameter data type is `param`.
pub(crate) fn with_data(param: Ty, ret: Ty, err: Ty, env: Ty) -> FnTy {
    FnTy {
        param,
        labels: Vec::new(),
        ret,
        err,
        env,
    }
}

/// The parameter data type of one shape group: the group's function takes
/// the parts of `shape` in canonical order.
pub(crate) fn group_data(shape: ShapeId, parts: &[Ty]) -> Ty {
    assert_eq!(shape.len(), parts.len(), "a group function takes the parts of its shape");
    Ty::data(shape, parts.to_vec())
}

impl Solver {
    /// `f <: g` for two function types: before the parameter data of `g`
    /// goes to `f`, a multi-shape `f` where `g` has one shape is refused.
    pub(super) fn multi_to_one_shape(&mut self, f: &FnTy, g: &FnTy, l: &Ty, r: &Ty) {
        if is_multi(f) && !is_multi(g) && matches!(g.param, Ty::Con(..)) {
            // syn3-stage10: a multi-shape value is called, or goes where a
            // function of one shape is wanted. The run time cannot call it
            // yet; delete this failure when it can.
            self.fail(FailKind::MultiCall, l, r);
        }
    }
}

// ---- Selection of branches (syntax three §2.6.2 to §2.6.7) ----

use super::coverage::{Col, Ctor, Lit, Pat};
use super::infer::{Checker, Scheme};
use super::known;
use crate::code;
use crate::hir::*;
use ilang_syntax::span::Span;
use std::rc::Rc;

/// What a rule-11 `case` leaves for the selection table of the function
/// whose body it is: each clause's pattern row over the subject, whether it
/// has a guard, and the type of its body.
pub(crate) struct CaseTable {
    pub rows: Vec<(Pat, bool)>,
    pub rets: Vec<Ty>,
}

/// The selection table of one shape group with several branches or a guard
/// (§2.6.1), read from the group's written function.
pub(crate) struct BranchTable {
    /// The number of parts of the group's shape.
    arity: usize,
    /// The rows are over the one argument itself, not over the argument
    /// data.
    single: bool,
    rows: Vec<(Pat, bool)>,
    /// `(F, (R_1, …, R_n))`: the group's function type and the result type
    /// of each branch, generalized together above `level`, so that one
    /// instance gives consistent types for all branches.
    level: u32,
    combined: Ty,
}

/// The `case` that is the whole body of a group function over its own
/// parameters: its span, and whether its subject is the one parameter.
pub(crate) fn group_case(value: &Expr) -> Option<(Span, bool, usize)> {
    let ExprKind::Fn(f) = &value.kind else {
        return None;
    };
    let params: Vec<VarId> = f
        .params
        .iter()
        .filter_map(|p| match p {
            Param::Pattern { pattern, .. } => Some(match &pattern.kind {
                PatternKind::Var(b) => Some(b.var),
                _ => None,
            }),
            Param::Env { .. } => None,
        })
        .collect::<Option<_>>()?;
    let [Stmt::Expr(body)] = f.body.stmts.as_slice() else {
        return None;
    };
    let ExprKind::Case { subject, .. } = &body.kind else {
        return None;
    };
    let is = |e: &Expr, v: VarId| matches!(&e.kind, ExprKind::Name(ValueRef::Local(x)) if *x == v);
    match &subject.kind {
        ExprKind::Name(_) if params.len() == 1 && is(subject, params[0]) => Some((body.span, true, 1)),
        ExprKind::Tuple(items)
            if items.len() == params.len() && items.iter().zip(&params).all(|(e, v)| is(e, *v)) =>
        {
            Some((body.span, false, params.len()))
        }
        _ => None,
    }
}

/// The alternatives of a pattern.
fn alternatives<'p>(p: &'p Pat, out: &mut Vec<&'p Pat>) {
    match p {
        Pat::Or(ps) => ps.iter().for_each(|x| alternatives(x, out)),
        _ => out.push(p),
    }
}

/// The rows of a branch over the keys of its group: one pattern for each
/// key, for each choice of the branch.
fn key_rows(row: &Pat, single: bool, arity: usize) -> Vec<Vec<Pat>> {
    if single {
        return vec![vec![row.clone()]];
    }
    let mut choices = Vec::new();
    alternatives(row, &mut choices);
    choices
        .into_iter()
        .map(|c| match c {
            Pat::Ctor(Ctor::Shape(_), parts) if parts.len() == arity => parts.clone(),
            // A catch-all for the whole argument data.
            _ => vec![Pat::Wild(None); arity],
        })
        .collect()
}

/// The type constructor of a written pattern that is not a catch-all.
fn head_of(a: &Pat) -> Option<Head> {
    match a {
        Pat::Head(h, _) | Pat::Except(h, _) => Some(*h),
        Pat::Ctor(c, _) => Some(c.head()),
        Pat::Wild(_) | Pat::Or(_) => None,
    }
}

fn same_candidate(x: &Pat, y: &Pat) -> bool {
    match (x, y) {
        (Pat::Head(a, _), Pat::Head(b, _)) => a == b,
        (Pat::Ctor(a, _), Pat::Ctor(b, _)) => a == b,
        _ => false,
    }
}

/// The values of the written pattern `a` that are values of the candidate
/// `c`, as a pattern.
fn meet(a: &Pat, c: &Pat) -> Option<Pat> {
    if matches!(a, Pat::Wild(_)) {
        return Some(c.clone());
    }
    match c {
        Pat::Head(h, _) => (head_of(a) == Some(*h)).then(|| a.clone()),
        Pat::Ctor(lit @ Ctor::Lit(Lit::Int(v) | Lit::Str(v)), _) => match a {
            Pat::Ctor(other, _) => (other == lit).then(|| a.clone()),
            Pat::Head(h, _) => (*h == lit.head()).then(|| c.clone()),
            Pat::Except(h, set) => (*h == lit.head() && set.contains(v)).then(|| c.clone()),
            Pat::Wild(_) | Pat::Or(_) => None,
        },
        _ => unreachable!("a candidate is a head or a literal"),
    }
}

/// The values of the type constructor `head` that the written pattern `a`
/// has in its pattern type: every value for a constructor or a type test.
fn values_of(a: &Pat, head: Head) -> Option<super::lits::ValueSet> {
    if head_of(a) != Some(head) {
        return None;
    }
    Some(match a {
        Pat::Except(_, set) => set.clone(),
        Pat::Ctor(Ctor::Lit(Lit::Int(v) | Lit::Str(v)), _) => {
            super::lits::ValueSet::only([std::sync::Arc::from(v.as_str())]).unwrap()
        }
        _ => super::lits::ValueSet::ALL,
    })
}

impl Checker<'_> {
    /// Keeps the selection table of a binding `name = group function`,
    /// after its `case` was typed. `generalized` is the level above which
    /// the binding is generalized, or `None` when it is not.
    pub(super) fn keep_branches(
        &mut self,
        key: ValueRef,
        value: &Expr,
        fn_ty: &Ty,
        generalized: Option<u32>,
        span: Span,
    ) {
        let Some((case_span, single, arity)) = group_case(value) else {
            return;
        };
        let Some(case) = self.case_tables.remove(&(self.module, case_span)) else {
            return;
        };
        let together = Ty::tuple(vec![fn_ty.clone(), Ty::tuple(case.rets)]);
        let (level, combined) = match generalized {
            Some(level) => {
                // The binding's own generalization has reported what there
                // is to report.
                let mark = self.diags.len();
                let combined = self.generalize(&together, level, span);
                self.diags.truncate(mark);
                (level, combined)
            }
            None => (u32::MAX, together),
        };
        let table = Rc::new(BranchTable {
            arity,
            single,
            rows: case.rows,
            level,
            combined,
        });
        self.branch_tables.insert(key, table);
    }

    /// The selection table that a name has: its own, or that of the name it
    /// is a direct alias of (§2.6.7).
    fn branch_table(&self, e: &Expr) -> Option<Rc<BranchTable>> {
        let ExprKind::Name(mut r) = e.kind else {
            return None;
        };
        for _ in 0..64 {
            if let Some(t) = self.branch_tables.get(&r) {
                return Some(t.clone());
            }
            r = self.alias_of(r)?;
        }
        None
    }

    /// The name that `r` is bound to by `r = name`.
    fn alias_of(&self, r: ValueRef) -> Option<ValueRef> {
        match r {
            ValueRef::Local(v) => self.local_aliases.get(&v).copied(),
            ValueRef::Def(d) => {
                let DefKind::Binding { binding, .. } = self.p.def(d).kind else {
                    return None;
                };
                let b = &self.p.module(d.module).bindings[binding as usize].binding;
                match (&b.pattern.kind, &b.value.kind) {
                    (PatternKind::Var(_), ExprKind::Name(to @ (ValueRef::Def(_) | ValueRef::Local(_)))) => {
                        Some(*to)
                    }
                    _ => None,
                }
            }
            ValueRef::Ctor(_) => None,
        }
    }

    /// The shape groups of a top-level chain, through direct aliases.
    fn chain_groups(&self, mut d: DefId) -> Option<&Vec<(ShapeId, DefId)>> {
        for _ in 0..64 {
            if let Some(gs) = self.p.chain_groups.get(&d) {
                return Some(gs);
            }
            match self.alias_of(ValueRef::Def(d))? {
                ValueRef::Def(to) => d = to,
                _ => return None,
            }
        }
        None
    }

    /// The name has a table that an expected type can select from.
    pub(super) fn selects(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Name(ValueRef::Def(d)) if self.chain_groups(*d).is_some() => true,
            _ => self.branch_table(e).is_some(),
        }
    }

    /// §2.6.2 step 4: the branches that the known types of the argument
    /// parts do not skip. Branch `i` is skipped when at some key `k` the
    /// known type `K_k` and `W_i,k - T_i,k` are disjoint (spec §4.5 rule
    /// 3): `W_i,k` is the written pattern type of the branch at `k` and
    /// `T_i,k` the members taken there by the earlier branches without a
    /// guard. A part with no known type skips nothing; guarded branches are
    /// selected like the others and take nothing.
    fn select(&mut self, table: &BranchTable, known: &[Ty]) -> Vec<usize> {
        let mut earlier: Vec<Vec<Pat>> = Vec::new();
        let mut selected = Vec::new();
        for (i, (row, guarded)) in table.rows.iter().enumerate() {
            // A branch with alternatives is one row for each way to take
            // one alternative of each item.
            let choices = key_rows(row, table.single, table.arity);
            if choices.iter().any(|c| !self.skipped(c, &earlier, known)) {
                selected.push(i);
            }
            if !guarded {
                earlier.extend(choices);
            }
        }
        selected
    }

    /// The row `choice` (one pattern for each key) is skipped for these
    /// known types.
    fn skipped(&mut self, choice: &[Pat], earlier: &[Vec<Pat>], known: &[Ty]) -> bool {
        (0..choice.len()).any(|k| self.key_disjoint(choice, k, earlier, &known[k]))
    }

    /// `K_k` and `W_k - T_k` are disjoint at the key `k` of the row.
    fn key_disjoint(&mut self, choice: &[Pat], k: usize, earlier: &[Vec<Pat>], known: &Ty) -> bool {
        let members = known::members(known);
        if members.is_empty() || members.iter().any(|m| m.head().is_none()) {
            return false;
        }
        let mut own = Vec::new();
        alternatives(&choice[k], &mut own);
        let catch_all = own.iter().any(|a| matches!(a, Pat::Wild(_)));
        // The candidates (§2.6.2 "Taken"): the type constructors, the
        // literal values and the type-test members written here or at this
        // key in an earlier branch without a guard.
        let mut candidates: Vec<Pat> = Vec::new();
        let mut written: Vec<&Pat> = own.clone();
        for row in earlier {
            alternatives(&row[k], &mut written);
        }
        for a in written {
            if let Some(c) = self.candidate(a)
                && !candidates.iter().any(|x| same_candidate(x, &c))
            {
                candidates.push(c);
            }
        }
        // A candidate is taken when every value that the row matches with
        // a value of the candidate at `k` is matched by an earlier row.
        let cols = vec![Col::Any; choice.len()];
        let mut taken_heads: Vec<Head> = Vec::new();
        let mut taken_values: Vec<(Head, super::lits::ValueSet)> = Vec::new();
        for c in &candidates {
            let here: Vec<Pat> = own.iter().filter_map(|a| meet(a, c)).collect();
            if here.is_empty() {
                continue;
            }
            let mut q = choice.to_vec();
            q[k] = Pat::Or(here);
            if self.useful(earlier, &q, &cols).is_none() {
                match c {
                    Pat::Ctor(ctor @ Ctor::Lit(Lit::Int(v) | Lit::Str(v)), _) => taken_values.push((
                        ctor.head(),
                        super::lits::ValueSet::only([std::sync::Arc::from(v.as_str())]).unwrap(),
                    )),
                    Pat::Head(h, _) => taken_heads.push(*h),
                    _ => unreachable!("a candidate is a head or a literal"),
                }
            }
        }
        // Some member of the known type has values in `W - T`.
        let overlaps = members.iter().any(|m| {
            let head = m.head().unwrap();
            if taken_heads.contains(&head) {
                return false;
            }
            let mut left = if catch_all {
                Some(super::lits::ValueSet::ALL)
            } else {
                own.iter()
                    .filter_map(|a| values_of(a, head))
                    .reduce(|x, y| x.union(&y))
            };
            for (h, values) in &taken_values {
                if *h == head {
                    left = left.and_then(|l| l.minus(values));
                }
            }
            left.is_some_and(|l| !m.values().disjoint(&l))
        });
        !overlaps
    }

    /// The candidate that a written pattern gives: its literal value, or
    /// its type constructor.
    fn candidate(&self, a: &Pat) -> Option<Pat> {
        Some(match a {
            Pat::Wild(_) | Pat::Or(_) => return None,
            Pat::Head(h, n) => Pat::Head(*h, *n),
            Pat::Except(h, _) => Pat::Head(*h, 0),
            Pat::Ctor(Ctor::Lit(Lit::Int(_) | Lit::Str(_)), _) => a.clone(),
            Pat::Ctor(c, _) => Pat::Head(
                c.head(),
                match c {
                    Ctor::Custom(r) => self.p.ty(r.ty).params.len(),
                    Ctor::Shape(shape) => shape.len(),
                    Ctor::Nil | Ctor::Cons => 1,
                    Ctor::Lit(_) => 0,
                },
            ),
        })
    }

    /// `F_sel` (§2.6.2 step 6) over the selected branches: an instance of
    /// the group's function type whose result is the union of the results
    /// of those branches.
    fn selected_fn(&mut self, table: &BranchTable, selected: &[usize], span: Span) -> Option<FnTy> {
        let instance = self.instantiate(&Scheme {
            level: table.level,
            ty: table.combined.clone(),
        });
        let Ty::Con(_, _, parts) = instance else {
            return None;
        };
        let [Ty::Fn(f), Ty::Con(_, _, rets)] = parts.as_slice() else {
            return None;
        };
        if f.arity() != Some(table.arity) {
            return None;
        }
        let ret = self.union(selected.iter().map(|i| rets[*i].clone()).collect(), span);
        Some(FnTy {
            param: f.param.clone(),
            labels: f.labels.clone(),
            ret,
            err: f.err.clone(),
            env: f.env.clone(),
        })
    }

    /// §2.6.2: the type of the callee at a direct call `f(d)` of a name
    /// with a table, from the known types of the argument parts, in the
    /// order of the parameters. `None` when every branch is selected: the
    /// callee has the type of its whole group.
    pub(super) fn select_at_call(&mut self, callee: &Expr, known: &[Ty], span: Span) -> Option<Ty> {
        let table = self.branch_table(callee)?;
        if known.len() != table.arity {
            return None;
        }
        let selected = self.select(&table, known);
        if selected.len() == table.rows.len() {
            return None;
        }
        if selected.is_empty() {
            let shown: Vec<String> = known
                .iter()
                .map(|k| super::display::show_known(&self.s, k))
                .collect();
            self.error(
                code::NO_BRANCH_SELECTED,
                span,
                format!(
                    "no branch of this function can match these arguments: their known types are `({})` (syntax three §2.6.2)",
                    shown.join(", ")
                ),
            );
            return None;
        }
        self.selected_fn(&table, &selected, span).map(|f| Ty::Fn(Box::new(f)))
    }

    /// §2.6.4: a name with a table, used where one function type is
    /// expected. A chain with several shape groups is the group of the
    /// expected shape; in a group, the branches are selected with the
    /// expected parameter types as the known types.
    pub(super) fn select_by_expected(&mut self, e: &Expr, expected: &Ty) -> Option<(Ty, Ty)> {
        let ExprKind::Name(r) = &e.kind else {
            return None;
        };
        let want: Vec<Ty> = match known::members(expected).as_slice() {
            [Ty::Fn(want)] if !is_multi(want) => want.parts()?.to_vec(),
            _ => return None,
        };
        if let ValueRef::Def(d) = r
            && let Some(groups) = self.chain_groups(*d)
        {
            let shape = ShapeId::positional(want.len() as u32);
            let (_, group) = groups.iter().find(|(s, _)| *s == shape)?;
            let group = ValueRef::Def(*group);
            self.out.selected_names.insert((self.module, e.span), group);
            let name = Expr {
                kind: ExprKind::Name(group),
                span: e.span,
            };
            return Some(self.infer_k(&name, expected));
        }
        let table = self.branch_table(e)?;
        if want.len() != table.arity {
            return None;
        }
        let selected = self.select(&table, &want);
        if selected.is_empty() || selected.len() == table.rows.len() {
            return None;
        }
        let f = self.selected_fn(&table, &selected, e.span)?;
        Some((Ty::Fn(Box::new(f)), Ty::Top))
    }
}
