//! The host boundary in the generated module (§9.7, §10.2, B.4, B.5):
//! import calls and their answers, async entry points, and the
//! `ilang_resume:…` exports. It calls control (`control.rs`) only through
//! `$host_boundary` and `$host_resume_k`.

use super::*;

/// The runtime functions of the host boundary (§9.7, B.5).
pub(super) const EFFECTS: &str = r#"
;; # The host boundary (§9.7, B.4, B.5)
;;
;; An async entry point runs in a root boundary, `$host_root`. A function of
;; an async host service starts the import, records it in `$host_pending`,
;; and suspends to the root boundary, where `$host_wait` keeps the
;; continuation in `$host_k` and gives `$host_waiting`: the status is 1. The
;; export `ilang_resume:…` of the import puts the answer in the globals of the
;; import and resumes the continuation, so the function reads the answer
;; with the frames of the program on the shadow stack (§11.7).
(global $host_root (mut (ref null $boundary)) (ref.null $boundary))
(global $host_k (mut eqref) (ref.null eq))
(global $host_pending (mut i32) (i32.const -1))
(global $host_waiting eqref (struct.new $waiting (i32.const 0)))
;; The async entry point that runs: whether it is a host export, which gives
;; its errors to the host, and the site of its binding (§11.7).
(global $async_export (mut i32) (i32.const 0))
(global $async_site (mut i32) (i32.const -1))
;; How it ended: its value or error, and 1 for an error.
(global $async_value (mut eqref) (ref.null eq))
(global $async_error (mut i32) (i32.const 0))

(func $host_wait (param $k eqref) (result eqref)
  (global.set $host_k (local.get $k))
  (global.get $host_waiting))

(func $host_root_boundary (result eqref)
  (if (ref.is_null (global.get $host_root)) (then unreachable))
  (global.get $host_root))

(func $host_boundary (param $body eqref) (param $env (ref null $env)) (result eqref)
  (local $b (ref $boundary))
  (local.set $b (struct.new $boundary (i32.const 11) (i32.const 0) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call) (i32.const 1)))
  (global.set $host_root (local.get $b))
  (return_call $boundary_run (local.get $b) (local.get $env) (local.get $body) (ref.null $flist)
    (ref.null eq) (ref.null eq) (ref.null $frame)))

(func $host_resume_k (result eqref)
  (local $k eqref)
  (global.set $host_pending (i32.const -1))
  (local.set $k (global.get $host_k))
  (global.set $host_k (ref.null eq))
  (return_call $control_resume (local.get $k) (ref.null eq) (global.get $root_env)))

;; The start of an async entry point. While one waits, the host may only
;; answer it (B.5).
(func $async_begin (param $export i32) (param $site i32)
  (if (i32.ne (global.get $host_pending) (i32.const -1)) (then unreachable))
  (global.set $async_export (local.get $export))
  (global.set $async_site (local.get $site))
  (global.set $async_value (ref.null eq))
  (global.set $async_error (i32.const 0)))

;; `ilang_result:…` may only be called when the entry point has ended.
(func $async_result
  (if (i32.ne (global.get $host_pending) (i32.const -1)) (then unreachable)))

;; Runs the body of an async entry point in a new root boundary, or, when
;; `body` is null, goes on with it after the host answered. Gives the status
;; of B.5: 1 while it waits, 0 when it has ended.
(func $async_run (param $body eqref) (result i32)
  (local $r eqref) (local $e eqref) (local $depth i32)
  (local.set $depth (call $shadow_depth))
  (local.set $e (block $error (result eqref)
    (block $cancel (result eqref)
      (local.set $r (try_table (result eqref) (catch $error $error) (catch $cancel $cancel)
        (if (result eqref) (ref.is_null (local.get $body))
          (then (call $host_resume_k))
          (else (call $host_boundary (local.get $body) (global.get $root_env))))))
      (if (ref.eq (local.get $r) (global.get $host_waiting)) (then (return (i32.const 1))))
      (global.set $async_value (local.get $r))
      (return (i32.const 0)))
    (drop) (call $stop_boundary_inactive) (unreachable)))
  ;; An error stops the program entry and tests; a host export gives it to
  ;; the host (§10.2).
  (if (i32.eqz (global.get $async_export))
    (then (call $stop_error (local.get $e) (global.get $async_site)) (unreachable)))
  (call $shadow_restore (local.get $depth))
  (global.set $async_value (local.get $e))
  (global.set $async_error (i32.const 1))
  (i32.const 0))
"#;

/// The type of the status "waiting" (B.5), after the types of control.
pub(super) const TYPES: &str = "(type $waiting (struct (field i32)))\n";

impl<'p> Emitter<'p> {
    /// Starts an import of an async host service (B.4): the program then
    /// waits for this import.
    pub(super) fn host_start(&mut self, import: ImportId, args: &[Atom]) -> String {
        let call = self.host_call(import, args);
        format!(
            "(block (result eqref)\n\
             (if (i32.ne (global.get $host_pending) (i32.const -1)) (then unreachable))\n\
             {call}\n(global.set $host_pending (i32.const {import})) (ref.null eq))"
        )
    }

    /// Calls an import (B.2, B.3): the arguments are encoded in new buffers
    /// through the globals `$hp…`, the results go to the globals `$hr…`, and
    /// the buffers of the arguments are freed.
    pub(super) fn host_call(&mut self, i: ImportId, args: &[Atom]) -> String {
        let program: &'p Program = self.p;
        let x = &program.imports[i as usize];
        let mut code = String::new();
        let mut k = 0;
        let mut frees = Vec::new();
        for (t, a) in x.abi.params.iter().zip(args) {
            let n = abi_in(*t).0.len();
            if n == 0 {
                continue;
            }
            let (_, out) = abi_out(*t, &self.atom(a));
            code.push_str(&out);
            for j in (k..k + n).rev() {
                let _ = write!(code, " (global.set $hp{i}_{j})");
            }
            code.push('\n');
            match t {
                AbiType::Int => frees.push(k + 1),
                AbiType::String | AbiType::BitArray => frees.push(k),
                _ => {}
            }
            k += n;
        }
        let gets: String = (0..k)
            .map(|j| format!(" (global.get $hp{i}_{j})"))
            .collect();
        let _ = write!(code, "(call $imp{i}{gets})");
        if x.kind != ImportKind::AsyncService {
            for j in (0..import_results(&x.abi).len()).rev() {
                let _ = write!(code, " (global.set $hr{i}_{j})");
            }
        }
        for j in frees {
            let _ = write!(code, "\n(call $free (global.get $hp{i}_{j}) (i32.const 0))");
        }
        code
    }

    /// The value of the results of an import in the globals `$hr…` (B.2):
    /// the value, or the error thrown. The result buffers are freed (B.3);
    /// a value that is not correct stops.
    pub(super) fn host_reply(&self, i: ImportId) -> String {
        let x = &self.p.imports[i as usize];
        let take = |t: AbiType, k: usize| -> String {
            let g = |j: usize| format!("(global.get $hr{i}_{})", k + j);
            match t {
                AbiType::Nil => "(ref.null eq)".to_string(),
                AbiType::Bool => format!("(call $abi_bool_in {})", g(0)),
                AbiType::Float => format!("(call $abi_float_in {})", g(0)),
                AbiType::Int => format!("(call $abi_int_take {} {} {})", g(0), g(1), g(2)),
                AbiType::String => format!("(call $abi_string_take {} {})", g(0), g(1)),
                AbiType::BitArray => format!("(call $abi_bits_take {} {})", g(0), g(1)),
                AbiType::Externref => format!("(call $abi_ref_in {})", g(0)),
            }
        };
        if x.abi.errors.is_empty() {
            return take(x.abi.result, 0);
        }
        let mut offsets = Vec::new();
        let mut k = 1;
        for t in std::iter::once(x.abi.result).chain(x.abi.errors.iter().copied()) {
            offsets.push((t, k));
            k += abi_in(t).0.len();
        }
        let n = x.abi.errors.len();
        let mut chain = format!("(throw $error {})", take(offsets[n].0, offsets[n].1));
        for m in (1..n).rev() {
            chain = format!(
                "(if (result eqref) (i32.eq (global.get $hr{i}_0) (i32.const {m}))\n\
                 (then (throw $error {})) (else {chain}))",
                take(offsets[m].0, offsets[m].1)
            );
        }
        format!(
            "(block (result eqref)\n\
             (global.set $hr{i}_0 (call $abi_tag_in (global.get $hr{i}_0) (i32.const {n})))\n\
             (if (result eqref) (i32.eqz (global.get $hr{i}_0)) (then {}) (else {chain})))",
            take(offsets[0].0, offsets[0].1)
        )
    }

    /// An async entry point (B.5): it runs in a root boundary and gives a
    /// status; `ilang_result:name` gives its results when it has ended.
    pub(super) fn async_export(&mut self, name: &str, e: &Export, body: FuncId) {
        let (sig, converted) = match &e.abi {
            Some(abi) => export_params(abi),
            None => (String::new(), Vec::new()),
        };
        let captures = if converted.is_empty() {
            "(ref.null $values)".to_string()
        } else {
            format!(
                "(array.new_fixed $values {} {})",
                converted.len(),
                converted.join(" ")
            )
        };
        let (export, site) = match &e.abi {
            Some(_) => (1, -1),
            None => (0, e.site.map_or(-1, |s| s as i64)),
        };
        let _ = writeln!(
            self.out,
            "(func (export \"{name}\"){sig} (result i32)\n\
             (call $async_begin (i32.const {export}) (i32.const {site}))\n\
             (call $async_run {}))",
            closure(1, body, &captures)
        );
        let Some(abi) = &e.abi else {
            let _ = writeln!(
                self.out,
                "(func (export \"ilang_result:{name}\") (call $async_result))"
            );
            return;
        };
        let value = "(global.get $async_value)";
        if abi.errors.is_empty() {
            let (types, out) = abi_out(abi.result, value);
            let _ = writeln!(
                self.out,
                "(func (export \"ilang_result:{name}\"){} (call $async_result) {out})",
                result_list(&types)
            );
            return;
        }
        let parts = parts(abi);
        let mut f = format!(
            "(func (export \"ilang_result:{name}\"){} (call $async_result)\n\
             (if (i32.eqz (global.get $async_error)) (then (return (i32.const 0){})))\n",
            result_list(&import_results(abi)),
            with_value(&parts, 0, value)
        );
        for (i, m) in abi.errors.iter().enumerate() {
            let _ = writeln!(
                f,
                "(if (i32.eq (call $value_tag {value}) (i32.const {})) (then (return (i32.const {}){})))",
                value_tag(*m),
                i + 1,
                with_value(&parts, i + 1, value)
            );
        }
        f.push_str("(unreachable))\n");
        self.out.push_str(&f);
    }

    /// `ilang_resume:namespace:label` of each async import (B.5): the
    /// answer of the host goes to the globals of the import, and the
    /// program goes on. Calling it when the program does not wait for this
    /// import traps.
    pub(super) fn resume_exports(&mut self) {
        let program: &'p Program = self.p;
        for (i, x) in program.imports.iter().enumerate() {
            if x.kind != ImportKind::AsyncService {
                continue;
            }
            let types = import_results(&x.abi);
            let params: String = types.iter().map(|t| format!(" (param {t})")).collect();
            let sets: String = (0..types.len())
                .map(|j| format!(" (global.set $hr{i}_{j} (local.get {j}))"))
                .collect();
            let _ = writeln!(
                self.out,
                "(func (export \"ilang_resume:{}:{}\"){params} (result i32)\n\
                 (if (i32.ne (global.get $host_pending) (i32.const {i})) (then unreachable)){sets}\n\
                 (call $async_run (ref.null eq)))",
                wat_string(&x.module),
                wat_string(&x.name)
            );
        }
    }
}
