//! Outward binary64 intervals, computed with exact integer/rational arithmetic.
//! Units are 2^-1074, so even subnormal inputs have exact integer endpoints.
//! No host floating-point arithmetic is used to establish a bound.
use super::rewrite;
use crate::ir::*;
use num_bigint::{BigInt, Sign};
use std::collections::BTreeMap;

fn scale() -> BigInt {
    BigInt::from(1) << 1074
}
fn max() -> BigInt {
    ((BigInt::from(1) << 53) - 1) << 2045
}
fn units(x: f64) -> BigInt {
    assert!(x.is_finite(), "source Float constant is finite");
    let bits = x.to_bits();
    let exp = (bits >> 52) & 2047;
    let frac = bits & ((1 << 52) - 1);
    let value = if exp == 0 {
        BigInt::from(frac)
    } else {
        BigInt::from(frac | (1 << 52)) << (exp - 1) as usize
    };
    if bits >> 63 != 0 { -value } else { value }
}
fn floor_ratio(n: &BigInt, d: &BigInt) -> BigInt {
    assert!(d > &BigInt::from(0), "positive interval denominator");
    let q = n / d;
    if n.sign() == Sign::Minus && n % d != BigInt::from(0) {
        q - 1
    } else {
        q
    }
}
fn ceil_ratio(n: &BigInt, d: &BigInt) -> BigInt {
    -floor_ratio(&-n, d)
}
/// Adjacent representable binary64 endpoints enclosing an integer-unit bound.
fn outward(n: BigInt, up: bool) -> BigInt {
    if n.sign() == Sign::Minus {
        return -outward(-n, !up);
    }
    let bits = n.bits();
    if bits <= 53 {
        return n;
    }
    let shift = (bits - 53) as usize;
    let floor = (&n >> shift) << shift;
    if up && floor != n {
        floor + (BigInt::from(1) << shift)
    } else {
        floor
    }
}
#[derive(Clone, Debug)]
struct Interval {
    lo: BigInt,
    hi: BigInt,
    zeros: u8,
}
impl Interval {
    fn unknown() -> Self {
        let hi = max();
        Self {
            lo: -&hi,
            hi,
            zeros: 3,
        }
    }
    fn constant(x: f64) -> Self {
        let lo = units(x);
        let zeros = if x == 0.0 {
            if x.is_sign_negative() { 2 } else { 1 }
        } else {
            0
        };
        Self {
            hi: lo.clone(),
            lo,
            zeros,
        }
    }
    fn hull(&self, other: &Self) -> Self {
        Self {
            lo: self.lo.clone().min(other.lo.clone()),
            hi: self.hi.clone().max(other.hi.clone()),
            zeros: self.zeros | other.zeros,
        }
    }
    fn rational(lo: BigInt, ld: BigInt, hi: BigInt, hd: BigInt) -> Option<Self> {
        let max = max();
        if lo < -(&max * &ld) || hi > &max * &hd {
            return None;
        }
        let lo = outward(floor_ratio(&lo, &ld), false);
        let hi = outward(ceil_ratio(&hi, &hd), true);
        let zeros = if lo <= BigInt::from(0) && hi >= BigInt::from(0) {
            3
        } else {
            0
        };
        Some(Self { lo, hi, zeros })
    }
}
fn transfer(op: Op, a: &Interval, b: &Interval) -> Option<Interval> {
    let one = BigInt::from(1);
    match op {
        Op::FloatAdd => Interval::rational(&a.lo + &b.lo, one.clone(), &a.hi + &b.hi, one),
        Op::FloatSub => Interval::rational(&a.lo - &b.hi, one.clone(), &a.hi - &b.lo, one),
        Op::FloatMul => {
            let products = [&a.lo * &b.lo, &a.lo * &b.hi, &a.hi * &b.lo, &a.hi * &b.hi];
            Interval::rational(
                products.iter().min().unwrap().clone(),
                scale(),
                products.iter().max().unwrap().clone(),
                scale(),
            )
        }
        Op::FloatDiv => {
            if b.lo == BigInt::from(0) && b.hi == BigInt::from(0) {
                return Some(Interval::constant(0.0));
            }
            if b.lo <= BigInt::from(0) && b.hi >= BigInt::from(0) {
                return None;
            }
            let mut fractions = Vec::new();
            for x in [&a.lo, &a.hi] {
                for y in [&b.lo, &b.hi] {
                    let mut n = x * scale();
                    let mut d = y.clone();
                    if d.sign() == Sign::Minus {
                        n = -n;
                        d = -d;
                    }
                    fractions.push((n, d));
                }
            }
            fractions.sort_by(|(an, ad), (bn, bd)| (an * bd).cmp(&(bn * ad)));
            let (lo, ld) = fractions.first().unwrap();
            let (hi, hd) = fractions.last().unwrap();
            Interval::rational(lo.clone(), ld.clone(), hi.clone(), hd.clone())
        }
        Op::FloatNeg => Some(Interval {
            lo: -&a.hi,
            hi: -&a.lo,
            zeros: ((a.zeros & 1) << 1) | ((a.zeros & 2) >> 1),
        }),
        _ => None,
    }
}
#[derive(Clone, PartialEq)]
struct Test {
    op: Op,
    args: Vec<Atom>,
    versions: Vec<Option<u64>>,
}
#[derive(Clone, Default)]
struct State {
    ranges: BTreeMap<Local, Interval>,
    tests: BTreeMap<Local, Test>,
    versions: BTreeMap<Local, u64>,
}
impl State {
    fn range(&self, a: &Atom) -> Interval {
        match a {
            Atom::Float(x) => Interval::constant(*x),
            Atom::Local(l) => self
                .ranges
                .get(l)
                .cloned()
                .unwrap_or_else(Interval::unknown),
            _ => Interval::unknown(),
        }
    }
    fn version(&self, a: &Atom) -> Option<u64> {
        if let Atom::Local(l) = a {
            self.versions.get(l).copied()
        } else {
            None
        }
    }
}
fn merge(states: Vec<State>, fresh: &mut u64) -> Option<State> {
    let mut states = states.into_iter();
    let mut out = states.next()?;
    for state in states {
        out.ranges.retain(|l, range| {
            if let Some(other) = state.ranges.get(l) {
                *range = range.hull(other);
                true
            } else {
                false
            }
        });
        out.tests.retain(|l, test| state.tests.get(l) == Some(test));
        for (&l, &v) in &state.versions {
            if out.versions.get(&l) != Some(&v) {
                *fresh += 1;
                out.versions.insert(l, *fresh);
            }
        }
    }
    Some(out)
}
fn restrict(state: &mut State, atom: &Atom, lo: Option<BigInt>, hi: Option<BigInt>) -> bool {
    let mut range = state.range(atom);
    if let Some(lo) = lo {
        range.lo = range.lo.max(lo);
    }
    if let Some(hi) = hi {
        range.hi = range.hi.min(hi);
    }
    if range.lo > range.hi {
        return false;
    }
    if range.lo > BigInt::from(0) || range.hi < BigInt::from(0) {
        range.zeros = 0;
    }
    if let Atom::Local(l) = atom {
        state.ranges.insert(*l, range);
    }
    true
}
fn comparison(state: &mut State, op: Op, args: &[Atom], truth: bool) -> bool {
    let (a, b) = (&args[0], &args[1]);
    let (op, a, b) = match op {
        Op::FloatGt => (Op::FloatLt, b, a),
        Op::FloatGe => (Op::FloatLe, b, a),
        _ => (op, a, b),
    };
    let (a, b, strict) = match (op, truth) {
        (Op::FloatLt, true) | (Op::FloatLe, false) => (a, b, true),
        (Op::FloatLe, true) | (Op::FloatLt, false) => (a, b, false),
        _ => return true,
    };
    // Negating x < y gives y <= x; negating x <= y gives y < x.
    let (a, b) = if truth { (a, b) } else { (b, a) };
    let ar = state.range(a);
    let br = state.range(b);
    let hi = if strict {
        outward(&br.hi - 1, false)
    } else {
        br.hi
    };
    let lo = if strict {
        outward(&ar.lo + 1, true)
    } else {
        ar.lo
    };
    restrict(state, a, None, Some(hi)) && restrict(state, b, Some(lo), None)
}
fn assume(mut state: State, cond: &Cond, truth: bool) -> Option<State> {
    let valid = match cond {
        Cond::Not(c) => return assume(state, c, !truth),
        Cond::And(a, b) if truth => return assume(assume(state, a, true)?, b, true),
        Cond::Or(a, b) if !truth => return assume(assume(state, a, false)?, b, false),
        Cond::Bool(Atom::Bool(b), expected) => (*b == *expected) == truth,
        Cond::True(Atom::Local(l))
        | Cond::Bool(Atom::Local(l), true)
        | Cond::Bool(Atom::Local(l), false) => {
            let truth = if matches!(cond, Cond::Bool(_, false)) {
                !truth
            } else {
                truth
            };
            if let Some(test) = state.tests.get(l).cloned()
                && test
                    .args
                    .iter()
                    .map(|a| state.version(a))
                    .collect::<Vec<_>>()
                    == test.versions
            {
                comparison(&mut state, test.op, &test.args, truth)
            } else {
                true
            }
        }
        Cond::IsFloat(a, x) if truth => {
            let n = units(*x);
            restrict(&mut state, a, Some(n.clone()), Some(n))
        }
        _ => true,
    };
    valid.then_some(state)
}
struct Analysis {
    next: u64,
    removed: usize,
    breaks: BTreeMap<Label, Vec<State>>,
}
impl Analysis {
    fn body(&mut self, body: &mut [Stmt], mut state: Option<State>) -> Option<State> {
        for stmt in body {
            let Some(mut current) = state.take() else {
                break;
            };
            state = match stmt {
                Stmt::Let { dst, value, .. } => {
                    let old_test = match value {
                        Value::Atom(Atom::Local(l)) => current.tests.get(l).cloned(),
                        _ => None,
                    };
                    let range = match value {
                        Value::Atom(Atom::Float(x)) => Some(Interval::constant(*x)),
                        Value::Atom(Atom::Local(l)) => current.ranges.get(l).cloned(),
                        Value::Op(op, args)
                            if matches!(
                                op,
                                Op::FloatAdd
                                    | Op::FloatSub
                                    | Op::FloatMul
                                    | Op::FloatDiv
                                    | Op::FloatNeg
                            ) =>
                        {
                            let a = current.range(&args[0]);
                            let b = args
                                .get(1)
                                .map(|a| current.range(a))
                                .unwrap_or_else(Interval::unknown);
                            let proved = transfer(*op, &a, &b);
                            if proved.is_some() && *op != Op::FloatNeg {
                                *value = Value::Finite(*op, args.clone());
                                self.removed += 1;
                            }
                            Some(proved.unwrap_or_else(Interval::unknown))
                        }
                        _ => None,
                    };
                    let test = match value {
                        Value::Op(op, args)
                            if matches!(
                                op,
                                Op::FloatLt | Op::FloatLe | Op::FloatGt | Op::FloatGe
                            ) =>
                        {
                            Some(Test {
                                op: *op,
                                args: args.clone(),
                                versions: args.iter().map(|a| current.version(a)).collect(),
                            })
                        }
                        _ => old_test,
                    };
                    current.ranges.remove(dst);
                    current.tests.remove(dst);
                    self.next += 1;
                    current.versions.insert(*dst, self.next);
                    if let Some(range) = range {
                        current.ranges.insert(*dst, range);
                    }
                    if let Some(test) = test {
                        current.tests.insert(*dst, test);
                    }
                    Some(current)
                }
                Stmt::If {
                    cond,
                    then,
                    otherwise,
                } => {
                    let a = self.body(then, assume(current.clone(), cond, true));
                    let b = self.body(otherwise, assume(current, cond, false));
                    merge(a.into_iter().chain(b).collect(), &mut self.next)
                }
                Stmt::Block { label, body } => {
                    let normal = self.body(body, Some(current));
                    let mut states = self.breaks.remove(label).unwrap_or_default();
                    states.extend(normal);
                    merge(states, &mut self.next)
                }
                Stmt::Break(label) => {
                    self.breaks.entry(*label).or_default().push(current);
                    None
                }
                Stmt::Return(_) | Stmt::TailCall { .. } | Stmt::Unreachable => None,
                _ => Some(current),
            };
        }
        state
    }
}
pub fn run(f: &mut Func) -> usize {
    if rewrite::size(&f.body) > 10000 {
        return 0;
    }
    let mut state = State::default();
    for l in 0..f.arity {
        state.versions.insert(l, 0);
    }
    let mut analysis = Analysis {
        next: 0,
        removed: 0,
        breaks: BTreeMap::new(),
    };
    analysis.body(&mut f.body, Some(state));
    analysis.removed
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_outward_binary64_bounds() {
        // The proof algorithm is checked independently at normal/subnormal
        // rounding boundaries and immediately below the overflow threshold.
        for bits in [
            1,
            2,
            3,
            0x000f_ffff_ffff_ffff,
            0x0010_0000_0000_0000,
            0x3ff0_0000_0000_0000,
            0x7fef_ffff_ffff_ffff,
        ] {
            let x = f64::from_bits(bits);
            let u = units(x);
            assert_eq!(outward(u.clone(), false), u);
            assert_eq!(outward(u.clone(), true), u);
            let n = Interval::constant(x);
            assert!(transfer(Op::FloatMul, &n, &Interval::constant(0.5)).is_some());
        }
        let huge = Interval::constant(f64::MAX);
        assert!(transfer(Op::FloatAdd, &huge, &huge).is_none());
        assert!(transfer(Op::FloatMul, &huge, &Interval::constant(2.0)).is_none());
        let tiny = Interval::constant(f64::from_bits(1));
        let half = transfer(Op::FloatMul, &tiny, &Interval::constant(0.5)).unwrap();
        assert_eq!(half.lo, BigInt::from(0));
        assert_eq!(half.hi, BigInt::from(1));
        let negative = transfer(
            Op::FloatMul,
            &Interval::constant(-f64::from_bits(1)),
            &Interval::constant(0.5),
        )
        .unwrap();
        assert_eq!(negative.lo, BigInt::from(-1));
        assert_eq!(negative.hi, BigInt::from(0));
        assert!(transfer(Op::FloatDiv, &huge, &tiny).is_none());
    }
}
