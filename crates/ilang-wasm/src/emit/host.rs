//! The host boundary in the generated module (§9.7, §10.2, B.4, B.5):
//! import calls and their answers, async entry points, and the
//! `ilang_resume:…` exports. It calls control (`control.rs`) only through
//! `$host_boundary` and `$host_resume_k`.

use super::*;

/// The runtime functions of the host boundary (§9.7, B.4, B.5).
///
/// These notes stay here, not in the WAT text, because every module carries
/// that text.
///
/// An async entry point runs in a root boundary, `$host_root`. A function of
/// an async host service takes a request id from the request table
/// (`$req_start`), starts the import with it, and suspends to the root
/// boundary, where `$host_wait` keeps the continuation in `$host_k` and gives
/// `$host_waiting`: the status is 1. The export `ilang_resume:…` of the
/// import gets the id first; it checks it against the table
/// (`$req_answer`), puts the answer in the globals of the import and resumes
/// the continuation, so the function reads the answer with the frames of the
/// program on the shadow stack (§11.7).
///
/// Stream R2 takes over the request table. These parts are correct only
/// while one request is outstanding, and R2 must replace them together:
/// - `$req_start`, `$req_answer`, `$req_outstanding`, `$req_clear` become a
///   table from id to { import, answer }, with a free list. The id may be the
///   slot index. `$req_clear` frees every entry; `$async_run` already calls
///   it before status 0 (B.5). `$async_result` and `$async_begin` check
///   `$async_live`, not `$req_outstanding`, because an entry point that
///   returns status 2 may have nothing outstanding and still not have ended;
///   keep that check, and keep clearing `$async_live` only at status 0.
/// - The answer globals `$hr{i}_*`: each resume must decode its answer into
///   the entry of its request before control leaves the export, or a second
///   answer of the same import overwrites the first.
/// - `$host_k` stays one continuation: the root continuation of whatever
///   waits (today the one async call; under R2 the place where the scheduler
///   waits for any answer). `$host_resume_k` keeps resuming that one `k`, and
///   passes it which request ended. Do not store a root continuation per
///   request: each would hold its own copy of the scheduler's state.
/// - The scheduler returns status 1 to the root only when no task can run.
///   At its scheduling points, while some task can still run, it returns
///   status 2 so that the host can give finished answers, and goes on in the
///   export `ilang_continue`, which R2 adds to the module (B.5). That export
///   must trap when the last status the module gave was not 2.
/// - Cancelling a request value lowers to the import `cancel(id)` from
///   `ilang:host/v1` and frees the entry (B.4). It drops host work only; the
///   task's continuation is an i value, and the scheduler cancels it in i
///   code so that its final functions run.
///
/// None of this changes the ABI: v3 hosts already accept many outstanding
/// requests in any order, status 2 and `cancel`. A need that v3 does not
/// cover (a new runtime import, status or export) means ABI version 4.
pub(super) const EFFECTS: &str = r#"
;; The host boundary (§9.7, B.4, B.5). The notes for stream R2 are on
;; `EFFECTS` in emit/host.rs.
(global $host_root (mut (ref null $boundary)) (ref.null $boundary))
(global $host_k (mut eqref) (ref.null eq))
;; The request table (B.4, B.5): the import of the outstanding request (-1
;; for none), its id, and the next id, which wraps at 2^31.
(global $host_import (mut i32) (i32.const -1))
(global $host_request (mut i32) (i32.const -1))
(global $host_next (mut i32) (i32.const 0))
;; 1 from `$async_begin` until the entry point gives status 0.
(global $async_live (mut i32) (i32.const 0))
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

;; The request table. In this module it has one entry; R2 replaces these
;; four functions with a table of requests (see the notes in emit/host.rs). The traps are bare
;; `unreachable`: a wrong answer is a fault of the host, not a stop.
(func $req_start (param $import i32) (result i32)
  (local $id i32)
  (if (i32.ne (global.get $host_import) (i32.const -1)) (then unreachable))
  (local.set $id (global.get $host_next))
  (global.set $host_next (i32.and (i32.add (local.get $id) (i32.const 1)) (i32.const 0x7fffffff)))
  (global.set $host_import (local.get $import))
  (global.set $host_request (local.get $id))
  (local.get $id))

(func $req_answer (param $import i32) (param $id i32)
  (if (i32.or (i32.ne (global.get $host_import) (local.get $import))
              (i32.ne (global.get $host_request) (local.get $id)))
    (then unreachable))
  (global.set $host_import (i32.const -1))
  (global.set $host_request (i32.const -1)))

(func $req_outstanding (result i32)
  (i32.ne (global.get $host_import) (i32.const -1)))

;; The entry point has ended: the host drops what is left (B.5), so forget it.
(func $req_clear
  (global.set $host_import (i32.const -1))
  (global.set $host_request (i32.const -1)))

(func $host_resume_k (result eqref)
  (local $k eqref)
  (local.set $k (global.get $host_k))
  (global.set $host_k (ref.null eq))
  (return_call $control_resume (local.get $k) (ref.null eq) (global.get $root_env)))

;; The start of an async entry point. Until it has ended, the host may only
;; answer it (B.5).
(func $async_begin (param $export i32) (param $site i32)
  (if (global.get $async_live) (then unreachable))
  (global.set $async_live (i32.const 1))
  (global.set $async_export (local.get $export))
  (global.set $async_site (local.get $site))
  (global.set $async_value (ref.null eq))
  (global.set $async_error (i32.const 0)))

;; `ilang_result:…` may only be called when the entry point has ended.
(func $async_result
  (if (global.get $async_live) (then unreachable)))

;; Runs the body of an async entry point in a new root boundary, or, when
;; `body` is null, goes on with it after the host answered. Gives the status
;; of B.5: 1 while it waits, 0 when it has ended. At 0 the requests that
;; are left are forgotten, as the host drops them.
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
      (call $req_clear)
      (global.set $async_live (i32.const 0))
      (return (i32.const 0)))
    (drop) (call $stop_boundary_inactive) (unreachable)))
  ;; An error stops the program entry and tests; a host export gives it to
  ;; the host (§10.2).
  (if (i32.eqz (global.get $async_export))
    (then (call $stop_error (local.get $e) (global.get $async_site)) (unreachable)))
  (call $shadow_restore (local.get $depth))
  (global.set $async_value (local.get $e))
  (global.set $async_error (i32.const 1))
  (call $req_clear)
  (global.set $async_live (i32.const 0))
  (i32.const 0))
"#;

/// The type of the status "waiting" (B.5), after the types of control.
pub(super) const TYPES: &str = "(type $waiting (struct (field i32)))\n";

impl<'p> Emitter<'p> {
    /// Starts an import of an async host service (B.4) with a new request
    /// id from the request table: the program then waits for this request.
    pub(super) fn host_start(&mut self, import: ImportId, args: &[Atom]) -> String {
        let id = format!(" (call $req_start (i32.const {import}))");
        let call = self.host_call_with(import, &id, args);
        format!("(block (result eqref)\n{call}\n(ref.null eq))")
    }

    /// Calls an import (B.2, B.3): the arguments are encoded in new buffers
    /// through the globals `$hp…`, the results go to the globals `$hr…`, and
    /// the buffers of the arguments are freed.
    pub(super) fn host_call(&mut self, i: ImportId, args: &[Atom]) -> String {
        self.host_call_with(i, "", args)
    }

    /// `host_call`, with `first` as the code of a first Wasm argument
    /// before the encoded arguments (the request id of B.4), or empty.
    fn host_call_with(&mut self, i: ImportId, first: &str, args: &[Atom]) -> String {
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
        let _ = write!(code, "(call $imp{i}{first}{gets})");
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
    pub(super) fn host_reply(&self, i: ImportId, result: AbiType) -> String {
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
                AbiType::Externref(ty) => format!("(call $abi_ref_in {} (i32.const {ty}))", g(0)),
            }
        };
        if x.abi.errors.is_empty() {
            return take(result, 0);
        }
        let mut offsets = Vec::new();
        let mut k = 1;
        for t in std::iter::once(result).chain(x.abi.errors.iter().copied()) {
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
    /// request id comes first, then the answer of the host, which goes to
    /// the globals of the import, and the program goes on. Calling it with
    /// an id that is not outstanding, or that another import started, traps.
    pub(super) fn resume_exports(&mut self) {
        let program: &'p Program = self.p;
        for (i, x) in program.imports.iter().enumerate() {
            if x.kind != ImportKind::AsyncService {
                continue;
            }
            let types = import_results(&x.abi);
            let params: String = types.iter().map(|t| format!(" (param {t})")).collect();
            let sets: String = (0..types.len())
                .map(|j| format!(" (global.set $hr{i}_{j} (local.get {}))", j + 1))
                .collect();
            let _ = writeln!(
                self.out,
                "(func (export \"ilang_resume:{}:{}\") (param $id i32){params} (result i32)\n\
                 (call $req_answer (i32.const {i}) (local.get $id)){sets}\n\
                 (call $async_run (ref.null eq)))",
                wat_string(&x.module),
                wat_string(&x.name)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    /// Every module carries `EFFECTS`, so its WAT keeps only short comments;
    /// longer notes go in the doc comment of `EFFECTS`.
    #[test]
    fn effects_keep_only_short_comments() {
        let mut block = 0;
        for line in super::EFFECTS.lines() {
            if line.trim_start().starts_with(";;") {
                block += 1;
                assert!(
                    block <= 6,
                    "a comment of more than 6 lines in EFFECTS: {line}"
                );
            } else {
                block = 0;
            }
        }
    }
}
