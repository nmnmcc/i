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
pub(crate) mod numeric;
mod wasi;

/// Support functions of the generated code, for tests in patterns.
const SUPPORT: &str = r#"
(func $is_data (param $v eqref) (param $set i32) (param $n i32) (result i32)
  (if (result i32) (i32.eq (call $value_tag (local.get $v)) (i32.const 4))
    (then (i32.and
      (i32.eq (struct.get $obj 1 (ref.cast (ref $obj) (local.get $v))) (local.get $set))
      (i32.eq (array.len (ref.as_non_null (struct.get $obj 5 (ref.cast (ref $obj) (local.get $v)))))
        (local.get $n))))
    (else (i32.const 0))))
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
  (call $is_data (local.get $v) (i32.const 0) (local.get $n)))
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
    "$keep_call",
    "$keep_reenter",
    "$request_handoff",
    "$wait_requests_reenter",
];

/// The WAT text of the whole module.
pub fn wat(p: &Program) -> String {
    wat_inner(p, false)
}

/// WAT for an isolated compiler macro run, including logical accounting.
pub fn wat_metered(p: &Program) -> (String, crate::budget::StaticCharge) {
    let raw = wat_inner(p, true);
    let cost = crate::budget::static_charge(&raw);
    (crate::budget::instrument(&raw), cost)
}

fn wat_inner(p: &Program, metered: bool) -> String {
    let mut e = Emitter {
        p,
        metered,
        numeric_enabled: false,
        numeric_plan: numeric::Plan::default(),
        worker: None,
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
        e.code("$start", start, None);
    }
    e.start();
    e.exports();
    e.resume_exports();
    let mut m = String::from("(module\n");
    for decision in &p.numeric_log {
        let _ = writeln!(m, ";; numeric: {}", decision.replace('\n', " "));
    }
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
             (type $clo{n} (sub final $obj (struct (field (mut i32)) (field i32) (field i64) (field f64) \
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
    if p.imports.iter().any(|x| x.kind == ImportKind::AsyncService) {
        m.push_str("(import \"ilang:host/v1\" \"cancel\" (func $host_cancel (param i32)))\n");
    }
    // The imports of B.4, before every definition.
    for (i, x) in p.imports.iter().enumerate() {
        if matches!(x.kind, ImportKind::WasiStdout | ImportKind::WasiAdapter) {
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
        m.push_str(&wasi::imports(p));
        m.push_str(&runtime::WASI_RUNTIME.replace(
            "(call $wasi_proc_exit",
            &format!("{} (call $wasi_proc_exit", wasi::teardown(p)),
        ));
        m.push_str(&wasi::runtime(p));
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
                "(func (export \"_start\") (call $ilang_start) {} (call $entry_{}) {})",
                wasi::start(p),
                export_id(entry),
                wasi::teardown(p)
            );
        }
    }
    if !p.imports.iter().any(|x| x.kind == ImportKind::AsyncService) {
        m.push_str("(func $host_cancel (param i32) unreachable)\n");
    }
    m.push_str(runtime::wat_for_expansion_sites(
        references,
        p.site_notes.iter().any(Option::is_some),
    ));
    m.push_str(SUPPORT);
    m.push_str(control::EFFECTS);
    m.push_str(&host::effects());
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
            closure(n, f, "(ref.null $values)", false)
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

fn closure(arity: u32, f: FuncId, captures: &str, patched: bool) -> String {
    let flags = u32::from(patched);
    format!(
        "(struct.new $clo{arity} (i32.const 7) (i32.const {flags}) (i64.const {arity}) (f64.const 0) \
         (ref.null $bytes) {captures} (ref.null $limbs) (ref.null $call) (ref.func $f{}))",
        f.0
    )
}

struct Emitter<'p> {
    p: &'p Program,
    metered: bool,
    numeric_enabled: bool,
    numeric_plan: numeric::Plan,
    worker: Option<crate::numeric::Worker>,
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
            FuncKind::Code(func) => self.code(&name, func, Some(id)),
            // A constructor or a library function used as a value has a
            // frame too, so that it takes the tail site of a tail call
            // (§11.7).
            FuncKind::Ctor { ctor, ty, arity } => {
                let locals = vec![LocalType::Value; *arity as usize + 1];
                self.header(&name, *arity, &locals);
                if self.metered {
                    let _ = writeln!(self.out, "(call $macro_enter (i32.const {}))", arity + 1);
                }
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
                if self.metered {
                    let _ = writeln!(self.out, "(call $macro_enter (i32.const {}))", arity + 1);
                }
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
        if self.worker.is_some() {
            let _ = write!(
                self.out,
                "(func {name} (param $clo (ref null $obj)) (param $env (ref null $env))"
            );
        } else {
            let _ = write!(
                self.out,
                "(func {name} (type $fn{arity}) (param $clo (ref null $obj)) (param $env (ref null $env))"
            );
        }
        for i in 0..arity {
            if self.worker.as_ref().is_some_and(|w| w.floats[i as usize]) {
                let _ = write!(self.out, " (param $numeric_f{i} f64)");
            } else if self.worker.as_ref().is_some_and(|w| w.ints[i as usize]) {
                let _ = write!(self.out, " (param $numeric_i{i} i64)");
            } else {
                let _ = write!(self.out, " (param $l{i} eqref)");
            }
        }
        self.out
            .push_str(if self.worker.as_ref().is_some_and(|w| w.float_result) {
                " (result f64)\n"
            } else {
                " (result eqref)\n"
            });
        if let Some(worker) = &self.worker {
            for (i, scalar) in worker
                .floats
                .iter()
                .zip(&worker.ints)
                .map(|(f, i)| *f || *i)
                .enumerate()
            {
                if scalar {
                    let _ = writeln!(self.out, " (local $l{i} eqref)");
                }
            }
        }
        for (i, t) in locals.iter().enumerate().skip(arity as usize) {
            let _ = write!(self.out, " (local $l{i} {})", local_type(*t));
        }
        self.out.push('\n');
    }

    fn code(&mut self, name: &str, f: &Func, id: Option<FuncId>) {
        let _ = writeln!(self.out, ";; {}", f.name.replace('\n', " "));
        self.worker = id.and_then(|id| self.p.workers.get(&id)).cloned();
        self.header(name, f.arity, &f.locals);
        self.numeric_plan = if self.p.numeric == crate::Numeric::Scalars && !self.metered {
            numeric::Plan::for_func(
                f,
                id.and_then(|id| self.p.numeric_locals.get(&id)),
                self.worker.as_ref(),
            )
        } else {
            numeric::Plan::default()
        };
        let mut declarations = self.numeric_plan.declarations();
        if let Some(worker) = &self.worker {
            for (i, float) in worker.floats.iter().enumerate() {
                if *float {
                    declarations =
                        declarations.replace(&format!(" (local $numeric_f{i} f64)\n"), "");
                }
            }
        }
        if let Some(worker) = &self.worker {
            for (i, int) in worker.ints.iter().enumerate() {
                if *int {
                    declarations =
                        declarations.replace(&format!(" (local $numeric_i{i} i64)\n"), "");
                }
            }
        }
        self.out.push_str(&declarations);
        self.numeric_enabled =
            self.p.numeric != crate::Numeric::Reference && numeric::has_float(&f.body);
        if self.numeric_enabled {
            self.out.push_str(
                " (local $numeric_a f64) (local $numeric_b f64) (local $numeric_result f64)\n",
            );
        }
        if f.frame {
            // `$slot` keeps the slot of the frame for the frame records of
            // its points.
            self.out.push_str(" (local $slot i32)\n");
            if self.metered {
                let _ = writeln!(
                    self.out,
                    "(call $macro_enter (i32.const {}))",
                    f.budget_frame
                );
            }
            self.out.push_str("(call $shadow_enter_call)\n");
        }
        let mut parameters = self.numeric_plan.parameters(f.arity);
        if let Some(worker) = &self.worker {
            for (i, float) in worker.floats.iter().enumerate() {
                if *float {
                    parameters = parameters.replace(
                        &format!(
                            "(local.set $numeric_f{i} {})\n",
                            numeric::unbox_float(&format!("(local.get $l{i})"))
                        ),
                        "",
                    );
                }
            }
        }
        self.out.push_str(&parameters);
        let mut body = String::new();
        self.stmts(&mut body, &f.body, f.frame);
        self.out.push_str(&body);
        self.out.push_str("(unreachable))\n");
        self.numeric_enabled = false;
        self.worker = None;
        self.numeric_plan = numeric::Plan::default();
    }

    fn stmts(&mut self, out: &mut String, stmts: &[Stmt], frame: bool) {
        enum Action<'a> {
            Stmt(&'a Stmt),
            Text(&'static str),
        }
        let mut work: Vec<_> = stmts.iter().rev().map(Action::Stmt).collect();
        while let Some(action) = work.pop() {
            match action {
                Action::Text(text) => out.push_str(text),
                Action::Stmt(Stmt::If {
                    cond,
                    then,
                    otherwise,
                }) => {
                    if self.metered {
                        out.push_str("(call $macro_charge (i64.const 1) (i64.const 0))\n");
                    }
                    let c = self.cond(cond);
                    let _ = writeln!(out, "(if {c} (then");
                    work.push(Action::Text("))\n"));
                    work.extend(otherwise.iter().rev().map(Action::Stmt));
                    work.push(Action::Text(") (else\n"));
                    work.extend(then.iter().rev().map(Action::Stmt));
                }
                Action::Stmt(Stmt::Block { label, body }) => {
                    if self.metered {
                        out.push_str("(call $macro_charge (i64.const 1) (i64.const 0))\n");
                    }
                    let _ = writeln!(out, "(block $b{label}");
                    work.push(Action::Text(")\n"));
                    work.extend(body.iter().rev().map(Action::Stmt));
                }
                Action::Stmt(stmt) => self.stmt(out, stmt, frame),
            }
        }
    }

    fn stmt(&mut self, out: &mut String, s: &Stmt, frame: bool) {
        if self.metered {
            out.push_str("(call $macro_charge (i64.const 1) (i64.const 0))\n");
        }
        match s {
            Stmt::Logical(action) => match action {
                Logical::Enter(site) => {
                    if let Some(site) = site {
                        let _ =
                            writeln!(out, "(call $shadow_site (i32.const {}))", site_code(*site));
                    }
                    out.push_str("(call $shadow_enter_call)\n");
                }
                Logical::Leave => out.push_str("(call $shadow_leave)\n"),
                Logical::Tail(site) => {
                    let _ = writeln!(
                        out,
                        "(call $shadow_leave_tail (i32.const {}))",
                        site.map_or(-1, site_code)
                    );
                }
            },
            Stmt::Let { dst, value, site } => {
                if let Some(site) = site {
                    let _ = writeln!(out, "(call $shadow_site (i32.const {}))", site_code(*site));
                }
                if let Some(assignment) = self.scalar_assignment(*dst, value) {
                    out.push_str(&assignment);
                } else if let Value::Op(Op::Panic, args) = value {
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
            Stmt::If { .. } | Stmt::Block { .. } => {
                unreachable!("structured statements are scheduled by stmts")
            }
            Stmt::Break(label) => {
                let _ = writeln!(out, "(br $b{label})");
            }
            Stmt::Return(a) => {
                let a = if self.worker.as_ref().is_some_and(|w| w.float_result) {
                    self.float_atom(a)
                } else {
                    self.atom(a)
                };
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
        let target = match callee {
            Callee::Direct(f) | Callee::Known { func: f, .. } => Some(*f),
            _ => None,
        };
        let worker = target.and_then(|f| self.p.workers.get(&f)).cloned();
        let prefix = if tail { "return_" } else { "" };
        let closure = match callee {
            Callee::Known { closure, .. } => {
                format!("(ref.cast (ref null $obj) {})", self.atom(closure))
            }
            _ => "(ref.null $obj)".into(),
        };
        let mut guards = Vec::new();
        let raw_args = args
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let raw = if worker.as_ref().is_some_and(|w| w.floats[i]) {
                    self.float_atom(a)
                } else if worker.as_ref().is_some_and(|w| w.ints[i]) {
                    let (guard, raw) = self.small_int(a);
                    guards.push(guard);
                    raw
                } else {
                    self.atom(a)
                };
                format!(" {raw}")
            })
            .collect::<String>();
        let call = match callee {
            Callee::Direct(f) | Callee::Known { func: f, .. } => {
                format!("({prefix}call $f{} {closure} {env}{raw_args})", f.0)
            }
            Callee::Value(a) => {
                let a = self.atom(a);
                format!(
                    "({prefix}call_ref $fn{n} (ref.cast (ref $clo{n}) {a}) {env}{raw_args} (struct.get $clo{n} 8 (ref.cast (ref $clo{n}) {a})))"
                )
            }
        };
        if guards.is_empty() {
            return call;
        }
        let worker = worker.expect("integer guards have a worker");
        assert!(
            !worker.float_result,
            "guarded Int fallback has Value result"
        );
        let guard = guards
            .into_iter()
            .reduce(|a, b| format!("(i32.and {a} {b})"))
            .unwrap();
        let canonical = args
            .iter()
            .map(|a| format!(" {}", self.atom(a)))
            .collect::<String>();
        let result = if tail { "" } else { " (result eqref)" };
        format!(
            "(if{result} {guard} (then {call}) (else ({prefix}call $f{} {closure} {env}{canonical})))",
            worker.origin.0
        )
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
            Atom::CurrentClosure => "(local.get $clo)".into(),
            Atom::Local(l) => match self.numeric_plan.locals.get(l) {
                Some(numeric::Repr::Float) => {
                    // Reuse incoming/canonical boxes. A purely scalar definition
                    // materializes once, at its first reference escape.
                    let boxed = numeric::box_float(&format!("(local.get $numeric_f{l})"));
                    format!(
                        "(block (result eqref) (if (ref.is_null (local.get $l{l})) (then (local.set $l{l} {boxed}))) (local.get $l{l}))"
                    )
                }
                Some(numeric::Repr::Int) => format!(
                    "(if (result eqref) (ref.is_null (local.get $l{l})) (then (ref.i31 (i32.wrap_i64 (local.get $numeric_i{l})))) (else (local.get $l{l})))"
                ),
                Some(numeric::Repr::Bool) => format!("(call $bool_new (local.get $numeric_c{l}))"),
                None => format!("(local.get $l{l})"),
            },
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
            Op::KeepEnv => format!("(call $control_keep_env {a} {env})"),
            Op::WaitRequests => format!("(call $request_wait_any {a} {env})"),
            Op::PollRequests => {
                if self.p.target == Target::Wasi {
                    "(ref.null eq)".to_string()
                } else {
                    format!("(call $request_poll {env})")
                }
            }
        }
    }

    fn value(&mut self, v: &Value) -> String {
        match v {
            Value::Atom(a) => self.atom(a),
            Value::Capture(i) => format!(
                "(array.get $values (ref.as_non_null (struct.get $obj 5 (ref.as_non_null (local.get $clo)))) (i32.const {i}))"
            ),
            Value::Call { callee, args, env } => {
                let raw = self.call(callee, args, false, &env_text(*env));
                if self.float_call(callee) {
                    numeric::box_float(&raw)
                } else {
                    raw
                }
            }
            Value::Finite(op, args) => {
                let args = args.iter().map(|a| self.float_atom(a)).collect::<Vec<_>>();
                numeric::box_float(
                    &numeric::float_operation(*op, &args, false).expect("proved Float operation"),
                )
            }
            Value::Op(op, args) => {
                if self.numeric_enabled
                    && matches!(
                        op,
                        Op::FloatAdd | Op::FloatSub | Op::FloatMul | Op::FloatDiv | Op::FloatNeg
                    )
                {
                    let args: Vec<_> = args.iter().map(|a| self.float_atom(a)).collect();
                    return numeric::box_float(
                        &numeric::float_op(*op, &args).expect("float operation"),
                    );
                }
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
            Value::Closure {
                func,
                captures,
                patched,
            } => {
                let n = self.p.funcs[func.0 as usize].arity();
                self.arities.insert(n);
                let caps = if captures.is_empty() && !patched {
                    "(ref.null $values)".to_string()
                } else {
                    format!(
                        "(array.new_fixed $values {}{}{})",
                        captures.len() + usize::from(*patched),
                        self.atoms(captures),
                        if *patched { " (ref.null eq)" } else { "" }
                    )
                };
                closure(n, *func, &caps, *patched)
            }
            Value::Tuple(parts) => format!(
                "(call $tuple (array.new_fixed $values {}{}))",
                parts.len(),
                self.atoms(parts)
            ),
            Value::Data(set, parts) => format!(
                "(call $data (i32.const {set}) (array.new_fixed $values {}{}))",
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
            Value::StringSuffix(a, n, z) => {
                format!(
                    "(call $string_trim_affixes {} (i32.const {n}) (i32.const {z}))",
                    self.atom(a)
                )
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
                if self.p.imports[*import as usize].kind == ImportKind::WasiAdapter {
                    return wasi::call(
                        &self.p.imports[*import as usize],
                        &args.iter().map(|a| self.atom(a)).collect::<Vec<_>>(),
                    );
                }
                let call = self.host_call(*import, args);
                let reply = self.host_reply(*import, *result);
                format!("(block (result eqref)\n{call}\n{reply})")
            }
            Value::HostStart {
                import,
                args,
                result,
            } => self.host_start(*import, args, *result),
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
        enum Action<'a> {
            Cond(&'a Cond),
            Text(&'static str),
        }
        let mut out = String::new();
        let mut work = vec![Action::Cond(c)];
        while let Some(action) = work.pop() {
            match action {
                Action::Text(text) => out.push_str(text),
                Action::Cond(Cond::Not(c)) => {
                    out.push_str("(i32.eqz ");
                    work.push(Action::Text(")"));
                    work.push(Action::Cond(c));
                }
                Action::Cond(Cond::And(a, b)) => {
                    out.push_str("(if (result i32) ");
                    work.push(Action::Text(") (else (i32.const 0)))"));
                    work.push(Action::Cond(b));
                    work.push(Action::Text(" (then "));
                    work.push(Action::Cond(a));
                }
                Action::Cond(Cond::Or(a, b)) => {
                    out.push_str("(if (result i32) ");
                    work.push(Action::Text("))"));
                    work.push(Action::Cond(b));
                    work.push(Action::Text(" (then (i32.const 1)) (else "));
                    work.push(Action::Cond(a));
                }
                Action::Cond(leaf) => out.push_str(&self.cond_leaf(leaf)),
            }
        }
        out
    }

    fn cond_leaf(&mut self, c: &Cond) -> String {
        if let Cond::True(Atom::Local(local)) | Cond::Bool(Atom::Local(local), _) = c
            && self.numeric_plan.locals.get(local) == Some(&numeric::Repr::Bool)
        {
            return if matches!(c, Cond::Bool(_, false)) {
                format!("(i32.eqz (local.get $numeric_c{local}))")
            } else {
                format!("(local.get $numeric_c{local})")
            };
        }
        match c {
            Cond::True(a) => format!("(call $bool_get {})", self.atom(a)),
            Cond::Not(_) | Cond::And(_, _) | Cond::Or(_, _) => {
                unreachable!("compound conditions are scheduled by cond")
            }
            Cond::Tag(a, t) => format!(
                "(i32.eq (call $value_tag {}) (i32.const {}))",
                self.atom(a),
                *t as i32
            ),
            Cond::Data(a, set, n) => format!(
                "(call $is_data {} (i32.const {set}) (i32.const {n}))",
                self.atom(a)
            ),
            Cond::Bool(a, b) => {
                format!("(call $is_bool {} (i32.const {}))", self.atom(a), *b as i32)
            }
            Cond::Ctor(a, c) => format!("(call $is_ctor {} (i32.const {c}))", self.atom(a)),
            Cond::Type(a, t) => format!("(call $is_type {} (i64.const {t}))", self.atom(a)),
            Cond::Tuple(a, n) => format!("(call $is_tuple {} (i32.const {n}))", self.atom(a)),
            Cond::Empty(a) => format!("(call $list_is_empty {})", self.atom(a)),
            Cond::Equal(a, b) => format!("(call $equal {} {})", self.atom(a), self.atom(b)),
            Cond::IsInt(a, n)
                if self.numeric_plan.locals.get(match a {
                    Atom::Local(l) => l,
                    _ => &u32::MAX,
                }) == Some(&numeric::Repr::Int)
                    && i64::try_from(n).is_ok_and(|n| (-1073741824..=1073741823).contains(&n)) =>
            {
                let (guard, raw) = self.small_int(a);
                format!(
                    "(i32.and {guard} (i64.eq {raw} (i64.const {})))",
                    i64::try_from(n).unwrap()
                )
            }
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
            Cond::StartsWith(a, b, c) => {
                format!(
                    "(call $string_matches_affixes {} {} {})",
                    self.atom(a),
                    self.atom(b),
                    self.atom(c)
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
        let _ = writeln!(self.out, "(func $ilang_start (export \"ilang_start\")");
        let shapes: Vec<String> = self
            .p
            .shapes
            .clone()
            .iter()
            .map(|labels| {
                let labels: Vec<String> = labels.iter().map(|l| self.string(l)).collect();
                format!(
                    "(array.new_fixed $values {} {})",
                    labels.len(),
                    labels.join(" ")
                )
            })
            .collect();
        let _ = writeln!(
            self.out,
            "(call $shapes_install (array.new_fixed $values {} {}))",
            shapes.len(),
            shapes.join(" ")
        );
        let _ = writeln!(
            self.out,
            "(call $sites_install (array.new_data $sites $site_data (i32.const 0) (i32.const {}))\n\
             (array.new_fixed $values {} {}))\n\
             (call $constructors_install (array.new_fixed $values {} {}))",
            self.p.sites.len() * 3,
            modules.len(),
            modules.join(" "),
            names.len(),
            names.join(" ")
        );
        if self.p.site_notes.iter().any(Option::is_some) {
            assert_eq!(self.p.site_notes.len(), self.p.sites.len());
            let notes: Vec<String> = self
                .p
                .site_notes
                .iter()
                .map(|note| self.string(note.as_deref().unwrap_or("")))
                .collect();
            let _ = writeln!(
                self.out,
                "(call $site_notes_install (array.new_fixed $values {} {}))",
                notes.len(),
                notes.join(" ")
            );
        }
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
                         (drop) (drop (call $request_entry_end)) (call $stop_boundary_inactive) (unreachable)))\n\
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
            Callee::Known { func, closure } => {
                let closure = self.atom(closure);
                format!(
                    "(call $f{} (ref.cast (ref null $obj) {closure}) {env} {})",
                    func.0,
                    converted.join(" ")
                )
            }
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
             (drop) (drop (call $request_entry_end)) (call $stop_boundary_inactive) (unreachable)))\n\
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
        Env::Root => "(global.get $root_env)".into(),
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
    add_custom_section(wasm, "ilang.abi", json.as_bytes());
}

pub(crate) fn add_json_section(wasm: &mut Vec<u8>, name: &str, json: &str) {
    add_custom_section(wasm, name, json.as_bytes());
}

pub fn add_custom_section(wasm: &mut Vec<u8>, name: &str, bytes: &[u8]) {
    let name = name.as_bytes();
    let mut payload = Vec::new();
    leb(&mut payload, name.len() as u64);
    payload.extend_from_slice(name);
    payload.extend_from_slice(bytes);
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

#[cfg(test)]
mod deep_emit_tests {
    use super::*;

    #[test]
    fn nested_conditions_and_blocks_emit_on_a_small_stack() {
        std::thread::Builder::new().stack_size(256 * 1024).spawn(|| {
            let program = Program::default();
            let mut emitter = Emitter { p:&program, metered:false, numeric_enabled:false, numeric_plan:numeric::Plan::default(), worker:None, arities:BTreeSet::new(), ctors:BTreeSet::new(), strings:HashMap::new(), data:String::new(), out:String::new() };
            let leaf = Cond::I32(1);
            let combined = Cond::And(Box::new(Cond::I32(1)), Box::new(Cond::Or(Box::new(Cond::I32(2)),Box::new(Cond::I32(3)))));
            assert_eq!(emitter.cond(&combined), "(if (result i32) (local.get $l1) (then (if (result i32) (local.get $l2) (then (i32.const 1)) (else (local.get $l3)))) (else (i32.const 0)))");
            let mut cond = leaf;
            for _ in 0..10_000 { cond = Cond::Not(Box::new(cond)); }
            let rendered = emitter.cond(&cond);
            assert_eq!(rendered.matches("(i32.eqz ").count(), 10_000);
            let mut stmt = Stmt::Return(Atom::Nil);
            for n in 0..10_000 { stmt = Stmt::Block { label:n, body:vec![stmt] }; }
            let mut rendered = String::new();
            emitter.stmts(&mut rendered, std::slice::from_ref(&stmt), false);
            assert_eq!(rendered.matches("(block $b").count(), 10_000);
            drop(cond);
            drop(stmt);
        }).unwrap().join().unwrap();
    }
}
