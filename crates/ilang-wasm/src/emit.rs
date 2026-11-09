//! From the intermediate form to a Wasm module: WAT text with the runtime
//! of `runtime/mod.rs`, assembled to a binary, with the `ilang.abi` custom
//! section (B.6).

use crate::ir::*;
use crate::runtime;
use num_bigint::{BigInt, Sign};
use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

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
const EFFECTS: &str = r#"
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

;; The `fail` of the `Assert` of an entry point (§9.6, §11.7).
(func $entry_assert_fail (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $failure eqref) (result eqref)
  (call $stop_assertion (call $field (local.get $failure) (i32.const 0)))
  (unreachable))
"#;

/// The types of control (§9.4), after `$env`.
const CONTROL_TYPES: &str = r#"
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
(type $waiting (struct (field i32)))
"#;

/// The runtime functions whose references are taken.
const DECLARED: &[&str] = &[
    "$entry_assert_fail",
    "$catch_reenter",
    "$ensure_reenter",
    "$final_reenter",
    "$boundary_reenter",
    "$cancel_reenter",
];

/// The WAT text of the whole module.
pub fn wat(p: &Program) -> String {
    let mut e = Emitter {
        p,
        arities: BTreeSet::new(),
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
    m.push_str(runtime::TYPES);
    m.push_str("(type $env (array (mut eqref)))\n");
    m.push_str(CONTROL_TYPES);
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
    m.push_str(runtime::HOST);
    // The imports of B.4, before every definition.
    for (i, x) in p.imports.iter().enumerate() {
        let params: Vec<&str> = x
            .abi
            .params
            .iter()
            .flat_map(|t| abi_in(*t).0.iter().copied())
            .collect();
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
    m.push_str(runtime::wat());
    m.push_str(SUPPORT);
    m.push_str(EFFECTS);
    m.push_str(&e.data);
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
            } => {
                if let Some(site) = site {
                    let _ = writeln!(out, "(call $shadow_site (i32.const {}))", site_code(*site));
                }
                // The slot of the frame, which may be a tail site (§11.7),
                // goes in the frame record. The inner frames are still on
                // the shadow stack when the record is made, so it is read
                // before the call.
                if frame {
                    out.push_str("(local.set $slot (call $shadow_top))\n");
                }
                let v = self.value(value);
                let record = self.record(*point);
                let _ = writeln!(
                    out,
                    "(local.set $l{dst} (block $ok{point} (result eqref)\n\
                     (block $cap{point} (result (ref $cap) exnref)\n\
                     (br $ok{point} (try_table (result eqref) (catch_ref $capture $cap{point})\n\
                     {v})))\n\
                     (call $capture_rethrow {record}) (unreachable)))"
                );
            }
            Stmt::Continue(q) => {
                let program: &'p Program = self.p;
                let point = &program.points[*q as usize];
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
            Stmt::Join(j) => {
                let program: &'p Program = self.p;
                let args: String = program.joins[*j as usize]
                    .params
                    .iter()
                    .map(|l| format!(" (local.get $l{l})"))
                    .collect();
                let _ = writeln!(out, "(return_call $j{j} (local.get $env){args})");
            }
        }
    }

    /// The locals of the function of a point.
    fn func_of(&self, point: &Point) -> &'p Func {
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
    fn record(&self, p: PointId) -> String {
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
    fn rest_func(
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
    fn join(&mut self, j: JoinId) {
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
    fn point(&mut self, p: PointId) {
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
            Atom::Ctor { ctor, ty } => format!(
                "(struct.new $obj (i32.const 5) (i32.const {ctor}) (i64.const {ty}) (f64.const 0) \
                 (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call))"
            ),
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
            Value::HostCall { import, args } => {
                let call = self.host_call(*import, args);
                let reply = self.host_reply(*import);
                format!("(block (result eqref)\n{call}\n{reply})")
            }
            Value::HostStart { import, args } => {
                let call = self.host_call(*import, args);
                format!(
                    "(block (result eqref)\n\
                     (if (i32.ne (global.get $host_pending) (i32.const -1)) (then unreachable))\n\
                     {call}\n(global.set $host_pending (i32.const {import})) (ref.null eq))"
                )
            }
            Value::HostReply(import) => self.host_reply(*import),
        }
    }

    /// Calls an import (B.2, B.3): the arguments are encoded in new buffers
    /// through the globals `$hp…`, the results go to the globals `$hr…`, and
    /// the buffers of the arguments are freed.
    fn host_call(&mut self, i: ImportId, args: &[Atom]) -> String {
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
    fn host_reply(&self, i: ImportId) -> String {
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
            "(func (export \"ilang_start\")\n\
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
                    let _ = writeln!(
                        self.out,
                        "(func (export \"{name}\") (local $e eqref)\n\
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

    /// An async entry point (B.5): it runs in a root boundary and gives a
    /// status; `ilang_result:name` gives its results when it has ended.
    fn async_export(&mut self, name: &str, e: &Export, body: FuncId) {
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
    fn resume_exports(&mut self) {
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
        AbiType::Externref => (&["externref"], "$abi_ref_in"),
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
        AbiType::Externref => format!("(call $abi_ref_out {v})"),
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
        AbiType::Externref => 10,
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
