//! From the intermediate form to a Wasm module: WAT text with the runtime
//! of `runtime/mod.rs`, assembled to a binary, with the `ilang.abi` custom
//! section (B.6).

use crate::ir::*;
use crate::runtime;
use num_bigint::{BigInt, Sign};
use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

mod control;
mod host;

/// Support functions of the generated code, for tests in patterns.
const SUPPORT: &str = r#"
(func $is_ctor (param $v eqref) (param $c i32) (result i32)
  (if (result i32) (i32.eq (call $value_tag (local.get $v)) (i32.const 5))
    (then (i32.eq (struct.get $obj 1 (ref.cast (ref $obj) (local.get $v))) (local.get $c)))
    (else (i32.const 0))))
(func $is_type (param $v eqref) (param $t i64) (result i32)
  (if (result i32) (i32.eq (call $value_tag (local.get $v)) (i32.const 5))
    (then (i64.eq (struct.get $obj 2 (ref.cast (ref $obj) (local.get $v))) (local.get $t)))
    (else (i32.const 0))))
(func $is_bool (param $v eqref) (param $b i32) (result i32)
  (if (result i32) (i32.eq (call $value_tag (local.get $v)) (i32.const 1))
    (then (i32.eq (struct.get $obj 1 (ref.cast (ref $obj) (local.get $v))) (local.get $b)))
    (else (i32.const 0))))
(func $is_int (param $v eqref) (param $n eqref) (result i32)
  (if (result i32) (i32.eq (call $value_tag (local.get $v)) (i32.const 8))
    (then (i32.eqz (call $int_compare (local.get $v) (local.get $n))))
    (else (i32.const 0))))
(func $is_float (param $v eqref) (param $x f64) (result i32)
  (if (result i32) (i32.eq (call $value_tag (local.get $v)) (i32.const 2))
    (then (f64.eq (struct.get $obj 3 (ref.cast (ref $obj) (local.get $v))) (local.get $x)))
    (else (i32.const 0))))
(func $is_string (param $v eqref) (param $s eqref) (result i32)
  (if (result i32) (i32.eq (call $value_tag (local.get $v)) (i32.const 3))
    (then (i32.eqz (call $string_compare (local.get $v) (local.get $s))))
    (else (i32.const 0))))
(func $is_tuple (param $v eqref) (param $n i32) (result i32)
  (if (result i32) (i32.eq (call $value_tag (local.get $v)) (i32.const 4))
    (then (i32.eq (array.len (ref.as_non_null (struct.get $obj 5 (ref.cast (ref $obj) (local.get $v)))))
      (local.get $n)))
    (else (i32.const 0))))
"#;

/// The `fail` of the `Assert` of entry points, after the host part.
const ENTRY: &str = r#"
;; The `fail` of the `Assert` of an entry point (§9.6, §11.7).
(func $entry_assert_fail (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $failure eqref) (result eqref)
  (call $stop_assertion (call $field (local.get $failure) (i32.const 0)))
  (unreachable))
"#;

/// The runtime functions whose references are taken.
const DECLARED: &[&str] = &[
    "$entry_assert_fail",
    "$catch_reenter",
    "$ensure_reenter",
    "$final_reenter",
    "$boundary_reenter",
    "$cancel_reenter",
    "$control_unpend",
];

/// The WAT text of the whole module.
pub fn wat(p: &Program) -> String {
    let mut e = Emitter {
        p,
        arities: BTreeSet::new(),
        ctors: BTreeSet::new(),
        strings: HashMap::new(),
        data: String::new(),
        out: String::new(),
    };
    e.arities.insert(0);
    e.arities.insert(1);
    for (i, f) in p.funcs.iter().enumerate() {
        e.func(FuncId(i as u32), f);
    }
    for i in 0..p.points.len() {
        e.point(i as PointId);
    }
    for i in 0..p.joins.len() {
        e.join(i as JoinId);
    }
    if let Some(start) = &p.start {
        e.code("$start", start);
    }
    e.start();
    e.exports();
    e.resume_exports();
    let mut m = String::from("(module\n");
    // A host reference can come into the program only through an import or
    // an export whose types have one (B.2).
    let references = p
        .imports
        .iter()
        .map(|x| &x.abi)
        .chain(p.exports.iter().filter_map(|e| e.abi.as_ref()))
        .any(|abi| {
            abi.params
                .iter()
                .chain(&abi.errors)
                .chain([&abi.result])
                .any(|t| matches!(t, AbiType::Externref(_)))
        });
    m.push_str(&runtime::types_for_host_references(references));
    m.push_str("(type $env (array (mut eqref)))\n");
    m.push_str(control::TYPES);
    m.push_str(host::TYPES);
    for n in &e.arities {
        let params = " eqref".repeat(*n as usize);
        let _ = writeln!(
            m,
            "(type $fn{n} (func (param (ref null $obj) (ref null $env){params}) (result eqref)))\n\
             (type $clo{n} (sub final $obj (struct (field i32) (field i32) (field i64) (field f64) \
             (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs)) \
             (field (ref null $call)) (field (ref $fn{n})))))"
        );
    }
    let wasi = p.target == Target::Wasi;
    if wasi {
        m.push_str(runtime::WASI_HOST);
        if references {
            m.push_str(runtime::WASI_REFERENCE_ID_IMPORT);
        }
    } else {
        m.push_str(runtime::HOST);
    }
    // The imports of B.4, before every definition.
    for (i, x) in p.imports.iter().enumerate() {
        if x.kind == ImportKind::WasiStdout {
            continue;
        }
        let mut params: Vec<&str> = x
            .abi
            .params
            .iter()
            .flat_map(|t| abi_in(*t).0.iter().copied())
            .collect();
        if x.kind == ImportKind::AsyncService {
            params.insert(0, "i32");
        }
        let results = if x.kind == ImportKind::AsyncService {
            String::new()
        } else {
            result_list(&import_results(&x.abi))
        };
        let _ = writeln!(
            m,
            "(import \"{}\" \"{}\" (func $imp{i}{}{results}))",
            wat_string(&x.module),
            wat_string(&x.name),
            if params.is_empty() {
                String::new()
            } else {
                format!(" (param {})", params.join(" "))
            }
        );
    }
    if wasi {
        m.push_str(runtime::WASI_RUNTIME);
        if !references {
            m.push_str(runtime::WASI_REFERENCE_ID);
        }
        for (i, x) in p.imports.iter().enumerate() {
            if x.kind == ImportKind::WasiStdout {
                let _ = writeln!(
                    m,
                    "(func $imp{i} (param i32 i32) (call $wasi_line (i32.const 1) (local.get 0) (local.get 1)))"
                );
            }
        }
        if let Some(entry) = &p.entry {
            let _ = writeln!(
                m,
                "(func (export \"_start\") (call $ilang_start) (call $entry_{}))",
                export_id(entry)
            );
        }
    }
    m.push_str(runtime::wat_for_host_references(references));
    m.push_str(SUPPORT);
    m.push_str(control::EFFECTS);
    m.push_str(host::EFFECTS);
    m.push_str(ENTRY);
    m.push_str(&e.data);
    for (ctor, ty) in &e.ctors {
        let _ = writeln!(
            m,
            "(global $k{ctor}_{ty} eqref (struct.new $obj (i32.const 5) (i32.const {ctor}) \
             (i64.const {ty}) (f64.const 0) (ref.null $bytes) (ref.null $values) (ref.null $limbs) \
             (ref.null $call)))"
        );
    }
    for g in 0..p.globals {
        let _ = writeln!(m, "(global $g{g} (mut eqref) (ref.null eq))");
    }
    // The Wasm values of the parameters and results of each import (B.2).
    for (i, x) in p.imports.iter().enumerate() {
        let params = x
            .abi
            .params
            .iter()
            .flat_map(|t| abi_in(*t).0.iter().copied());
        for (j, t) in params.enumerate() {
            let _ = writeln!(m, "(global $hp{i}_{j} (mut {t}) {})", zero(t));
        }
        for (j, t) in import_results(&x.abi).into_iter().enumerate() {
            let _ = writeln!(m, "(global $hr{i}_{j} (mut {t}) {})", zero(t));
        }
    }
    for &f in &p.constants {
        let n = p.funcs[f.0 as usize].arity();
        let _ = writeln!(
            m,
            "(global $c{} (ref $clo{n}) {})",
            f.0,
            closure(n, f, "(ref.null $values)")
        );
    }
    m.push_str(&e.out);
    let mut declared: Vec<String> = (0..p.funcs.len()).map(|i| format!("$f{i}")).collect();
    declared.extend((0..p.points.len()).map(|i| format!("$r{i}")));
    declared.extend(DECLARED.iter().map(|d| d.to_string()));
    let _ = writeln!(m, "(elem declare func {})", declared.join(" "));
    m.push_str(")\n");
    m
}

fn closure(arity: u32, f: FuncId, captures: &str) -> String {
    format!(
        "(struct.new $clo{arity} (i32.const 7) (i32.const 0) (i64.const {arity}) (f64.const 0) \
         (ref.null $bytes) {captures} (ref.null $limbs) (ref.null $call) (ref.func $f{}))",
        f.0
    )
}

struct Emitter<'p> {
    p: &'p Program,
    arities: BTreeSet<u32>,
    /// The constructors without fields that are used, as (constructor id,
    /// type id).
    ctors: BTreeSet<(u32, u32)>,
    strings: HashMap<Vec<u8>, usize>,
    data: String,
    out: String,
}

impl<'p> Emitter<'p> {
    fn func(&mut self, id: FuncId, f: &FuncKind) {
        let name = format!("$f{}", id.0);
        match f {
            FuncKind::Code(func) => self.code(&name, func),
            // A constructor or a library function used as a value has a
            // frame too, so that it takes the tail site of a tail call
            // (§11.7).
            FuncKind::Ctor { ctor, ty, arity } => {
                let locals = vec![LocalType::Value; *arity as usize + 1];
                self.header(&name, *arity, &locals);
                let fields: String = (0..*arity).map(|i| format!(" (local.get $l{i})")).collect();
                let _ = writeln!(
                    self.out,
                    "(call $shadow_enter_call)\n\
                     (local.set $l{arity} (call $record (i32.const {ctor}) (i64.const {ty}) (array.new_fixed $values {arity}{fields})))\n\
                     (call $shadow_leave) (local.get $l{arity}))"
                );
            }
            FuncKind::Runtime { op, arity } => {
                let locals = vec![LocalType::Value; *arity as usize + 1];
                self.header(&name, *arity, &locals);
                let args: Vec<String> = (0..*arity).map(|i| format!("(local.get $l{i})")).collect();
                let body = self.op(*op, &args, "(local.get $env)");
                let _ = writeln!(
                    self.out,
                    "(call $shadow_enter_call)\n(local.set $l{arity} {body})\n\
                     (call $shadow_leave) (local.get $l{arity}))"
                );
            }
        }
    }

    fn header(&mut self, name: &str, arity: u32, locals: &[LocalType]) {
        self.arities.insert(arity);
        let _ = write!(
            self.out,
            "(func {name} (type $fn{arity}) (param $clo (ref null $obj)) (param $env (ref null $env))"
        );
        for i in 0..arity {
            let _ = write!(self.out, " (param $l{i} eqref)");
        }
        self.out.push_str(" (result eqref)\n");
        for (i, t) in locals.iter().enumerate().skip(arity as usize) {
            let _ = write!(self.out, " (local $l{i} {})", local_type(*t));
        }
        self.out.push('\n');
    }

    fn code(&mut self, name: &str, f: &Func) {
        let _ = writeln!(self.out, ";; {}", f.name.replace('\n', " "));
        self.header(name, f.arity, &f.locals);
        if f.frame {
            // `$slot` keeps the slot of the frame for the frame records of
            // its points.
            self.out
                .push_str(" (local $slot i32)\n(call $shadow_enter_call)\n");
        }
        let mut body = String::new();
        self.stmts(&mut body, &f.body, f.frame);
        self.out.push_str(&body);
        self.out.push_str("(unreachable))\n");
    }

    fn stmts(&mut self, out: &mut String, stmts: &[Stmt], frame: bool) {
        for s in stmts {
            self.stmt(out, s, frame);
        }
    }

    fn stmt(&mut self, out: &mut String, s: &Stmt, frame: bool) {
        match s {
            Stmt::Let { dst, value, site } => {
                if let Some(site) = site {
                    let _ = writeln!(out, "(call $shadow_site (i32.const {}))", site_code(*site));
                }
                if let Value::Op(Op::Panic, args) = value {
                    let a = self.atom(&args[0]);
                    let _ = writeln!(out, "(call $panic {a}) (unreachable)");
                } else if let Value::Op(Op::Raise, args) = value {
                    let a = self.atom(&args[0]);
                    let _ = writeln!(out, "(throw $error {a})");
                } else {
                    let v = self.value(value);
                    let _ = writeln!(out, "(local.set $l{dst} {v})");
                }
            }
            Stmt::LetNum { dst, value, site } => {
                if let Some(site) = site {
                    let _ = writeln!(out, "(call $shadow_site (i32.const {}))", site_code(*site));
                }
                let v = self.num_value(value);
                let _ = writeln!(out, "(local.set $l{dst} {v})");
            }
            Stmt::ReadUtf {
                codepoint,
                width,
                ok,
                bits,
                start,
                encoding,
                little,
            } => {
                let b = self.atom(bits);
                let s = num(start);
                let _ = writeln!(
                    out,
                    "(call $bits_read_utf {b} {s} (i32.const {encoding}) (i32.const {}))\n\
                     (local.set $l{ok}) (local.set $l{width}) (local.set $l{codepoint})",
                    *little as i32
                );
            }
            Stmt::If {
                cond,
                then,
                otherwise,
            } => {
                let c = self.cond(cond);
                let _ = writeln!(out, "(if {c} (then");
                self.stmts(out, then, frame);
                out.push_str(") (else\n");
                self.stmts(out, otherwise, frame);
                out.push_str("))\n");
            }
            Stmt::Block { label, body } => {
                let _ = writeln!(out, "(block $b{label}");
                self.stmts(out, body, frame);
                out.push_str(")\n");
            }
            Stmt::Break(label) => {
                let _ = writeln!(out, "(br $b{label})");
            }
            Stmt::Return(a) => {
                let a = self.atom(a);
                if frame {
                    out.push_str("(call $shadow_leave)\n");
                }
                let _ = writeln!(out, "(return {a})");
            }
            Stmt::TailCall {
                callee,
                args,
                env,
                site,
            } => {
                if frame {
                    let s = site.map_or(-1, site_code);
                    let _ = writeln!(out, "(call $shadow_leave_tail (i32.const {s}))");
                }
                let call = self.call(callee, args, true, &env_text(*env));
                let _ = writeln!(out, "{call}");
            }
            Stmt::LetEnv {
                dst,
                env,
                service,
                value,
            } => {
                let v = self.atom(value);
                let _ = writeln!(
                    out,
                    "(local.set $l{dst} (call $env_with {} (i32.const {service}) {v}))",
                    env_text(*env)
                );
            }
            Stmt::Unreachable => out.push_str("(unreachable)\n"),
            Stmt::SetCapture {
                closure,
                index,
                value,
            } => {
                let v = self.atom(value);
                let _ = writeln!(
                    out,
                    "(array.set $values (ref.as_non_null (struct.get $obj 5 (ref.cast (ref $obj) (local.get $l{closure})))) (i32.const {index}) {v})"
                );
            }
            Stmt::SetGlobal(g, a) => {
                let a = self.atom(a);
                let _ = writeln!(out, "(global.set $g{} {a})", g.0);
            }
            Stmt::Suspendable {
                dst,
                value,
                site,
                point,
            } => self.suspendable(out, *dst, value, *site, *point, frame),
            Stmt::Continue(q) => self.continue_point(out, *q),
            Stmt::Join(j) => self.join_call(out, *j),
        }
    }

    fn call(&mut self, callee: &Callee, args: &[Atom], tail: bool, env: &str) -> String {
        let n = args.len() as u32;
        self.arities.insert(n);
        let args: String = args.iter().map(|a| format!(" {}", self.atom(a))).collect();
        let prefix = if tail { "return_" } else { "" };
        match callee {
            Callee::Direct(f) => {
                format!("({prefix}call $f{} (ref.null $obj) {env}{args})", f.0)
            }
            Callee::Value(a) => {
                let a = self.atom(a);
                format!(
                    "({prefix}call_ref $fn{n} (ref.cast (ref $clo{n}) {a}) {env}{args} \
                     (struct.get $clo{n} 8 (ref.cast (ref $clo{n}) {a})))"
                )
            }
        }
    }

    fn string(&mut self, s: &str) -> String {
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            return "(call $string_from_bytes (array.new_fixed $bytes 0))".into();
        }
        let next = self.strings.len();
        let id = *self.strings.entry(bytes.to_vec()).or_insert_with(|| {
            let mut text = String::new();
            for b in bytes {
                let _ = write!(text, "\\{b:02x}");
            }
            let _ = writeln!(self.data, "(data $s{next} \"{text}\")");
            next
        });
        format!(
            "(call $string_from_bytes (array.new_data $bytes $s{id} (i32.const 0) (i32.const {})))",
            bytes.len()
        )
    }

    fn atom(&mut self, a: &Atom) -> String {
        match a {
            Atom::Local(l) => format!("(local.get $l{l})"),
            Atom::Global(g) => format!("(global.get $g{})", g.0),
            Atom::Func(f) => format!("(global.get $c{})", f.0),
            Atom::Nil => "(ref.null eq)".into(),
            Atom::Bool(true) => "(global.get $value_true)".into(),
            Atom::Bool(false) => "(global.get $value_false)".into(),
            Atom::Int(n) => int(n),
            Atom::Float(x) => format!(
                "(struct.new $obj (i32.const 2) (i32.const 0) (i64.const 0) (f64.reinterpret_i64 (i64.const {})) \
                 (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call))",
                x.to_bits() as i64
            ),
            Atom::String(s) => self.string(s),
            // A constructor without fields is made once, in a global: its
            // values are equal however many there are (§11.4).
            Atom::Ctor { ctor, ty } => {
                self.ctors.insert((*ctor, *ty));
                format!("(global.get $k{ctor}_{ty})")
            }
            Atom::EmptyList => "(global.get $value_empty_list)".into(),
        }
    }

    fn atoms(&mut self, atoms: &[Atom]) -> String {
        atoms.iter().map(|a| format!(" {}", self.atom(a))).collect()
    }

    /// An operation; those that call i code get the environment `env`.
    fn op(&mut self, op: Op, args: &[String], env: &str) -> String {
        let a = args.first().cloned().unwrap_or_default();
        let b = args.get(1).cloned().unwrap_or_default();
        let compare_int =
            |c: &str| format!("(call $bool_new ({c} (call $int_compare {a} {b}) (i32.const 0)))");
        let compare_float =
            |c: &str| format!("(call $bool_new ({c} (call $float_get {a}) (call $float_get {b})))");
        match op {
            Op::IntAdd => format!("(call $int_add {a} {b})"),
            Op::IntSub => format!("(call $int_sub {a} {b})"),
            Op::IntMul => format!("(call $int_mul {a} {b})"),
            Op::IntDiv => format!("(call $int_div {a} {b})"),
            Op::IntRem => format!("(call $int_rem {a} {b})"),
            Op::IntNeg => format!("(call $int_neg {a})"),
            Op::IntLt => compare_int("i32.lt_s"),
            Op::IntLe => compare_int("i32.le_s"),
            Op::IntGt => compare_int("i32.gt_s"),
            Op::IntGe => compare_int("i32.ge_s"),
            Op::FloatAdd => format!("(call $float_add {a} {b})"),
            Op::FloatSub => format!("(call $float_sub {a} {b})"),
            Op::FloatMul => format!("(call $float_mul {a} {b})"),
            Op::FloatDiv => format!("(call $float_div {a} {b})"),
            Op::FloatNeg => format!("(call $float_neg {a})"),
            Op::FloatLt => compare_float("f64.lt"),
            Op::FloatLe => compare_float("f64.le"),
            Op::FloatGt => compare_float("f64.gt"),
            Op::FloatGe => compare_float("f64.ge"),
            Op::Equal => format!("(call $bool_new (call $equal {a} {b}))"),
            Op::NotEqual => format!("(call $bool_new (i32.eqz (call $equal {a} {b})))"),
            Op::StringAppend => format!("(call $string_append {a} {b})"),
            Op::Panic => format!("(call $panic {a}) (unreachable)"),
            Op::Runtime { name, .. } => format!("(call ${name} {})", args.join(" ")),
            Op::Raise => format!("(throw $error {a})"),
            Op::Catch => format!("(call $fail_catch {a} {env})"),
            Op::Ensure => format!("(call $resource_ensure {a} {b} {env})"),
            Op::Boundary => format!("(call $control_boundary {a} {env})"),
            Op::Suspend => format!("(call $control_suspend {a} {b})"),
            Op::Resume => format!("(call $control_resume {a} {b} {env})"),
            Op::Cancel => format!("(call $control_cancel {a} {env})"),
        }
    }

    fn value(&mut self, v: &Value) -> String {
        match v {
            Value::Atom(a) => self.atom(a),
            Value::Capture(i) => format!(
                "(array.get $values (ref.as_non_null (struct.get $obj 5 (ref.as_non_null (local.get $clo)))) (i32.const {i}))"
            ),
            Value::Call { callee, args, env } => self.call(callee, args, false, &env_text(*env)),
            Value::Op(op, args) => {
                let args: Vec<String> = args.iter().map(|a| self.atom(a)).collect();
                self.op(*op, &args, "(local.get $env)")
            }
            Value::OpEnv(op, args, env) => {
                let args: Vec<String> = args.iter().map(|a| self.atom(a)).collect();
                self.op(*op, &args, &env_text(*env))
            }
            Value::Service(env, id) => {
                format!("(array.get $env {} (i32.const {id}))", env_text(*env))
            }
            Value::Closure { func, captures } => {
                let n = self.p.funcs[func.0 as usize].arity();
                self.arities.insert(n);
                let caps = if captures.is_empty() {
                    "(ref.null $values)".to_string()
                } else {
                    format!(
                        "(array.new_fixed $values {}{})",
                        captures.len(),
                        self.atoms(captures)
                    )
                };
                closure(n, *func, &caps)
            }
            Value::Tuple(parts) => format!(
                "(call $tuple (array.new_fixed $values {}{}))",
                parts.len(),
                self.atoms(parts)
            ),
            Value::Record { ctor, ty, fields } => format!(
                "(call $record (i32.const {ctor}) (i64.const {ty}) (array.new_fixed $values {}{}))",
                fields.len(),
                self.atoms(fields)
            ),
            Value::Cons(h, t) => format!("(call $list_cons {} {})", self.atom(h), self.atom(t)),
            Value::Field(a, i) => format!("(call $field {} (i32.const {i}))", self.atom(a)),
            Value::ListHead(a) => format!("(call $list_head {})", self.atom(a)),
            Value::ListTail(a) => format!("(call $list_tail {})", self.atom(a)),
            Value::StringSuffix(a, n) => {
                format!("(call $string_suffix {} (i32.const {n}))", self.atom(a))
            }
            Value::BitsEmpty => "(call $bits_empty)".into(),
            Value::BitsConcat(a, b) => {
                format!("(call $bits_concat {} {})", self.atom(a), self.atom(b))
            }
            Value::BitsFromInt {
                value,
                width,
                little,
            } => format!(
                "(call $bits_from_int {} {} (i32.const {}))",
                self.atom(value),
                num(width),
                *little as i32
            ),
            Value::BitsFromFloat {
                value,
                width,
                little,
            } => format!(
                "(call $bits_from_float {} (i64.const {width}) (i32.const {}))",
                self.atom(value),
                *little as i32
            ),
            Value::BitsFromUtf {
                value,
                encoding,
                little,
                codepoint,
            } => format!(
                "(call $bits_from_utf {} (i32.const {encoding}) (i32.const {}) (i32.const {}))",
                self.atom(value),
                *little as i32,
                *codepoint as i32
            ),
            Value::BitsTake(a, n) => format!("(call $bits_take {} {})", self.atom(a), num(n)),
            Value::BitsWhole(a) => format!("(call $bits_whole {})", self.atom(a)),
            Value::BitsReadInt {
                bits,
                start,
                width,
                signed,
                little,
            } => format!(
                "(call $bits_read_int {} {} {} (i32.const {}) (i32.const {}))",
                self.atom(bits),
                num(start),
                num(width),
                *signed as i32,
                *little as i32
            ),
            Value::BitsReadFloat {
                bits,
                start,
                width,
                little,
            } => format!(
                "(call $bits_read_float {} {} (i64.const {width}) (i32.const {}))",
                self.atom(bits),
                num(start),
                *little as i32
            ),
            Value::BitsSlice {
                bits,
                start,
                length,
            } => format!(
                "(call $bits_slice {} {} {})",
                self.atom(bits),
                num(start),
                num(length)
            ),
            Value::HostCall {
                import,
                args,
                result,
            } => {
                let call = self.host_call(*import, args);
                let reply = self.host_reply(*import, *result);
                format!("(block (result eqref)\n{call}\n{reply})")
            }
            Value::HostStart { import, args } => self.host_start(*import, args),
            Value::HostReply(reply) => self.host_reply(reply.import, reply.result),
        }
    }

    fn num_value(&mut self, v: &NumValue) -> String {
        match v {
            NumValue::Num(n) => num(n),
            NumValue::Add(a, b) => format!("(i64.add {} {})", num(a), num(b)),
            NumValue::Sub(a, b) => format!("(i64.sub {} {})", num(a), num(b)),
            NumValue::BitsLength(a) => format!("(call $bits_len64 {})", self.atom(a)),
            NumValue::LengthOf(a, unit) => {
                format!("(call $bits_length {} (i64.const {unit}))", self.atom(a))
            }
            NumValue::TakeLength(a, unit) => {
                format!(
                    "(call $bits_take_length {} (i64.const {unit}))",
                    self.atom(a)
                )
            }
            NumValue::WidthOf(a, unit) => format!(
                "(call $bits_width {} (i64.const {unit}) (i32.const 0))",
                self.atom(a)
            ),
        }
    }

    fn cond(&mut self, c: &Cond) -> String {
        match c {
            Cond::True(a) => format!("(call $bool_get {})", self.atom(a)),
            Cond::Not(c) => format!("(i32.eqz {})", self.cond(c)),
            Cond::And(a, b) => format!(
                "(if (result i32) {} (then {}) (else (i32.const 0)))",
                self.cond(a),
                self.cond(b)
            ),
            Cond::Or(a, b) => format!(
                "(if (result i32) {} (then (i32.const 1)) (else {}))",
                self.cond(a),
                self.cond(b)
            ),
            Cond::Tag(a, t) => format!(
                "(i32.eq (call $value_tag {}) (i32.const {}))",
                self.atom(a),
                *t as i32
            ),
            Cond::Bool(a, b) => {
                format!("(call $is_bool {} (i32.const {}))", self.atom(a), *b as i32)
            }
            Cond::Ctor(a, c) => format!("(call $is_ctor {} (i32.const {c}))", self.atom(a)),
            Cond::Type(a, t) => format!("(call $is_type {} (i64.const {t}))", self.atom(a)),
            Cond::Tuple(a, n) => format!("(call $is_tuple {} (i32.const {n}))", self.atom(a)),
            Cond::Empty(a) => format!("(call $list_is_empty {})", self.atom(a)),
            Cond::Equal(a, b) => format!("(call $equal {} {})", self.atom(a), self.atom(b)),
            Cond::IsInt(a, n) => match i64::try_from(n) {
                // Small Ints are always `i31ref` (`runtime/mod.rs`).
                Ok(v) if (-(1 << 30)..(1 << 30)).contains(&v) => {
                    format!("(ref.eq {} (ref.i31 (i32.const {v})))", self.atom(a))
                }
                _ => format!("(call $is_int {} {})", self.atom(a), int(n)),
            },
            Cond::IsFloat(a, x) => format!(
                "(call $is_float {} (f64.reinterpret_i64 (i64.const {})))",
                self.atom(a),
                x.to_bits() as i64
            ),
            Cond::IsString(a, s) => {
                let s = self.string(s);
                format!("(call $is_string {} {s})", self.atom(a))
            }
            Cond::StartsWith(a, b) => {
                format!(
                    "(call $string_starts_with {} {})",
                    self.atom(a),
                    self.atom(b)
                )
            }
            Cond::I32(l) => format!("(local.get $l{l})"),
            Cond::NumLt(a, b) => format!("(i64.lt_s {} {})", num(a), num(b)),
            Cond::NumEq(a, b) => format!("(i64.eq {} {})", num(a), num(b)),
            Cond::Whole(a) => format!("(i64.eqz (i64.and {} (i64.const 7)))", num(a)),
            Cond::BitsHas(a, s, w) => {
                format!("(call $bits_has {} {} {})", self.atom(a), num(s), num(w))
            }
            Cond::BitsFloat {
                bits,
                start,
                width,
                little,
            } => format!(
                "(call $bits_float_finite {} {} (i64.const {width}) (i32.const {}))",
                self.atom(bits),
                num(start),
                *little as i32
            ),
            Cond::BitsEqual(a, b) => {
                format!("(call $bits_match_equal {} {})", self.atom(a), self.atom(b))
            }
        }
    }

    /// `ilang_start` (B.1): the site table, the constructor names, and the
    /// top-level values.
    fn start(&mut self) {
        let mut sites = String::new();
        for s in &self.p.sites {
            for n in [s.module, s.line, s.column] {
                for b in n.to_le_bytes() {
                    let _ = write!(sites, "\\{b:02x}");
                }
            }
        }
        let _ = writeln!(self.data, "(data $site_data \"{sites}\")");
        let modules: Vec<String> = self.p.site_modules.iter().map(|m| self.string(m)).collect();
        let names: Vec<String> = self.p.constructors.iter().map(|m| self.string(m)).collect();
        let _ = writeln!(
            self.out,
            "(func $ilang_start (export \"ilang_start\")\n\
             (call $sites_install (array.new_data $sites $site_data (i32.const 0) (i32.const {}))\n\
             (array.new_fixed $values {} {}))\n\
             (call $constructors_install (array.new_fixed $values {} {}))",
            self.p.sites.len() * 3,
            modules.len(),
            modules.join(" "),
            names.len(),
            names.join(" ")
        );
        // The root environment: the `Assert` of the runtime (§9.6), and
        // no other service yet.
        let _ = writeln!(
            self.out,
            "(global.set $root_env (array.new $env (ref.null eq) (i32.const {})))\n\
             (array.set $env (ref.as_non_null (global.get $root_env)) (i32.const {ASSERT_SERVICE})\n\
             (call $record (i32.const 3) (i64.const 2) (array.new_fixed $values 1\n\
             (struct.new $clo1 (i32.const 7) (i32.const 0) (i64.const 1) (f64.const 0) (ref.null $bytes)\n\
             (ref.null $values) (ref.null $limbs) (ref.null $call) (ref.func $entry_assert_fail)))))",
            self.p.services.max(1)
        );
        // §9.7: the host services that the entry points need, made of
        // closures that call the imports.
        for s in &self.p.host_services {
            let fields: String = s
                .fields
                .iter()
                .map(|f| format!(" (global.get $c{})", f.0))
                .collect();
            let _ = writeln!(
                self.out,
                "(array.set $env (ref.as_non_null (global.get $root_env)) (i32.const {})\n\
                 (call $record (i32.const {}) (i64.const {}) (array.new_fixed $values {}{fields})))",
                s.service,
                s.ctor,
                s.ty,
                s.fields.len()
            );
        }
        if self.p.start.is_some() {
            self.out
                .push_str("(drop (call $start (ref.null $obj) (global.get $root_env)))\n");
        }
        self.out.push_str(")\n");
    }

    /// The entry points (B.5). An error that comes to the program entry or a
    /// test stops the program with only the position of its binding (§11.7);
    /// one that comes to a host export is given to the host with its tag
    /// (B.2). An unwinding that comes to an entry point has lost its
    /// boundary.
    fn exports(&mut self) {
        let program: &'p Program = self.p;
        for e in &program.exports {
            let name = wat_string(&e.name);
            if let Some(body) = e.async_body {
                self.async_export(&name, e, body);
                continue;
            }
            match &e.abi {
                None => {
                    let call = self.call(&e.callee, &[], false, "(global.get $root_env)");
                    let site = e.site.map_or(-1, |s| s as i64);
                    let id = export_id(&e.name);
                    let _ = writeln!(
                        self.out,
                        "(func $entry_{id} (export \"{name}\") (local $e eqref)\n\
                         (local.set $e (block $error (result eqref)\n\
                         (block $cancel (result eqref)\n\
                         (drop (try_table (result eqref) (catch $error $error) (catch $cancel $cancel) {call}))\n\
                         (return))\n\
                         (drop) (call $stop_boundary_inactive) (unreachable)))\n\
                         (call $stop_error (local.get $e) (i32.const {site})) (unreachable))"
                    );
                }
                Some(abi) => self.host_export(&name, &e.callee, abi),
            }
        }
    }

    fn host_export(&mut self, name: &str, callee: &Callee, abi: &ExportAbi) {
        let (sig, converted) = export_params(abi);
        let n = abi.params.len() as u32;
        self.arities.insert(n);
        let env = "(global.get $root_env)";
        let call = match callee {
            Callee::Direct(f) => format!(
                "(call $f{} (ref.null $obj) {env} {})",
                f.0,
                converted.join(" ")
            ),
            Callee::Value(a) => {
                let a = self.atom(a);
                format!(
                    "(call_ref $fn{n} (ref.cast (ref $clo{n}) {a}) {env} {} \
                     (struct.get $clo{n} 8 (ref.cast (ref $clo{n}) {a})))",
                    converted.join(" ")
                )
            }
        };
        if abi.errors.is_empty() {
            let (types, out) = abi_out(abi.result, &call);
            let results = result_list(&types);
            let _ = writeln!(self.out, "(func (export \"{name}\"){sig}{results} {out})");
            return;
        }
        let parts = parts(abi);
        let results = result_list(&import_results(abi));
        let with = |at: usize, value: &str| with_value(&parts, at, value);
        // The error is given to the host, and the instance may be used
        // again: its shadow stack is put back (`runtime/mod.rs`).
        let mut f = format!(
            "(func (export \"{name}\"){sig}{results} (local $v eqref) (local $e eqref) (local $depth i32)\n\
             (local.set $depth (call $shadow_depth))\n\
             (local.set $e (block $error (result eqref)\n\
             (block $cancel (result eqref)\n\
             (local.set $v (try_table (result eqref) (catch $error $error) (catch $cancel $cancel) {call}))\n\
             (return (i32.const 0){}))\n\
             (drop) (call $stop_boundary_inactive) (unreachable)))\n\
             (call $shadow_restore (local.get $depth))\n",
            with(0, "(local.get $v)")
        );
        for (i, m) in abi.errors.iter().enumerate() {
            let _ = writeln!(
                f,
                "(if (i32.eq (call $value_tag (local.get $e)) (i32.const {})) (then (return (i32.const {}){})))",
                value_tag(*m),
                i + 1,
                with(i + 1, "(local.get $e)")
            );
        }
        f.push_str("(unreachable))\n");
        self.out.push_str(&f);
    }
}

/// The parameters of a host export: their Wasm types, and the values made
/// from them (B.2).
fn export_params(abi: &ExportAbi) -> (String, Vec<String>) {
    let mut sig = String::new();
    let mut converted = Vec::new();
    let mut k = 0;
    for p in &abi.params {
        let (types, convert) = abi_in(*p);
        if types.is_empty() {
            converted.push("(ref.null eq)".to_string());
            continue;
        }
        let mut gets = String::new();
        for t in types {
            let _ = write!(sig, " (param {t})");
            let _ = write!(gets, " (local.get {k})");
            k += 1;
        }
        if let AbiType::Externref(ty) = p {
            let _ = write!(gets, " (i32.const {ty})");
        }
        converted.push(format!("(call {convert}{gets})"));
    }
    (sig, converted)
}

/// The value and the members of the errors of a function (B.2).
fn parts(abi: &ExportAbi) -> Vec<AbiType> {
    std::iter::once(abi.result)
        .chain(abi.errors.iter().copied())
        .collect()
}

/// B.2: the value, then each member of the errors, with `value` at `at`;
/// the values that are not used are 0 or null.
fn with_value(parts: &[AbiType], at: usize, value: &str) -> String {
    let mut out = String::new();
    for (i, p) in parts.iter().enumerate() {
        if i == at {
            out.push(' ');
            // For `Nil`, the value is dropped: nothing is passed.
            out.push_str(&abi_out(*p, value).1);
        } else {
            for t in abi_in(*p).0 {
                out.push(' ');
                out.push_str(zero(t));
            }
        }
    }
    out
}

/// The Wasm types of a value (B.2) and the function that makes it from
/// them.
fn abi_in(t: AbiType) -> (&'static [&'static str], &'static str) {
    match t {
        AbiType::Nil => (&[], ""),
        AbiType::Bool => (&["i32"], "$abi_bool_in"),
        AbiType::Int => (&["i32", "i32", "i32"], "$abi_int_in"),
        AbiType::Float => (&["f64"], "$abi_float_in"),
        AbiType::String => (&["i32", "i32"], "$abi_string_in"),
        AbiType::BitArray => (&["i32", "i32"], "$abi_bits_in"),
        AbiType::Externref(_) => (&["externref"], "$abi_ref_in"),
    }
}

/// The Wasm types of a value given to the host, and the code that gives
/// them for the value `v`. For `Nil`, `v` is worked out and dropped.
fn abi_out(t: AbiType, v: &str) -> (Vec<&'static str>, String) {
    let types = abi_in(t).0.to_vec();
    let out = match t {
        AbiType::Nil => format!("(drop {v})"),
        AbiType::Bool => format!("(call $abi_bool_out {v})"),
        AbiType::Int => format!("(call $abi_int_out {v})"),
        AbiType::Float => format!("(call $abi_float_out {v})"),
        AbiType::String => format!("(call $abi_string_out {v})"),
        AbiType::BitArray => format!("(call $abi_bits_out {v})"),
        AbiType::Externref(_) => format!("(call $abi_ref_out {v})"),
    };
    (types, out)
}

fn result_list(types: &[&str]) -> String {
    if types.is_empty() {
        String::new()
    } else {
        format!(" (result {})", types.join(" "))
    }
}

/// The results of a function at the host boundary (B.2): the tag when
/// there are errors, the value, then each member of the errors.
fn import_results(abi: &ExportAbi) -> Vec<&'static str> {
    let mut out = Vec::new();
    if !abi.errors.is_empty() {
        out.push("i32");
    }
    out.extend(abi_in(abi.result).0);
    for e in &abi.errors {
        out.extend(abi_in(*e).0);
    }
    out
}

/// A string of the WAT text.
fn wat_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b == b'"' || b == b'\\' || !(0x20..0x7f).contains(&b) {
            let _ = write!(out, "\\{b:02x}");
        } else {
            out.push(b as char);
        }
    }
    out
}

fn zero(t: &str) -> &'static str {
    match t {
        "i32" => "(i32.const 0)",
        "f64" => "(f64.const 0)",
        _ => "(ref.null extern)",
    }
}

/// The value tag (`runtime/mod.rs`) of the values of an error member that
/// the host can take (§10.3).
fn value_tag(t: AbiType) -> u32 {
    match t {
        AbiType::Nil => 0,
        AbiType::Bool => 1,
        AbiType::Float => 2,
        AbiType::String => 3,
        AbiType::Int => 8,
        AbiType::BitArray => 9,
        AbiType::Externref(_) => 10,
    }
}

fn local_type(t: LocalType) -> &'static str {
    match t {
        LocalType::Value => "eqref",
        LocalType::I64 => "i64",
        LocalType::I32 => "i32",
        LocalType::Env => "(ref null $env)",
    }
}

fn env_text(e: Env) -> String {
    match e {
        Env::Param => "(local.get $env)".into(),
        Env::Local(l) => format!("(local.get $l{l})"),
    }
}

fn num(n: &Num) -> String {
    match n {
        Num::Local(l) => format!("(local.get $l{l})"),
        Num::Const(c) => format!("(i64.const {c})"),
    }
}

/// An Int literal (`runtime/mod.rs`: small numbers are `i31ref`).
fn int(n: &BigInt) -> String {
    if let Ok(v) = i64::try_from(n) {
        if (-(1 << 30)..(1 << 30)).contains(&v) {
            return format!("(ref.i31 (i32.const {v}))");
        }
        return format!("(call $int_from_i64 (i64.const {v}))");
    }
    let (sign, digits) = n.to_u32_digits();
    let sign = if sign == Sign::Minus { -1 } else { 1 };
    let limbs: String = digits
        .iter()
        .map(|d| format!(" (i32.const {})", *d as i32))
        .collect();
    format!(
        "(call $int_pack (i32.const {sign}) (array.new_fixed $limbs {}{limbs}))",
        digits.len()
    )
}

/// Appends the `ilang.abi` custom section (B.6).
pub fn add_abi_section(wasm: &mut Vec<u8>, json: &str) {
    let name = b"ilang.abi";
    let mut payload = Vec::new();
    leb(&mut payload, name.len() as u64);
    payload.extend_from_slice(name);
    payload.extend_from_slice(json.as_bytes());
    wasm.push(0);
    leb(wasm, payload.len() as u64);
    wasm.extend_from_slice(&payload);
}

fn leb(out: &mut Vec<u8>, mut n: u64) {
    loop {
        let byte = (n & 0x7f) as u8;
        n >>= 7;
        if n == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// The WAT id of the function of an entry point or test export: its name,
/// with every byte that an id may not have written as `_` and its hex.
fn export_id(name: &str) -> String {
    let mut id = String::new();
    for b in name.bytes() {
        if b.is_ascii_alphanumeric() {
            id.push(b as char);
        } else {
            let _ = write!(id, "_{b:02x}");
        }
    }
    id
}
