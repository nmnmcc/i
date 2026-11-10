//! Control (§9.4) in the generated module: the runtime functions of errors,
//! unwinding, the environment and control, and the frame records, segments,
//! joins, and re-entry functions of the points of `control.rs`.
//!
//! The host part (`host.rs`) calls this part only through `$host_boundary`
//! and `$host_resume_k`, whose behavior must not change.

use super::*;

/// Errors, unwinding, the environment, and control (`ir.rs`). The functions
/// that call i code save the depth of the shadow stack, and put it back when
/// they take an exception (`runtime/mod.rs`). No handler takes an exception
/// of another tag (no `catch_all`), so a stop, which traps, runs no handlers
/// (§9.5).
///
/// # Control (§9.4)
///
/// - A boundary is a `$boundary` whose field 8 is 1 while it is active.
/// - `suspend` throws `$capture` with a `$cap`: the boundary, `f`, and the
///   frame records so far. Each frame that the exception goes through puts
///   its record first and throws it again (`$capture_rethrow`), so the list
///   is outermost first.
/// - A frame record is a `$frame`: the re-entry function and the values it
///   needs. A re-entry function (`$refn`) gets its record, the records of
///   the frames inside it, the environment, and what to do at the innermost
///   frame: give `v`, or unwind with `$cancel` when `cancel` (the
///   continuation) is not null. It rebuilds its frame, re-enters the inner
///   ones (`$reenter_next`) in a `$capture` handler that puts the same record
///   first again, and goes on.
/// - Each runtime function that calls i code has a record of its own:
///   `$catch_reenter` (`fail.catch`), `$ensure_reenter` and
///   `$final_reenter` (`resource.ensure`, also while its final function
///   runs), `$boundary_reenter` (a boundary that a capture for another
///   boundary went through), and `$cancel_reenter` (a final function that
///   `cancel` runs suspended).
/// - The boundary takes the `$capture` of its own boundary, makes the
///   continuation `$cont`, and calls `f(k)` as a tail call in its place.
///   `resume` re-enters the records inside a boundary that is active again,
///   so the value at the boundary is the value of `resume`. `cancel`
///   re-enters them with the `$cancel` of the continuation, and takes it.
pub(super) const EFFECTS: &str = r#"
(tag $error (param eqref))
(tag $cancel (param eqref))
(tag $capture (param (ref $cap)))
(global $root_env (mut (ref null $env)) (ref.null $env))

;; `env` with `v` in the slot `id` (§9.2): a new array.
(func $env_with (param $env (ref null $env)) (param $id i32) (param $v eqref) (result (ref null $env))
  (local $new (ref $env)) (local $n i32)
  (local.set $n (array.len (local.get $env)))
  (local.set $new (array.new_default $env (local.get $n)))
  (array.copy $env $env (local.get $new) (i32.const 0) (local.get $env) (i32.const 0) (local.get $n))
  (array.set $env (local.get $new) (local.get $id) (local.get $v))
  (local.get $new))

;; Calls a computation (§9.1) with the environment.
(func $call0 (param $f eqref) (param $env (ref null $env)) (result eqref)
  (call_ref $fn0 (ref.cast (ref $clo0) (local.get $f)) (local.get $env)
    (struct.get $clo0 8 (ref.cast (ref $clo0) (local.get $f)))))

;; Calls a function of one parameter with the environment.
(func $call1 (param $f eqref) (param $a eqref) (param $env (ref null $env)) (result eqref)
  (call_ref $fn1 (ref.cast (ref $clo1) (local.get $f)) (local.get $env) (local.get $a)
    (struct.get $clo1 8 (ref.cast (ref $clo1) (local.get $f)))))

;; Puts the frame record `f` first in the capture `c`, and throws `x` again.
(func $capture_rethrow (param $c (ref $cap)) (param $x exnref) (param $f (ref $frame))
  (struct.set $cap 2 (local.get $c) (struct.new $flist (local.get $f) (struct.get $cap 2 (local.get $c))))
  (throw_ref (local.get $x)))

;; Re-enters the frames `rest`, the outermost first. At the innermost frame,
;; gives `v`, or unwinds with `$cancel` when `cancel` is not null.
(func $reenter_next (param $rest (ref null $flist)) (param $env (ref null $env)) (param $v eqref)
    (param $cancel eqref) (result eqref)
  (local $f (ref $frame))
  (if (ref.is_null (local.get $rest))
    (then
      (if (i32.eqz (ref.is_null (local.get $cancel))) (then (throw $cancel (local.get $cancel))))
      (return (local.get $v))))
  (local.set $f (struct.get $flist 0 (ref.as_non_null (local.get $rest))))
  (return_call_ref $refn (local.get $f) (struct.get $flist 1 (ref.as_non_null (local.get $rest)))
    (local.get $env) (local.get $v) (local.get $cancel) (struct.get $frame 0 (local.get $f))))

;; `fail.catch(body)` (§9.3).
(func $fail_catch (param $body eqref) (param $env (ref null $env)) (result eqref)
  (return_call $catch_run (local.get $env) (local.get $body) (ref.null $flist) (ref.null eq) (ref.null eq)
    (ref.null $frame)))

;; Calls `body`, or re-enters `rest` when `body` is null, and takes `$error`.
(func $catch_run (param $env (ref null $env)) (param $body eqref) (param $rest (ref null $flist))
    (param $v eqref) (param $cancel eqref) (param $frame (ref null $frame)) (result eqref)
  (local $depth i32) (local $e eqref) (local $r eqref)
  (local.set $depth (call $shadow_depth))
  (block $captured (result (ref $cap) exnref)
    (local.set $e
      (block $caught (result eqref)
        (local.set $r
          (try_table (result eqref) (catch $error $caught) (catch_ref $capture $captured)
            (if (result eqref) (ref.is_null (local.get $body))
              (then (call $reenter_next (local.get $rest) (local.get $env) (local.get $v) (local.get $cancel)))
              (else (call $call0 (local.get $body) (local.get $env))))))
        (return (call $result_ok (local.get $r)))))
    (call $shadow_restore (local.get $depth))
    (return (call $result_error (local.get $e))))
  (call $capture_rethrow
    (if (result (ref $frame)) (ref.is_null (local.get $frame))
      (then (struct.new $frame (ref.func $catch_reenter) (ref.null $values)))
      (else (ref.as_non_null (local.get $frame)))))
  (unreachable))

(func $catch_reenter (type $refn) (param $frame (ref $frame)) (param $rest (ref null $flist))
    (param $env (ref null $env)) (param $v eqref) (param $cancel eqref) (result eqref)
  (return_call $catch_run (local.get $env) (ref.null eq) (local.get $rest) (local.get $v) (local.get $cancel)
    (local.get $frame)))

;; `resource.ensure(body, with)` (§9.5): `with` runs after `body`, also when
;; an error or an unwinding goes through, which then goes on. A capture
;; does not run it: the computation is paused.
(func $resource_ensure (param $body eqref) (param $with eqref) (param $env (ref null $env)) (result eqref)
  (return_call $ensure_run (local.get $with) (local.get $env) (local.get $body) (ref.null $flist)
    (ref.null eq) (ref.null eq) (ref.null $frame)))

(func $ensure_run (param $with eqref) (param $env (ref null $env)) (param $body eqref)
    (param $rest (ref null $flist)) (param $v eqref) (param $cancel eqref) (param $frame (ref null $frame))
    (result eqref)
  (local $depth i32) (local $r eqref)
  (local.set $depth (call $shadow_depth))
  (block $captured (result (ref $cap) exnref)
    (block $cancelled (result eqref)
      (block $failed (result eqref)
        (local.set $r
          (try_table (result eqref) (catch $error $failed) (catch $cancel $cancelled)
              (catch_ref $capture $captured)
            (if (result eqref) (ref.is_null (local.get $body))
              (then (call $reenter_next (local.get $rest) (local.get $env) (local.get $v) (local.get $cancel)))
              (else (call $call0 (local.get $body) (local.get $env))))))
        (return_call $ensure_final (local.get $with) (local.get $env) (i32.const 0) (local.get $r)))
      (local.set $r)
      (call $shadow_restore (local.get $depth))
      (return_call $ensure_final (local.get $with) (local.get $env) (i32.const 1) (local.get $r)))
    (local.set $r)
    (call $shadow_restore (local.get $depth))
    (return_call $ensure_final (local.get $with) (local.get $env) (i32.const 2) (local.get $r)))
  (call $capture_rethrow
    (if (result (ref $frame)) (ref.is_null (local.get $frame))
      (then (struct.new $frame (ref.func $ensure_reenter) (array.new_fixed $values 1 (local.get $with))))
      (else (ref.as_non_null (local.get $frame)))))
  (unreachable))

(func $ensure_reenter (type $refn) (param $frame (ref $frame)) (param $rest (ref null $flist))
    (param $env (ref null $env)) (param $v eqref) (param $cancel eqref) (result eqref)
  (return_call $ensure_run
    (array.get $values (ref.as_non_null (struct.get $frame 1 (local.get $frame))) (i32.const 0))
    (local.get $env) (ref.null eq) (local.get $rest) (local.get $v) (local.get $cancel) (local.get $frame)))

;; Runs the final function, then ends as `body` did: mode 0 gives `r`, 1
;; throws the error `r`, 2 goes on unwinding the continuation `r`.
(func $ensure_final (param $with eqref) (param $env (ref null $env)) (param $mode i32) (param $r eqref)
    (result eqref)
  (block $captured (result (ref $cap) exnref)
    (drop (try_table (result eqref) (catch_ref $capture $captured)
      (call $call0 (local.get $with) (local.get $env))))
    (return_call $ensure_end (local.get $mode) (local.get $r)))
  (call $capture_rethrow (struct.new $frame (ref.func $final_reenter)
    (array.new_fixed $values 2 (local.get $r) (ref.i31 (local.get $mode)))))
  (unreachable))

(func $ensure_end (param $mode i32) (param $r eqref) (result eqref)
  (if (i32.eq (local.get $mode) (i32.const 1)) (then (throw $error (local.get $r))))
  (if (i32.eq (local.get $mode) (i32.const 2)) (then (throw $cancel (local.get $r))))
  (local.get $r))

(func $final_reenter (type $refn) (param $frame (ref $frame)) (param $rest (ref null $flist))
    (param $env (ref null $env)) (param $v eqref) (param $cancel eqref) (result eqref)
  (local $saved (ref $values))
  (local.set $saved (ref.as_non_null (struct.get $frame 1 (local.get $frame))))
  (block $captured (result (ref $cap) exnref)
    (drop (try_table (result eqref) (catch_ref $capture $captured)
      (call $reenter_next (local.get $rest) (local.get $env) (local.get $v) (local.get $cancel))))
    (return_call $ensure_end
      (i31.get_u (ref.cast (ref i31) (array.get $values (local.get $saved) (i32.const 1))))
      (array.get $values (local.get $saved) (i32.const 0))))
  (call $capture_rethrow (local.get $frame))
  (unreachable))

;; `control.boundary(body)` (§9.4).
(func $control_boundary (param $body eqref) (param $env (ref null $env)) (result eqref)
  (return_call $boundary_run
    (struct.new $boundary (i32.const 11) (i32.const 0) (i64.const 0) (f64.const 0)
      (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call) (i32.const 1))
    (local.get $env) (local.get $body) (ref.null $flist) (ref.null eq) (ref.null eq) (ref.null $frame)))

;; The active boundary `b`, in the place where `env` is: calls `body(b)`,
;; or re-enters `rest` when `body` is null. It is not active once the code
;; inside it is not on the stack (§9.4 rule 1).
(func $boundary_run (param $b (ref $boundary)) (param $env (ref null $env)) (param $body eqref)
    (param $rest (ref null $flist)) (param $v eqref) (param $cancel eqref) (param $frame (ref null $frame))
    (result eqref)
  (local $depth i32) (local $r eqref) (local $c (ref null $cap)) (local $x exnref) (local $f eqref)
  (local.set $depth (call $shadow_depth))
  (block $unwind (result eqref exnref)
    (block $captured (result (ref $cap) exnref)
      (local.set $r
        (try_table (result eqref) (catch_ref $capture $captured) (catch_ref $error $unwind)
            (catch_ref $cancel $unwind)
          (if (result eqref) (ref.is_null (local.get $body))
            (then (call $reenter_next (local.get $rest) (local.get $env) (local.get $v) (local.get $cancel)))
            (else (call $call1 (local.get $body) (local.get $b) (local.get $env))))))
      (struct.set $boundary 8 (local.get $b) (i32.const 0))
      (return (local.get $r)))
    (local.set $x)
    (local.set $c)
    (struct.set $boundary 8 (local.get $b) (i32.const 0))
    (if (ref.eq (struct.get $cap 0 (local.get $c)) (local.get $b))
      (then
        ;; Rule 2: `f(k)` is done in the place of the boundary.
        (call $shadow_restore (local.get $depth))
        (local.set $f (struct.get $cap 1 (local.get $c)))
        (return_call_ref $fn1 (ref.cast (ref $clo1) (local.get $f)) (local.get $env)
          (struct.new $cont (i32.const 12) (i32.const 0) (i64.const 0) (f64.const 0)
            (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)
            (i32.const 0) (local.get $b) (struct.get $cap 2 (local.get $c)))
          (struct.get $clo1 8 (ref.cast (ref $clo1) (local.get $f))))))
    ;; A capture for an outer boundary: this one is paused with it.
    (if (ref.is_null (local.get $frame))
      (then (local.set $frame
        (struct.new $frame (ref.func $boundary_reenter) (array.new_fixed $values 1 (local.get $b))))))
    (call $capture_rethrow (ref.as_non_null (local.get $c)) (local.get $x) (ref.as_non_null (local.get $frame)))
    (unreachable))
  (local.set $x)
  (drop)
  (struct.set $boundary 8 (local.get $b) (i32.const 0))
  (throw_ref (local.get $x)))

(func $boundary_reenter (type $refn) (param $frame (ref $frame)) (param $rest (ref null $flist))
    (param $env (ref null $env)) (param $v eqref) (param $cancel eqref) (result eqref)
  (local $b (ref $boundary))
  (local.set $b (ref.cast (ref $boundary)
    (array.get $values (ref.as_non_null (struct.get $frame 1 (local.get $frame))) (i32.const 0))))
  (struct.set $boundary 8 (local.get $b) (i32.const 1))
  (return_call $boundary_run (local.get $b) (local.get $env) (ref.null eq) (local.get $rest) (local.get $v)
    (local.get $cancel) (local.get $frame)))

;; `control.suspend(b, f)`: stops when `b` is not active.
(func $control_suspend (param $b eqref) (param $f eqref) (result eqref)
  (local $boundary (ref $boundary))
  (local.set $boundary (ref.cast (ref $boundary) (local.get $b)))
  (if (i32.eqz (struct.get $boundary 8 (local.get $boundary)))
    (then (call $stop_boundary_inactive) (unreachable)))
  (throw $capture (struct.new $cap (local.get $boundary) (local.get $f) (ref.null $flist))))

;; The continuation `k`, used now: stops when it was already used (rule 5).
(func $continuation_take (param $k eqref) (result (ref $cont))
  (local $c (ref $cont))
  (local.set $c (ref.cast (ref $cont) (local.get $k)))
  (if (struct.get $cont 8 (local.get $c)) (then (call $stop_continuation_used) (unreachable)))
  (struct.set $cont 8 (local.get $c) (i32.const 1))
  (local.get $c))

;; `control.resume(k, v)`: the paused computation goes on in a boundary that
;; is active again, on top of this place (rule 3).
(func $control_resume (param $k eqref) (param $v eqref) (param $env (ref null $env)) (result eqref)
  (local $c (ref $cont)) (local $b (ref $boundary))
  (local.set $c (call $continuation_take (local.get $k)))
  (local.set $b (struct.get $cont 9 (local.get $c)))
  (struct.set $boundary 8 (local.get $b) (i32.const 1))
  (return_call $boundary_run (local.get $b) (local.get $env) (ref.null eq) (struct.get $cont 10 (local.get $c))
    (local.get $v) (ref.null eq) (ref.null $frame)))

;; `control.cancel(k)`: unwinds the paused computation (rule 4). Its
;; boundary stays not active.
(func $control_cancel (param $k eqref) (param $env (ref null $env)) (result eqref)
  (local $c (ref $cont))
  (local.set $c (call $continuation_take (local.get $k)))
  (return_call $cancel_run (local.get $c) (struct.get $cont 10 (local.get $c)) (local.get $env)
    (ref.null eq) (local.get $c) (ref.null $frame)))

(func $cancel_run (param $k (ref $cont)) (param $rest (ref null $flist)) (param $env (ref null $env))
    (param $v eqref) (param $cancel eqref) (param $frame (ref null $frame)) (result eqref)
  (local $depth i32) (local $e eqref) (local $x exnref)
  (local.set $depth (call $shadow_depth))
  (block $captured (result (ref $cap) exnref)
    (block $cancelled (result eqref exnref)
      (drop (try_table (result eqref) (catch_ref $cancel $cancelled) (catch_ref $capture $captured)
        (call $reenter_next (local.get $rest) (local.get $env) (local.get $v) (local.get $cancel))))
      (return (ref.null eq)))
    (local.set $x)
    (local.set $e)
    (if (i32.eqz (ref.eq (local.get $e) (local.get $k))) (then (throw_ref (local.get $x))))
    (call $shadow_restore (local.get $depth))
    (return (ref.null eq)))
  (call $capture_rethrow
    (if (result (ref $frame)) (ref.is_null (local.get $frame))
      (then (struct.new $frame (ref.func $cancel_reenter) (array.new_fixed $values 1 (local.get $k))))
      (else (ref.as_non_null (local.get $frame)))))
  (unreachable))

(func $cancel_reenter (type $refn) (param $frame (ref $frame)) (param $rest (ref null $flist))
    (param $env (ref null $env)) (param $v eqref) (param $cancel eqref) (result eqref)
  (return_call $cancel_run
    (ref.cast (ref $cont)
      (array.get $values (ref.as_non_null (struct.get $frame 1 (local.get $frame))) (i32.const 0)))
    (local.get $rest) (local.get $env) (local.get $v) (local.get $cancel) (local.get $frame)))
"#;

/// The types of control (§9.4), after `$env`.
pub(super) const TYPES: &str = r#"
(type $boundary (sub final $obj (struct (field i32) (field i32) (field i64) (field f64)
  (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs))
  (field (ref null $call)) (field (mut i32)))))
(rec
  (type $frame (struct (field (ref $refn)) (field (ref null $values))))
  (type $refn (func (param (ref $frame) (ref null $flist) (ref null $env) eqref eqref) (result eqref)))
  (type $flist (struct (field (ref $frame)) (field (ref null $flist)))))
(type $cont (sub final $obj (struct (field i32) (field i32) (field i64) (field f64)
  (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs))
  (field (ref null $call)) (field (mut i32)) (field (ref $boundary)) (field (ref null $flist)))))
(type $cap (struct (field (ref $boundary)) (field eqref) (field (mut (ref null $flist)))))
(type $box (struct (field i64)))
"#;

impl<'p> Emitter<'p> {
    /// A call that may suspend (§9.4): a handler around it adds the frame
    /// record of its point to a capture that goes through.
    pub(super) fn suspendable(
        &mut self,
        out: &mut String,
        dst: Local,
        value: &Value,
        site: Option<Site>,
        point: PointId,
        frame: bool,
    ) {
        if let Some(site) = site {
            let _ = writeln!(out, "(call $shadow_site (i32.const {}))", site_code(site));
        }
        // The slot of the frame, which may be a tail site (§11.7),
        // goes in the frame record. The inner frames are still on
        // the shadow stack when the record is made, so it is read
        // before the call.
        if frame {
            out.push_str("(local.set $slot (call $shadow_top))\n");
        }
        let v = self.value(value);
        let record = self.record(point);
        let _ = writeln!(
            out,
            "(local.set $l{dst} (block $ok{point} (result eqref)\n\
             (block $cap{point} (result (ref $cap) exnref)\n\
             (br $ok{point} (try_table (result eqref) (catch_ref $capture $cap{point})\n\
             {v})))\n\
             (call $capture_rethrow {record}) (unreachable)))"
        );
    }

    /// Goes on with the segment of the point `q` (§9.4).
    pub(super) fn continue_point(&mut self, out: &mut String, q: PointId) {
        let program: &'p Program = self.p;
        let point = &program.points[q as usize];
        let args: String = point
            .params
            .iter()
            .map(|l| format!(" (local.get $l{l})"))
            .collect();
        let _ = writeln!(
            out,
            "(return_call $s{q} (local.get $env){args} (local.get $l{}))",
            point.dst
        );
    }

    /// Goes on with the join `j` (§9.4).
    pub(super) fn join_call(&mut self, out: &mut String, j: JoinId) {
        let program: &'p Program = self.p;
        let args: String = program.joins[j as usize]
            .params
            .iter()
            .map(|l| format!(" (local.get $l{l})"))
            .collect();
        let _ = writeln!(out, "(return_call $j{j} (local.get $env){args})");
    }

    /// The locals of the function of a point.
    pub(super) fn func_of(&self, point: &Point) -> &'p Func {
        let p: &'p Program = self.p;
        match &p.funcs[point.func.0 as usize] {
            FuncKind::Code(f) => f,
            _ => unreachable!("a point is in code"),
        }
    }

    /// The frame record of a point (§9.4): its re-entry function, the saved
    /// locals, then the values of the environment statements it rebuilds,
    /// then, for a function with a frame, the slot of its frame on the
    /// shadow stack as an `i31`. Numbers are boxed.
    pub(super) fn record(&self, p: PointId) -> String {
        let program: &'p Program = self.p;
        let point = &program.points[p as usize];
        let locals = &self.func_of(point).locals;
        let mut items = Vec::new();
        for l in point.saved(locals) {
            items.push(match locals[l as usize] {
                LocalType::Value => format!("(local.get $l{l})"),
                LocalType::I64 => format!("(struct.new $box (local.get $l{l}))"),
                LocalType::I32 => format!("(struct.new $box (i64.extend_i32_u (local.get $l{l})))"),
                LocalType::Env => unreachable!("environments are rebuilt"),
            });
        }
        for link in &point.envs {
            items.push(format!(
                "(array.get $env (local.get $l{}) (i32.const {}))",
                link.dst, link.service
            ));
        }
        if self.func_of(point).frame {
            items.push("(ref.i31 (local.get $slot))".into());
        }
        let saved = if items.is_empty() {
            "(ref.null $values)".to_string()
        } else {
            format!(
                "(array.new_fixed $values {} {})",
                items.len(),
                items.join(" ")
            )
        };
        format!("(struct.new $frame (ref.func $r{p}) {saved})")
    }

    /// A function that goes on with the rest of the function `f`: a
    /// segment of a point, with the value of its call `dst`, or a join.
    pub(super) fn rest_func(
        &mut self,
        name: &str,
        f: &Func,
        params: &[Local],
        dst: Option<Local>,
        locals: &[Local],
        code: &[Stmt],
    ) {
        let ty = |l: Local| local_type(f.locals[l as usize]);
        let _ = write!(
            self.out,
            ";; {} {}\n(func ${name} (param $env (ref null $env))",
            f.name.replace('\n', " "),
            match dst {
                Some(_) => format!("after point {}", &name[1..]),
                None => format!("join {}", &name[1..]),
            }
        );
        for (j, &l) in params.iter().enumerate() {
            let _ = write!(self.out, " (param $a{j} {})", ty(l));
        }
        if dst.is_some() {
            self.out.push_str(" (param $res eqref)");
        }
        self.out
            .push_str(" (result eqref)\n (local $clo (ref null $obj))");
        if f.frame {
            self.out.push_str(" (local $slot i32)");
        }
        for &l in locals {
            let _ = write!(self.out, " (local $l{l} {})", ty(l));
        }
        self.out.push('\n');
        for (j, &l) in params.iter().enumerate() {
            let _ = writeln!(self.out, "(local.set $l{l} (local.get $a{j}))");
        }
        if let Some(dst) = dst {
            let _ = writeln!(self.out, "(local.set $l{dst} (local.get $res))");
        }
        let mut body = String::new();
        self.stmts(&mut body, code, f.frame);
        self.out.push_str(&body);
        self.out.push_str("(unreachable))\n");
    }

    /// The function `$j…` of a join (§9.4).
    pub(super) fn join(&mut self, j: JoinId) {
        let program: &'p Program = self.p;
        let join = &program.joins[j as usize];
        let FuncKind::Code(f) = &program.funcs[join.func.0 as usize] else {
            unreachable!("a join is in code")
        };
        self.rest_func(
            &format!("j{j}"),
            f,
            &join.params,
            None,
            &join.locals,
            &join.segment,
        );
    }

    /// The segment `$s…` and the re-entry function `$r…` of a point (§9.4).
    pub(super) fn point(&mut self, p: PointId) {
        let program: &'p Program = self.p;
        let point = &program.points[p as usize];
        let f = self.func_of(point);
        let locals = &f.locals;
        let ty = |l: Local| local_type(locals[l as usize]);
        // The segment: the environment, the locals it needs, and the value
        // of the call.
        self.rest_func(
            &format!("s{p}"),
            f,
            &point.params,
            Some(point.dst),
            &point.locals,
            &point.segment,
        );
        // The re-entry: rebuild the frame, re-enter the inner frames in a
        // handler that adds this record again, and go on with the segment.
        let _ = write!(
            self.out,
            "(func $r{p} (type $refn) (param $frame (ref $frame)) (param $rest (ref null $flist)) \
             (param $env (ref null $env)) (param $v eqref) (param $cancel eqref) (result eqref)\n \
             (local $saved (ref null $values)) (local $res eqref)"
        );
        let saved: Vec<Local> = point.saved(locals).collect();
        let mut declared: BTreeSet<Local> = saved.iter().copied().collect();
        declared.extend(point.envs.iter().map(|e| e.dst));
        for &l in &declared {
            let _ = write!(self.out, " (local $l{l} {})", ty(l));
        }
        self.out
            .push_str("\n(local.set $saved (struct.get $frame 1 (local.get $frame)))\n");
        for (j, &l) in saved.iter().enumerate() {
            let get =
                format!("(array.get $values (ref.as_non_null (local.get $saved)) (i32.const {j}))");
            let v = match locals[l as usize] {
                LocalType::I64 => format!("(struct.get $box 0 (ref.cast (ref $box) {get}))"),
                LocalType::I32 => {
                    format!("(i32.wrap_i64 (struct.get $box 0 (ref.cast (ref $box) {get})))")
                }
                _ => get,
            };
            let _ = writeln!(self.out, "(local.set $l{l} {v})");
        }
        // §9.4 rule 3: the environment statements of the frame, on the
        // environment of the place of `resume`.
        for (k, link) in point.envs.iter().enumerate() {
            let _ = writeln!(
                self.out,
                "(local.set $l{} (call $env_with {} (i32.const {}) \
                 (array.get $values (ref.as_non_null (local.get $saved)) (i32.const {}))))",
                link.dst,
                env_text(link.from),
                link.service,
                saved.len() + k
            );
        }
        // The slot of the frame as it was at the point: the site of the
        // call, or the tail site of the tail call that called the function
        // (§11.7).
        if f.frame {
            let _ = writeln!(
                self.out,
                "(call $shadow_enter (i31.get_s (ref.cast (ref i31) \
                 (array.get $values (ref.as_non_null (local.get $saved)) (i32.const {})))))",
                saved.len() + point.envs.len()
            );
        }
        let args: String = point
            .params
            .iter()
            .map(|l| format!(" (local.get $l{l})"))
            .collect();
        let _ = writeln!(
            self.out,
            "(local.set $res (block $ok (result eqref)\n\
             (block $cap (result (ref $cap) exnref)\n\
             (br $ok (try_table (result eqref) (catch_ref $capture $cap)\n\
             (call $reenter_next (local.get $rest) {} (local.get $v) (local.get $cancel)))))\n\
             (call $capture_rethrow (local.get $frame)) (unreachable)))\n\
             (return_call $s{p} (local.get $env){args} (local.get $res)))",
            env_text(point.env)
        );
    }
}
