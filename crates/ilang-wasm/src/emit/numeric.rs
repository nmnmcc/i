//! Local numeric emission. Keep construction and reads in the same Wasm body
//! so Binaryen's ordinary Heap2Local pass can see their complete use chains.
use super::*;

pub(super) fn box_float(raw: &str) -> String {
    format!(
        "(struct.new $obj (i32.const 2) (i32.const 0) (i64.const 0) {raw} (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call))"
    )
}

pub(super) fn unbox_float(value: &str) -> String {
    format!("(struct.get $obj 3 (ref.cast (ref $obj) {value}))")
}

pub(super) fn checked(raw: &str) -> String {
    format!(
        "(block (result f64) (local.set $numeric_result {raw}) (if (i32.eqz (f64.le (f64.abs (local.get $numeric_result)) (f64.const 0x1.fffffffffffffp+1023))) (then (call $stop_float) unreachable)) (local.get $numeric_result))"
    )
}

pub(super) fn float_op(op: Op, args: &[String]) -> Option<String> {
    float_operation(op, args, true)
}
pub(super) fn float_operation(op: Op, args: &[String], check: bool) -> Option<String> {
    let instruction = match op {
        Op::FloatAdd => "f64.add",
        Op::FloatSub => "f64.sub",
        Op::FloatMul => "f64.mul",
        Op::FloatDiv => "f64.div",
        Op::FloatNeg => "f64.neg",
        _ => return None,
    };
    let a = &args[0];
    let raw = if op == Op::FloatNeg {
        format!("({instruction} {a})")
    } else if op == Op::FloatDiv {
        // Both operands have already been evaluated in source order by ANF.
        // The zero case deliberately constructs positive zero.
        format!(
            "(block (result f64) (local.set $numeric_a {a}) (local.set $numeric_b {}) (if (result f64) (f64.eq (local.get $numeric_b) (f64.const 0)) (then (f64.const 0)) (else (f64.div (local.get $numeric_a) (local.get $numeric_b)))))",
            args[1]
        )
    } else {
        format!("({instruction} {a} {})", args[1])
    };
    Some(if op == Op::FloatNeg || !check {
        raw
    } else {
        checked(&raw)
    })
}

pub(crate) fn has_float(stmts: &[Stmt]) -> bool {
    let mut work = vec![stmts];
    while let Some(stmts) = work.pop() {
        for stmt in stmts {
            match stmt {
                Stmt::Let {
                    value: Value::Op(op, _) | Value::Finite(op, _),
                    ..
                }
                | Stmt::Suspendable {
                    value: Value::Op(op, _) | Value::Finite(op, _),
                    ..
                } if matches!(
                    op,
                    Op::FloatAdd | Op::FloatSub | Op::FloatMul | Op::FloatDiv | Op::FloatNeg
                ) =>
                {
                    return true;
                }
                Stmt::If {
                    then, otherwise, ..
                } => {
                    work.push(then);
                    work.push(otherwise);
                }
                Stmt::Block { body, .. } => work.push(body),
                _ => {}
            }
        }
    }
    false
}

/// A local encoding, never a source type. Int uses an i64 containing a proved
/// i31 value and a nullable fallback holding an ordinary big Int. Null means
/// the scalar is current; all escape reads materialize canonical i31 values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Repr {
    Float,
    Int,
    Bool,
}

#[derive(Default)]
pub(super) struct Plan {
    pub locals: std::collections::BTreeMap<Local, Repr>,
    pub small: std::collections::BTreeMap<Local, (i64, i64)>,
}

impl Plan {
    pub fn for_func(
        f: &Func,
        facts: Option<&HashMap<Local, ilang_check::types::NumericFact>>,
        worker: Option<&crate::numeric::Worker>,
    ) -> Self {
        let mut definitions: HashMap<Local, Vec<&Value>> = HashMap::new();
        let mut work = vec![f.body.as_slice()];
        while let Some(stmts) = work.pop() {
            for stmt in stmts {
                match stmt {
                    Stmt::Let { dst, value, .. } | Stmt::Suspendable { dst, value, .. } => {
                        definitions.entry(*dst).or_default().push(value)
                    }
                    Stmt::If {
                        then, otherwise, ..
                    } => {
                        work.push(then);
                        work.push(otherwise);
                    }
                    Stmt::Block { body, .. } => work.push(body),
                    _ => {}
                }
            }
        }
        let mut plan = Self::default();
        if let Some(worker) = worker {
            for (l, int) in worker.ints.iter().enumerate() {
                if *int {
                    plan.locals.insert(l as u32, Repr::Int);
                }
            }
        }
        if let Some(facts) = facts {
            for (&local, fact) in facts {
                if local < f.arity && *fact == ilang_check::types::NumericFact::FiniteF64 {
                    plan.locals.insert(local, Repr::Float);
                }
            }
        }
        // The original structured IR is not SSA. Only single definitions can
        // select an encoding here; multi-definition joins remain Value. No
        // facts from one branch can change an incoming edge in another branch.
        loop {
            let before = plan.locals.len();
            for (&local, definitions) in &definitions {
                if definitions.len() > 1
                    && definitions.iter().all(|v| match v {
                        Value::Atom(Atom::Float(_)) => true,
                        Value::Atom(Atom::Local(source)) => {
                            plan.locals.get(source) == Some(&Repr::Float)
                        }
                        Value::Op(
                            Op::FloatAdd
                            | Op::FloatSub
                            | Op::FloatMul
                            | Op::FloatDiv
                            | Op::FloatNeg,
                            _,
                        ) => true,
                        _ => false,
                    })
                {
                    plan.locals.insert(local, Repr::Float);
                }
                if definitions.len() != 1
                    || local < f.arity
                    || f.locals[local as usize] != LocalType::Value
                {
                    continue;
                }
                let value = definitions[0];
                let repr = match value {
                    Value::Op(
                        Op::FloatAdd | Op::FloatSub | Op::FloatMul | Op::FloatDiv | Op::FloatNeg,
                        _,
                    )
                    | Value::Finite(_, _)
                    | Value::Atom(Atom::Float(_)) => Some(Repr::Float),
                    Value::Op(
                        Op::IntAdd | Op::IntSub | Op::IntMul | Op::IntDiv | Op::IntRem | Op::IntNeg,
                        _,
                    )
                    | Value::Atom(Atom::Int(_)) => Some(Repr::Int),
                    Value::Op(Op::Equal | Op::NotEqual, args)
                        if [Repr::Float, Repr::Int].iter().any(|repr| {
                            args.iter().all(|a| match a {
                                Atom::Local(l) => plan.locals.get(l) == Some(repr),
                                Atom::Float(_) => *repr == Repr::Float,
                                Atom::Int(_) => *repr == Repr::Int,
                                _ => false,
                            })
                        }) =>
                    {
                        Some(Repr::Bool)
                    }
                    Value::Op(
                        Op::FloatLt
                        | Op::FloatLe
                        | Op::FloatGt
                        | Op::FloatGe
                        | Op::IntLt
                        | Op::IntLe
                        | Op::IntGt
                        | Op::IntGe,
                        _,
                    ) => Some(Repr::Bool),
                    Value::Atom(Atom::Local(source)) => plan.locals.get(source).copied(),
                    // Only a final checker fact about this fresh definition,
                    // never the consumer's requested type, licenses this cast.
                    _ => match facts.and_then(|facts| facts.get(&local)) {
                        Some(ilang_check::types::NumericFact::FiniteF64) => Some(Repr::Float),
                        _ => None,
                    },
                };
                if let Some(repr) = repr {
                    plan.locals.insert(local, repr);
                }
            }
            if plan.locals.len() == before {
                break;
            }
        }
        if let Some(worker) = worker {
            for (l, int) in worker.ints.iter().enumerate() {
                if *int {
                    plan.small.insert(l as u32, (-1073741824, 1073741823));
                }
            }
        }
        loop {
            let before = plan.small.len();
            for (&local, values) in &definitions {
                if values.len() != 1
                    || plan.small.contains_key(&local)
                    || plan.locals.get(&local) != Some(&Repr::Int)
                {
                    continue;
                }
                let interval = match values[0] {
                    Value::Atom(a) => plan.interval(a),
                    Value::Op(op, args) => {
                        let Some((lo, hi)) = plan.interval(&args[0]) else {
                            continue;
                        };
                        let (lo, hi) = (lo as i128, hi as i128);
                        let bounds = if *op == Op::IntNeg {
                            (-hi, -lo)
                        } else {
                            let Some((bl, bh)) = plan.interval(&args[1]) else {
                                continue;
                            };
                            let (bl, bh) = (bl as i128, bh as i128);
                            match op {
                                Op::IntAdd => (lo + bl, hi + bh),
                                Op::IntSub => (lo - bh, hi - bl),
                                Op::IntMul => {
                                    let corners = [lo * bl, lo * bh, hi * bl, hi * bh];
                                    (
                                        *corners.iter().min().unwrap(),
                                        *corners.iter().max().unwrap(),
                                    )
                                }
                                Op::IntDiv | Op::IntRem => {
                                    let max = lo.abs().max(hi.abs());
                                    if bl == 0 && bh == 0 {
                                        (0, 0)
                                    } else {
                                        (-max, max)
                                    }
                                }
                                _ => continue,
                            }
                        };
                        (bounds.0 >= -1073741824 && bounds.1 <= 1073741823)
                            .then_some((bounds.0 as i64, bounds.1 as i64))
                    }
                    _ => None,
                };
                if let Some(bounds) = interval {
                    plan.small.insert(local, bounds);
                }
            }
            if plan.small.len() == before {
                break;
            }
        }
        plan
    }

    fn interval(&self, a: &Atom) -> Option<(i64, i64)> {
        match a {
            Atom::Int(n) => {
                let n = i64::try_from(n).ok()?;
                (-1073741824..=1073741823).contains(&n).then_some((n, n))
            }
            Atom::Local(l) => self.small.get(l).copied(),
            _ => None,
        }
    }

    pub fn parameters(&self, arity: u32) -> String {
        let mut text = String::new();
        for (&local, repr) in &self.locals {
            if local < arity && *repr == Repr::Float {
                assert_eq!(
                    *repr,
                    Repr::Float,
                    "only final Float parameter facts are unboxed at entry"
                );
                writeln!(
                    text,
                    "(local.set $numeric_f{local} {})",
                    unbox_float(&format!("(local.get $l{local})"))
                )
                .unwrap();
            }
        }
        text
    }

    pub fn declarations(&self) -> String {
        let mut text = String::new();
        for (local, repr) in &self.locals {
            let (prefix, ty) = match repr {
                Repr::Float => ("f", "f64"),
                Repr::Int => ("i", "i64"),
                Repr::Bool => ("c", "i32"),
            };
            writeln!(text, " (local $numeric_{prefix}{local} {ty})").unwrap();
        }
        text
    }
}

impl Emitter<'_> {
    pub(super) fn restore_scalar(&self, local: Local) -> String {
        match self.numeric_plan.locals.get(&local) {
            Some(Repr::Float) => format!(
                "(local.set $numeric_f{local} {})\n",
                unbox_float(&format!("(local.get $l{local})"))
            ),
            Some(Repr::Int) => Self::int_normalize(local),
            Some(Repr::Bool) => format!(
                "(local.set $numeric_c{local} (call $is_bool (local.get $l{local}) (i32.const 1)))\n"
            ),
            None => String::new(),
        }
    }
    pub(super) fn float_call(&self, callee: &Callee) -> bool {
        match callee {
            Callee::Direct(f) | Callee::Known { func: f, .. } => {
                self.p.workers.get(f).is_some_and(|w| w.float_result)
            }
            _ => false,
        }
    }

    pub(super) fn float_atom(&mut self, atom: &Atom) -> String {
        match atom {
            Atom::Float(value) => format!(
                "(f64.reinterpret_i64 (i64.const {}))",
                value.to_bits() as i64
            ),
            Atom::Local(local) if self.numeric_plan.locals.get(local) == Some(&Repr::Float) => {
                format!("(local.get $numeric_f{local})")
            }
            _ => unbox_float(&self.atom(atom)),
        }
    }

    /// (is-small, exact small scalar). The scalar expression is only evaluated
    /// in the guarded successor. General Int never goes through int_to_i64.
    pub(super) fn small_int(&mut self, atom: &Atom) -> (String, String) {
        if let Atom::Local(local) = atom
            && self.numeric_plan.locals.get(local) == Some(&Repr::Int)
        {
            return (
                if self.numeric_plan.small.contains_key(local) {
                    "(i32.const 1)".into()
                } else {
                    format!("(ref.is_null (local.get $l{local}))")
                },
                format!("(local.get $numeric_i{local})"),
            );
        }
        if let Atom::Int(value) = atom {
            if let Ok(value) = value.to_string().parse::<i32>()
                && (-(1 << 30)..(1 << 30)).contains(&value)
            {
                return ("(i32.const 1)".into(), format!("(i64.const {value})"));
            }
            return ("(i32.const 0)".into(), "(i64.const 0)".into());
        }
        let boxed = self.atom(atom);
        (
            format!("(ref.test (ref i31) {boxed})"),
            format!("(i64.extend_i32_s (i31.get_s (ref.cast (ref i31) {boxed})))"),
        )
    }

    pub(super) fn int_normalize(local: Local) -> String {
        format!(
            "(if (ref.test (ref i31) (local.get $l{local})) (then (local.set $numeric_i{local} (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $l{local}))))) (local.set $l{local} (ref.null eq))))"
        )
    }

    pub(super) fn scalar_assignment(&mut self, local: Local, value: &Value) -> Option<String> {
        let repr = *self.numeric_plan.locals.get(&local)?;
        Some(match repr {
            Repr::Float => {
                let mut reference = "(ref.null eq)".to_string();
                let raw = match value {
                    Value::Finite(op, args) => {
                        let args = args.iter().map(|a| self.float_atom(a)).collect::<Vec<_>>();
                        float_operation(*op, &args, false).expect("proved Float operation")
                    }
                    Value::Call { callee, args, env } if self.float_call(callee) => {
                        self.call(callee, args, false, &env_text(*env))
                    }
                    Value::Atom(atom) => {
                        if let Atom::Local(source) = atom
                            && self.numeric_plan.locals.get(source) == Some(&Repr::Float)
                        {
                            reference = format!("(local.get $l{source})");
                        }
                        self.float_atom(atom)
                    }
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
                        let args = args.iter().map(|a| self.float_atom(a)).collect::<Vec<_>>();
                        float_op(*op, &args).expect("Float definition has a scalar operation")
                    }
                    _ => {
                        // A call/projection already produced a canonical Float.
                        // Keep that reference rather than throwing it away and
                        // allocating a second box when the value escapes.
                        let boxed = self.value(value);
                        return Some(format!(
                            "(local.set $l{local} {boxed})\n(local.set $numeric_f{local} {})\n",
                            unbox_float(&format!("(local.get $l{local})"))
                        ));
                    }
                };
                format!("(local.set $numeric_f{local} {raw})\n(local.set $l{local} {reference})\n")
            }
            Repr::Int => match value {
                Value::Atom(atom) => {
                    let (small, raw) = self.small_int(atom);
                    let boxed = self.atom(atom);
                    format!(
                        "(if {small} (then (local.set $numeric_i{local} {raw}) (local.set $l{local} (ref.null eq))) (else (local.set $l{local} {boxed}) {}))\n",
                        Self::int_normalize(local)
                    )
                }
                Value::Op(op, args) => {
                    let (small_a, a) = self.small_int(&args[0]);
                    let (guard, raw) = if *op == Op::IntNeg {
                        (small_a, format!("(i64.sub (i64.const 0) {a})"))
                    } else {
                        let (small_b, b) = self.small_int(&args[1]);
                        let instruction = match op {
                            Op::IntAdd => "i64.add",
                            Op::IntSub => "i64.sub",
                            Op::IntMul => "i64.mul",
                            Op::IntDiv => "i64.div_s",
                            Op::IntRem => "i64.rem_s",
                            _ => panic!("noninteger definition has Int encoding"),
                        };
                        let raw = format!("({instruction} {a} {b})");
                        let raw = if matches!(op, Op::IntDiv | Op::IntRem) {
                            format!(
                                "(if (result i64) (i64.eqz {b}) (then (i64.const 0)) (else {raw}))"
                            )
                        } else {
                            raw
                        };
                        (format!("(i32.and {small_a} {small_b})"), raw)
                    };
                    if self.numeric_plan.small.contains_key(&local) {
                        return Some(format!(
                            "(local.set $numeric_i{local} {raw})\n(local.set $l{local} (ref.null eq))\n"
                        ));
                    }
                    let generic = self.value(value);
                    format!(
                        "(if {guard} (then (local.set $numeric_i{local} {raw}) (if (i32.and (i64.ge_s (local.get $numeric_i{local}) (i64.const -1073741824)) (i64.le_s (local.get $numeric_i{local}) (i64.const 1073741823))) (then (local.set $l{local} (ref.null eq))) (else (local.set $l{local} (call $int_from_i64 (local.get $numeric_i{local})))))) (else (local.set $l{local} {generic}) {}))\n",
                        Self::int_normalize(local)
                    )
                }
                _ => panic!("Int encoding without integer definition"),
            },
            Repr::Bool => {
                let raw = match value {
                    Value::Atom(Atom::Local(source)) => {
                        assert_eq!(self.numeric_plan.locals.get(source), Some(&Repr::Bool));
                        format!("(local.get $numeric_c{source})")
                    }
                    Value::Op(op, args) if matches!(op, Op::Equal | Op::NotEqual) => {
                        let float = args.iter().all(|a| matches!(a, Atom::Float(_)) || matches!(a, Atom::Local(l) if self.numeric_plan.locals.get(l) == Some(&Repr::Float)));
                        let equal = if float {
                            let a = self.float_atom(&args[0]);
                            let b = self.float_atom(&args[1]);
                            format!("(f64.eq {a} {b})")
                        } else {
                            let (ga, a) = self.small_int(&args[0]);
                            let (gb, b) = self.small_int(&args[1]);
                            let ca = self.atom(&args[0]);
                            let cb = self.atom(&args[1]);
                            format!(
                                "(if (result i32) (i32.and {ga} {gb}) (then (i64.eq {a} {b})) (else (call $equal {ca} {cb})))"
                            )
                        };
                        if *op == Op::NotEqual {
                            format!("(i32.eqz {equal})")
                        } else {
                            equal
                        }
                    }
                    Value::Op(op, args) => {
                        let (float, instruction) = match op {
                            Op::FloatLt => (true, "f64.lt"),
                            Op::FloatLe => (true, "f64.le"),
                            Op::FloatGt => (true, "f64.gt"),
                            Op::FloatGe => (true, "f64.ge"),
                            Op::IntLt => (false, "i64.lt_s"),
                            Op::IntLe => (false, "i64.le_s"),
                            Op::IntGt => (false, "i64.gt_s"),
                            Op::IntGe => (false, "i64.ge_s"),
                            _ => panic!("Bool encoding without numeric comparison"),
                        };
                        if float {
                            let a = self.float_atom(&args[0]);
                            let b = self.float_atom(&args[1]);
                            format!("({instruction} {a} {b})")
                        } else {
                            let (small_a, a) = self.small_int(&args[0]);
                            let (small_b, b) = self.small_int(&args[1]);
                            let boxed_a = self.atom(&args[0]);
                            let boxed_b = self.atom(&args[1]);
                            let generic = instruction.replace("i64", "i32");
                            format!(
                                "(if (result i32) (i32.and {small_a} {small_b}) (then ({instruction} {a} {b})) (else ({generic} (call $int_compare {boxed_a} {boxed_b}) (i32.const 0))))"
                            )
                        }
                    }
                    _ => panic!("Bool encoding without comparison definition"),
                };
                format!("(local.set $numeric_c{local} {raw})\n")
            }
        })
    }
}
