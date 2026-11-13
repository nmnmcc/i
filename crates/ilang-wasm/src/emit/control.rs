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
///   frame records so far. Each frame that the exception goes through adds
///   its record at the end (`$capture_rethrow`), so the list is innermost
///   first.
/// - A frame record is a `$frame`: its re-entry function, the values it
///   needs, its slot on the shadow stack, and its kind. The kind says
///   whether the record is called when an error or a cancel reaches it:
///   only when its frame had a handler for it. A frame whose call is under
///   an environment statement also keeps the statements' services and
///   values.
/// - `resume` and `cancel` re-enter only the innermost record. `$tramp_run`
///   keeps the others, and gives each one what the frames inside it ended
///   with: a value, an error or a cancel. A record then does what its frame
///   would have done at that point. Code frames have nothing to do with an
///   error or a cancel, so they are passed over. When an error or a cancel
///   goes out of `$tramp_run`, the slots of the frames it passed over are
///   put back on the shadow stack, where they would have been.
/// - A capture that leaves `$tramp_run` takes the records that are still
///   waiting with it, in one step.
/// - A boundary or a cancel that a capture goes through keeps the records
///   inside it in one record of its own. `resume` re-enters such records at
///   once, so their boundaries are active again.
/// - `$boundary_run` is kept for `host.rs` only; `$boundary_core` does the
///   work.
pub(super) const EFFECTS: &str = r#"
(tag $error (param eqref))
(tag $cancel (param eqref))
(tag $capture (param (ref $cap)))
(global $root_env (mut (ref null $env)) (ref.null $env))
;; The live trampolines, innermost first.
(global $pend (mut (ref null $tramp)) (ref.null $tramp))

;; `env` with `v` in the slot `id` (§9.2): a new array.
(func $env_with (param $env (ref null $env)) (param $id i32) (param $v eqref) (result (ref null $env))
  (local $new (ref $env)) (local $n i32)
  (local.set $n (array.len (local.get $env)))
  (local.set $new (array.new_default $env (local.get $n)))
  (array.copy $env $env (local.get $new) (i32.const 0) (local.get $env) (i32.const 0) (local.get $n))
  (array.set $env (local.get $new) (local.get $id) (local.get $v))
  (local.get $new))

;; `env` with the pairs `[service, value]` of `delta` set in order, so a
;; later pair wins, as a chain of `$env_with` does: one new array.
(func $env_apply (param $env (ref null $env)) (param $delta (ref null $values)) (result (ref null $env))
  (local $new (ref $env)) (local $pairs (ref $values)) (local $n i32) (local $i i32)
  (if (ref.is_null (local.get $delta)) (then (return (local.get $env))))
  (local.set $pairs (ref.as_non_null (local.get $delta)))
  (local.set $n (array.len (local.get $env)))
  (local.set $new (array.new_default $env (local.get $n)))
  (array.copy $env $env (local.get $new) (i32.const 0) (local.get $env) (i32.const 0) (local.get $n))
  (block $done (loop $each
    (br_if $done (i32.ge_u (local.get $i) (array.len (local.get $pairs))))
    (array.set $env (local.get $new)
      (i31.get_u (ref.cast (ref i31) (array.get $values (local.get $pairs) (local.get $i))))
      (array.get $values (local.get $pairs) (i32.add (local.get $i) (i32.const 1))))
    (local.set $i (i32.add (local.get $i) (i32.const 2)))
    (br $each)))
  (local.get $new))

;; Calls a computation (§9.1) with the environment.
(func $call0 (param $f eqref) (param $env (ref null $env)) (result eqref)
  (call_ref $fn0 (ref.cast (ref $clo0) (local.get $f)) (local.get $env)
    (struct.get $clo0 8 (ref.cast (ref $clo0) (local.get $f)))))

;; Calls a function of one parameter with the environment.
(func $call1 (param $f eqref) (param $a eqref) (param $env (ref null $env)) (result eqref)
  (call_ref $fn1 (ref.cast (ref $clo1) (local.get $f)) (local.get $env) (local.get $a)
    (struct.get $clo1 8 (ref.cast (ref $clo1) (local.get $f)))))

;; Adds the frame record `f` at the end of the capture `c` (the list is
;; innermost first), and throws `x` again.
(func $capture_rethrow (param $c (ref $cap)) (param $x exnref) (param $f (ref $frame))
  (local $n (ref $flist))
  (local.set $n (struct.new $flist (local.get $f) (ref.null $flist) (ref.null $flist) (ref.null $env)
    (i32.const 0)))
  (if (ref.is_null (struct.get $cap 3 (local.get $c)))
    (then (struct.set $cap 2 (local.get $c) (local.get $n)))
    (else (struct.set $flist 1 (ref.as_non_null (struct.get $cap 3 (local.get $c))) (local.get $n))))
  (struct.set $cap 3 (local.get $c) (local.get $n))
  (if (i32.eq (struct.get $frame 3 (local.get $f)) (i32.const 1))
    (then
      (if (ref.is_null (struct.get $cap 5 (local.get $c)))
        (then (struct.set $cap 4 (local.get $c) (local.get $n)))
        (else (struct.set $flist 2 (ref.as_non_null (struct.get $cap 5 (local.get $c))) (local.get $n))))
      (struct.set $cap 5 (local.get $c) (local.get $n))))
  (throw_ref (local.get $x)))

;; The records of the capture `c`, or null when it has none.
(func $capture_sub (param $c (ref $cap)) (result (ref null $sub))
  (if (ref.is_null (struct.get $cap 2 (local.get $c))) (then (return (ref.null $sub))))
  (struct.new $sub (struct.get $cap 2 (local.get $c)) (struct.get $cap 3 (local.get $c))
    (struct.get $cap 4 (local.get $c)) (struct.get $cap 5 (local.get $c))))

;; A capture that goes through a boundary or a cancel: its records go into
;; one record of kind 3 (re-entry `f`, values `[first, records]`), which is
;; then the only record of the capture. Throws `x` again.
(func $capture_pack (param $c (ref $cap)) (param $x exnref) (param $f (ref $refn)) (param $first eqref)
  (local $n (ref $flist))
  (local.set $n (struct.new $flist
    (struct.new $frame (local.get $f)
      (array.new_fixed $values 2 (local.get $first) (call $capture_sub (local.get $c)))
      (i32.const -1) (i32.const 3) (ref.null $values))
    (ref.null $flist) (ref.null $flist) (ref.null $env) (i32.const 0)))
  (struct.set $cap 2 (local.get $c) (local.get $n))
  (struct.set $cap 3 (local.get $c) (local.get $n))
  (struct.set $cap 4 (local.get $c) (ref.null $flist))
  (struct.set $cap 5 (local.get $c) (ref.null $flist))
  (throw_ref (local.get $x)))

;; Joins the records `p … last` (kind-1 records `pm … mlast`) after the
;; records of the capture `c`.
(func $capture_splice (param $c (ref $cap)) (param $p (ref null $flist)) (param $pm (ref null $flist))
    (param $last (ref null $flist)) (param $mlast (ref null $flist))
  (if (i32.eqz (ref.is_null (local.get $p)))
    (then
      (if (ref.is_null (struct.get $cap 3 (local.get $c)))
        (then (struct.set $cap 2 (local.get $c) (local.get $p)))
        (else (struct.set $flist 1 (ref.as_non_null (struct.get $cap 3 (local.get $c))) (local.get $p))))
      (struct.set $cap 3 (local.get $c) (local.get $last))))
  (if (i32.eqz (ref.is_null (local.get $pm)))
    (then
      (if (ref.is_null (struct.get $cap 5 (local.get $c)))
        (then (struct.set $cap 4 (local.get $c) (local.get $pm)))
        (else (struct.set $flist 2 (ref.as_non_null (struct.get $cap 5 (local.get $c))) (local.get $pm))))
      (struct.set $cap 5 (local.get $c) (local.get $mlast)))))

;; The number of code records (kind 0 or 1) from `n` to the end of its list.
(func $code_count (param $n (ref null $flist)) (result i32)
  (local $k i32)
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $n)))
    (if (i32.le_u (struct.get $frame 3 (struct.get $flist 0 (ref.as_non_null (local.get $n)))) (i32.const 1))
      (then (local.set $k (i32.add (local.get $k) (i32.const 1)))))
    (local.set $n (struct.get $flist 1 (ref.as_non_null (local.get $n))))
    (br $each)))
  (local.get $k))

;; Writes the slots of the code records from `n` to the end of its list into
;; `a`, below `hi`: the innermost highest. Gives the lowest index written.
(func $code_slots (param $a (ref $frames)) (param $hi i32) (param $n (ref null $flist)) (result i32)
  (local $f (ref $frame))
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $n)))
    (local.set $f (struct.get $flist 0 (ref.as_non_null (local.get $n))))
    (if (i32.le_u (struct.get $frame 3 (local.get $f)) (i32.const 1))
      (then
        (local.set $hi (i32.sub (local.get $hi) (i32.const 1)))
        (array.set $frames (local.get $a) (local.get $hi) (struct.get $frame 2 (local.get $f)))))
    (local.set $n (struct.get $flist 1 (ref.as_non_null (local.get $n))))
    (br $each)))
  (local.get $hi))

;; Puts the slots of the code records from `from` to the end of its list
;; into the shadow stack at `at`, the outermost lowest, and moves the slots
;; at `at` and above up by their number. Makes room for all of them at once.
(func $shadow_insert (param $at i32) (param $from (ref null $flist))
  (local $n i32) (local $sp i32) (local $need i32) (local $old (ref $frames)) (local $new (ref $frames))
  (local $size i32)
  (local.set $n (call $code_count (local.get $from)))
  (if (i32.eqz (local.get $n)) (then (return)))
  (local.set $sp (global.get $sp))
  (local.set $need (i32.add (local.get $sp) (local.get $n)))
  ;; The limit of $shadow_grow: out of stack.
  (if (i32.gt_u (local.get $need) (i32.const 268435456)) (then unreachable))
  (local.set $old (global.get $shadow))
  (local.set $size (array.len (local.get $old)))
  (if (i32.gt_u (local.get $need) (local.get $size))
    (then
      (local.set $size (i32.shl (local.get $size) (i32.const 1)))
      (if (i32.gt_u (local.get $need) (local.get $size)) (then (local.set $size (local.get $need))))
      (local.set $new (array.new_default $frames (local.get $size)))
      (array.copy $frames $frames (local.get $new) (i32.const 0) (local.get $old) (i32.const 0) (local.get $at))
      (array.copy $frames $frames (local.get $new) (i32.add (local.get $at) (local.get $n))
        (local.get $old) (local.get $at) (i32.sub (local.get $sp) (local.get $at)))
      (global.set $shadow (local.get $new)))
    (else
      (array.copy $frames $frames (local.get $old) (i32.add (local.get $at) (local.get $n))
        (local.get $old) (local.get $at) (i32.sub (local.get $sp) (local.get $at)))))
  (drop (call $code_slots (global.get $shadow) (i32.add (local.get $at) (local.get $n)) (local.get $from)))
  (global.set $sp (local.get $need)))

;; Called by `$stop` (runtime/stops.rs) before it prints: a new shadow
;; stack with the slots of the records that wait in the trampolines of
;; `$pend`, in the places where eager re-entry would have pushed them.
(func $control_unpend (type $unpend)
  (local $sp i32) (local $n i32) (local $top i32) (local $hi i32) (local $len i32)
  (local $t (ref null $tramp)) (local $new (ref $frames)) (local $old (ref $frames))
  (if (ref.is_null (global.get $pend)) (then (return)))
  (local.set $old (global.get $shadow))
  (local.set $sp (global.get $sp))
  (if (i32.gt_u (local.get $sp) (array.len (local.get $old)))
    (then (local.set $sp (array.len (local.get $old)))))
  ;; A trampoline above the running depth is not live (section 5.9 of the
  ;; design): it is skipped.
  (local.set $top (local.get $sp))
  (local.set $t (global.get $pend))
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $t)))
    (if (i32.le_u (struct.get $tramp 0 (ref.as_non_null (local.get $t))) (local.get $top))
      (then
        (local.set $n (i32.add (local.get $n)
          (call $code_count (struct.get $tramp 1 (ref.as_non_null (local.get $t))))))
        (local.set $top (struct.get $tramp 0 (ref.as_non_null (local.get $t))))))
    (local.set $t (struct.get $tramp 2 (ref.as_non_null (local.get $t))))
    (br $each)))
  (if (i32.eqz (local.get $n)) (then (return)))
  (local.set $new (array.new_default $frames (i32.add (local.get $sp) (local.get $n))))
  (local.set $hi (i32.add (local.get $sp) (local.get $n)))
  (local.set $top (local.get $sp))
  (local.set $t (global.get $pend))
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $t)))
    (if (i32.le_u (struct.get $tramp 0 (ref.as_non_null (local.get $t))) (local.get $top))
      (then
        (local.set $len (i32.sub (local.get $top) (struct.get $tramp 0 (ref.as_non_null (local.get $t)))))
        (local.set $hi (i32.sub (local.get $hi) (local.get $len)))
        (array.copy $frames $frames (local.get $new) (local.get $hi)
          (local.get $old) (struct.get $tramp 0 (ref.as_non_null (local.get $t))) (local.get $len))
        (local.set $top (struct.get $tramp 0 (ref.as_non_null (local.get $t))))
        (local.set $hi (call $code_slots (local.get $new) (local.get $hi)
          (struct.get $tramp 1 (ref.as_non_null (local.get $t)))))))
    (local.set $t (struct.get $tramp 2 (ref.as_non_null (local.get $t))))
    (br $each)))
  (array.copy $frames $frames (local.get $new) (i32.const 0) (local.get $old) (i32.const 0) (local.get $top))
  (global.set $shadow (local.get $new))
  (global.set $sp (i32.add (local.get $sp) (local.get $n))))

;; For code that resets `$sp` for a new entry in the same instance.
(func $control_reset
  (global.set $pend (ref.null $tramp)))

;; Re-enters the records `s`, innermost first. The innermost one is given
;; `v` with `mode` (0 a value, 1 an error, 2 a cancel); each next one is
;; given what the one before it ended with. A code record is called only
;; with a value, and a runtime record only in the modes its frame had a
;; handler for. The records not called yet are pending in a `$tramp` in
;; `$pend`; a capture takes them with it.
(func $tramp_run (param $s (ref null $sub)) (param $env (ref null $env)) (param $v eqref) (param $mode i32)
    (result eqref)
  (local $node (ref null $flist)) (local $nm (ref null $flist)) (local $last (ref null $flist))
  (local $mlast (ref null $flist)) (local $first (ref null $flist)) (local $from (ref null $flist))
  (local $m (ref null $flist)) (local $prev (ref null $flist)) (local $next (ref null $flist))
  (local $this (ref $flist)) (local $fr (ref $frame)) (local $e (ref null $env)) (local $cur (ref null $env))
  (local $given (ref null $env)) (local $d i32) (local $t (ref $tramp)) (local $c (ref $cap)) (local $x exnref)
  (if (ref.is_null (local.get $s)) (then (return_call $ensure_end (local.get $mode) (local.get $v))))
  (local.set $node (struct.get $sub 0 (ref.as_non_null (local.get $s))))
  (if (ref.is_null (local.get $node)) (then (return_call $ensure_end (local.get $mode) (local.get $v))))
  (local.set $last (struct.get $sub 1 (ref.as_non_null (local.get $s))))
  (local.set $nm (struct.get $sub 2 (ref.as_non_null (local.get $s))))
  (local.set $mlast (struct.get $sub 3 (ref.as_non_null (local.get $s))))
  ;; The environment each kind-1 record is given, outermost first (§9.4
  ;; rule 3): reverse the kind-1 chain, walk it, and reverse it back.
  (local.set $m (local.get $nm))
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $m)))
    (local.set $next (struct.get $flist 2 (ref.as_non_null (local.get $m))))
    (struct.set $flist 2 (ref.as_non_null (local.get $m)) (local.get $prev))
    (local.set $prev (local.get $m))
    (local.set $m (local.get $next))
    (br $each)))
  (local.set $m (local.get $prev))
  (local.set $prev (ref.null $flist))
  (local.set $e (local.get $env))
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $m)))
    (struct.set $flist 3 (ref.as_non_null (local.get $m)) (local.get $e))
    (local.set $e (call $env_apply (local.get $e)
      (struct.get $frame 4 (struct.get $flist 0 (ref.as_non_null (local.get $m))))))
    (local.set $next (struct.get $flist 2 (ref.as_non_null (local.get $m))))
    (struct.set $flist 2 (ref.as_non_null (local.get $m)) (local.get $prev))
    (local.set $prev (local.get $m))
    (local.set $m (local.get $next))
    (br $each)))
  (local.set $cur (local.get $e))
  (local.set $d (call $shadow_depth))
  (local.set $t (struct.new $tramp (local.get $d) (local.get $node) (global.get $pend)))
  (global.set $pend (local.get $t))
  (global.set $stop_unpend (ref.func $control_unpend))
  (local.set $first (local.get $node))
  ;; Where the slots of the records passed over go back, if the list ends
  ;; with an error or a cancel: from the first record still pending at the
  ;; last catch, at depth `d`.
  (local.set $from (local.get $node))
  (block $captured (result (ref $cap) exnref)
    (loop $step
      (if (ref.is_null (local.get $node))
        (then
          (if (local.get $mode) (then (call $shadow_insert (local.get $d) (local.get $from))))
          (global.set $pend (struct.get $tramp 2 (local.get $t)))
          (return_call $ensure_end (local.get $mode) (local.get $v))))
      (local.set $this (ref.as_non_null (local.get $node)))
      (local.set $fr (struct.get $flist 0 (local.get $this)))
      ;; A record is taken once, and a delimiter is only ever the first.
      (if (struct.get $flist 4 (local.get $this)) (then unreachable))
      (struct.set $flist 4 (local.get $this) (i32.const 1))
      (if (i32.and (i32.eq (struct.get $frame 3 (local.get $fr)) (i32.const 3))
          (i32.eqz (ref.eq (local.get $this) (local.get $first))))
        (then unreachable))
      (local.set $node (struct.get $flist 1 (local.get $this)))
      (struct.set $tramp 1 (local.get $t) (local.get $node))
      (if (ref.eq (local.get $this) (local.get $nm))
        (then
          (local.set $given (struct.get $flist 3 (local.get $this)))
          (local.set $nm (struct.get $flist 2 (local.get $this)))
          (local.set $cur (local.get $given)))
        (else (local.set $given (local.get $cur))))
      (if (local.get $mode)
        (then
          ;; Kinds 2, 3 and 4 take an error; kinds 2 and 3 take a cancel.
          (br_if $step (i32.eqz (i32.and (i32.const 1)
            (i32.shr_u
              (select (i32.const 0x1C) (i32.const 0x0C) (i32.eq (local.get $mode) (i32.const 1)))
              (struct.get $frame 3 (local.get $fr))))))
          (call $shadow_restore (local.get $d))))
      (local.set $v
        (block $cancelled (result eqref)
          (local.set $v
            (block $failed (result eqref)
              (local.set $v
                (try_table (result eqref) (catch $error $failed) (catch $cancel $cancelled)
                    (catch_ref $capture $captured)
                  (call_ref $refn (local.get $fr) (local.get $given) (local.get $v) (local.get $mode)
                    (struct.get $frame 0 (local.get $fr)))))
              (local.set $mode (i32.const 0))
              (br $step)))
          (local.set $from (local.get $node))
          (local.set $mode (i32.const 1))
          (br $step)))
      (local.set $from (local.get $node))
      (local.set $mode (i32.const 2))
      (br $step))
    (unreachable))
  (local.set $x)
  (local.set $c)
  (call $capture_splice (local.get $c) (local.get $node) (local.get $nm) (local.get $last) (local.get $mlast))
  (global.set $pend (struct.get $tramp 2 (local.get $t)))
  (throw_ref (local.get $x)))

;; `fail.catch(body)` (§9.3).
(func $fail_catch (param $body eqref) (param $env (ref null $env)) (result eqref)
  (return_call $catch_run (local.get $env) (local.get $body)))

;; Calls `body`, and takes `$error`.
(func $catch_run (param $env (ref null $env)) (param $body eqref) (result eqref)
  (local $depth i32) (local $e eqref) (local $r eqref)
  (local.set $depth (call $shadow_depth))
  (block $captured (result (ref $cap) exnref)
    (local.set $e
      (block $caught (result eqref)
        (local.set $r
          (try_table (result eqref) (catch $error $caught) (catch_ref $capture $captured)
            (call $call0 (local.get $body) (local.get $env))))
        (return (call $result_ok (local.get $r)))))
    ;; An escape goes on to its boundary.
    (if (ref.test (ref $escaped) (local.get $e)) (then (throw $error (local.get $e))))
    (call $shadow_restore (local.get $depth))
    (return (call $result_error (local.get $e))))
  (call $capture_rethrow
    (struct.new $frame (ref.func $catch_reenter) (ref.null $values) (i32.const -1) (i32.const 4)
      (ref.null $values)))
  (unreachable))

;; The body ended with the value or the error `v`.
(func $catch_reenter (type $refn) (param $frame (ref $frame)) (param $env (ref null $env)) (param $v eqref)
    (param $mode i32) (result eqref)
  (if (i32.eq (local.get $mode) (i32.const 1))
    (then
      (if (ref.test (ref $escaped) (local.get $v)) (then (throw $error (local.get $v))))
      (return_call $result_error (local.get $v))))
  (return_call $result_ok (local.get $v)))

;; `resource.ensure(body, with)` (§9.5): `with` runs after `body`, also when
;; an error or an unwinding goes through, which then goes on. A capture
;; does not run it: the computation is paused.
(func $resource_ensure (param $body eqref) (param $with eqref) (param $env (ref null $env)) (result eqref)
  (return_call $ensure_run (local.get $with) (local.get $env) (local.get $body)))

(func $ensure_run (param $with eqref) (param $env (ref null $env)) (param $body eqref) (result eqref)
  (local $depth i32) (local $r eqref)
  (local.set $depth (call $shadow_depth))
  (block $captured (result (ref $cap) exnref)
    (block $cancelled (result eqref)
      (block $failed (result eqref)
        (local.set $r
          (try_table (result eqref) (catch $error $failed) (catch $cancel $cancelled)
              (catch_ref $capture $captured)
            (call $call0 (local.get $body) (local.get $env))))
        (return_call $ensure_final (local.get $with) (local.get $env) (i32.const 0) (local.get $r)))
      (local.set $r)
      (call $shadow_restore (local.get $depth))
      (return_call $ensure_final (local.get $with) (local.get $env) (i32.const 1) (local.get $r)))
    (local.set $r)
    (call $shadow_restore (local.get $depth))
    (return_call $ensure_final (local.get $with) (local.get $env) (i32.const 2) (local.get $r)))
  (call $capture_rethrow
    (struct.new $frame (ref.func $ensure_reenter) (array.new_fixed $values 1 (local.get $with))
      (i32.const -1) (i32.const 2) (ref.null $values)))
  (unreachable))

;; The body ended with `v` in the mode `mode`: run the final function.
(func $ensure_reenter (type $refn) (param $frame (ref $frame)) (param $env (ref null $env)) (param $v eqref)
    (param $mode i32) (result eqref)
  (return_call $ensure_final
    (array.get $values (ref.as_non_null (struct.get $frame 1 (local.get $frame))) (i32.const 0))
    (local.get $env) (local.get $mode) (local.get $v)))

;; Runs the final function, then ends as `body` did: mode 0 gives `r`, 1
;; throws the error `r`, 2 goes on unwinding the continuation `r`.
(func $ensure_final (param $with eqref) (param $env (ref null $env)) (param $mode i32) (param $r eqref)
    (result eqref)
  (block $captured (result (ref $cap) exnref)
    (drop (try_table (result eqref) (catch_ref $capture $captured)
      (call $call0 (local.get $with) (local.get $env))))
    (return_call $ensure_end (local.get $mode) (local.get $r)))
  (call $capture_rethrow (struct.new $frame (ref.func $final_reenter)
    (array.new_fixed $values 2 (local.get $r) (ref.i31 (local.get $mode)))
    (i32.const -1) (i32.const 37) (ref.null $values)))
  (unreachable))

(func $ensure_end (param $mode i32) (param $r eqref) (result eqref)
  (if (i32.eq (local.get $mode) (i32.const 1)) (then (throw $error (local.get $r))))
  (if (i32.eq (local.get $mode) (i32.const 2)) (then (throw $cancel (local.get $r))))
  (local.get $r))

;; Kind 37 has the same low five dispatch bits as kind 5, with a final-only
;; marker so ordinary host-wait re-entry cannot protect a task from cancel.
;; The final function returned: end as `body` did.
(func $final_reenter (type $refn) (param $frame (ref $frame)) (param $env (ref null $env)) (param $v eqref)
    (param $mode i32) (result eqref)
  (local $saved (ref $values))
  (local.set $saved (ref.as_non_null (struct.get $frame 1 (local.get $frame))))
  (return_call $ensure_end
    (i31.get_u (ref.cast (ref i31) (array.get $values (local.get $saved) (i32.const 1))))
    (array.get $values (local.get $saved) (i32.const 0))))

;; `control.boundary(body)` (§9.4).
(func $control_boundary (param $body eqref) (param $env (ref null $env)) (result eqref)
  (return_call $boundary_core
    (struct.new $boundary (i32.const 11) (i32.const 0) (i64.const 0) (f64.const 0)
      (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call) (i32.const 1)
      (ref.null eq) (i32.const 0) (ref.null eq) (ref.null eq))
    (local.get $env) (local.get $body) (ref.null $sub) (ref.null eq) (i32.const 0)))

;; Kept with these parameters for `emit/host.rs`. `rest`, `cancel` and
;; `frame` must be null.
(func $boundary_run (param $b (ref $boundary)) (param $env (ref null $env)) (param $body eqref)
    (param $rest (ref null $flist)) (param $v eqref) (param $cancel eqref) (param $frame (ref null $frame))
    (result eqref)
  (if (i32.eqz (ref.is_null (local.get $rest))) (then unreachable))
  (if (i32.eqz (ref.is_null (local.get $cancel))) (then unreachable))
  (if (i32.eqz (ref.is_null (local.get $frame))) (then unreachable))
  (return_call $boundary_core (local.get $b) (local.get $env) (local.get $body) (ref.null $sub) (local.get $v)
    (i32.const 0)))

;; The active boundary `b`, in the place where `env` is: calls `body(b)`,
;; or re-enters the records `sub` with `v` in the mode `mode` when `body`
;; is null. It is not active once the code inside it is not on the stack
;; (§9.4 rule 1).
(func $boundary_core (param $b (ref $boundary)) (param $env (ref null $env)) (param $body eqref)
    (param $sub (ref null $sub)) (param $v eqref) (param $mode i32) (result eqref)
  (local $depth i32) (local $r eqref) (local $c (ref null $cap)) (local $x exnref) (local $f eqref)
  (local.set $depth (call $shadow_depth))
  (block $unwind (result eqref exnref)
    (block $captured (result (ref $cap) exnref)
      (local.set $r
        (try_table (result eqref) (catch_ref $capture $captured) (catch_ref $error $unwind)
            (catch_ref $cancel $unwind)
          (if (result eqref) (ref.is_null (local.get $body))
            (then (call $tramp_run (local.get $sub) (local.get $env) (local.get $v) (local.get $mode)))
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
            (i32.const 0) (local.get $b) (call $capture_sub (ref.as_non_null (local.get $c))) (ref.null eq))
          (struct.get $clo1 8 (ref.cast (ref $clo1) (local.get $f))))))
    ;; A capture for an outer boundary: this one is paused with it.
    (call $capture_pack (ref.as_non_null (local.get $c)) (local.get $x) (ref.func $boundary_reenter)
      (local.get $b))
    (unreachable))
  (local.set $x)
  (local.set $f)
  (struct.set $boundary 8 (local.get $b) (i32.const 0))
  ;; `control.escape(b, v)` for this boundary: it ends with `v`.
  (if (ref.test (ref $escaped) (local.get $f))
    (then
      (if (ref.eq (struct.get $escaped 0 (ref.cast (ref $escaped) (local.get $f))) (local.get $b))
        (then
          (call $shadow_restore (local.get $depth))
          (return (struct.get $escaped 1 (ref.cast (ref $escaped) (local.get $f))))))))
  (throw_ref (local.get $x)))

;; A boundary that a capture for another boundary went through: active
;; again, around the records inside it.
(func $boundary_reenter (type $refn) (param $frame (ref $frame)) (param $env (ref null $env)) (param $v eqref)
    (param $mode i32) (result eqref)
  (local $saved (ref $values)) (local $b (ref $boundary))
  (local.set $saved (ref.as_non_null (struct.get $frame 1 (local.get $frame))))
  (local.set $b (ref.cast (ref $boundary) (array.get $values (local.get $saved) (i32.const 0))))
  (struct.set $boundary 8 (local.get $b) (i32.const 1))
  (return_call $boundary_core (local.get $b) (local.get $env) (ref.null eq)
    (ref.cast (ref null $sub) (array.get $values (local.get $saved) (i32.const 1)))
    (local.get $v) (local.get $mode)))

;; A light boundary (`Value::Light`): a boundary whose handle is only given
;; to `control.escape`. Field 2 keeps the depth of the shadow stack where it
;; starts, which an escape to it puts back.
(func $light_new (result eqref)
  (struct.new $boundary (i32.const 11) (i32.const 0) (i64.extend_i32_u (call $shadow_depth)) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call) (i32.const 1)
    (ref.null eq) (i32.const 0) (ref.null eq) (ref.null eq)))

;; A provided value made later (`Stmt::LetEnv` of a light boundary's
;; handle): the light boundary `b` stands for the value `builder(b)` in the
;; environment until a service read needs it (`$service_force`).
(func $light_lazy (param $b eqref) (param $builder eqref) (result eqref)
  (struct.set $boundary 11 (ref.cast (ref $boundary) (local.get $b)) (local.get $builder))
  (ref.null eq))

;; The value of a service read from the environment: a light boundary that
;; stands for a provided value gives that value, made once (the same value
;; at every read); anything else is the value.
(func $service_force (param $v eqref) (param $env (ref null $env)) (result eqref)
  (local $b (ref $boundary)) (local $m eqref)
  (if (i32.eqz (ref.test (ref $boundary) (local.get $v))) (then (return (local.get $v))))
  (local.set $b (ref.cast (ref $boundary) (local.get $v)))
  (if (ref.is_null (struct.get $boundary 11 (local.get $b))) (then (return (local.get $v))))
  (local.set $m (struct.get $boundary 12 (local.get $b)))
  (if (ref.is_null (local.get $m))
    (then
      (local.set $m (call $call1 (struct.get $boundary 11 (local.get $b)) (local.get $b) (local.get $env)))
      (struct.set $boundary 12 (local.get $b) (local.get $m))))
  (local.get $m))

;; The body of the light boundary `b` gave `r`.
(func $light_done (param $b eqref) (param $r eqref) (result eqref)
  (struct.set $boundary 8 (ref.cast (ref $boundary) (local.get $b)) (i32.const 0))
  (local.get $r))

;; The error or the cancel `e` (exception `x`) left the body of the light
;; boundary `b`: an escape to `b` ends it with its value; anything else goes
;; on, as in `$boundary_core`.
(func $light_unwind (param $e eqref) (param $x exnref) (param $b eqref) (result eqref)
  (local $boundary (ref $boundary)) (local $esc (ref $escaped))
  (local.set $boundary (ref.cast (ref $boundary) (local.get $b)))
  (struct.set $boundary 8 (local.get $boundary) (i32.const 0))
  (if (ref.test (ref $escaped) (local.get $e))
    (then
      (local.set $esc (ref.cast (ref $escaped) (local.get $e)))
      (if (ref.eq (struct.get $escaped 0 (local.get $esc)) (local.get $boundary))
        (then
          (call $shadow_restore (i32.wrap_i64 (struct.get $boundary 2 (local.get $boundary))))
          (return (struct.get $escaped 1 (local.get $esc)))))))
  (throw_ref (local.get $x)))

;; A capture for another boundary went through the light boundary `b`: it
;; is paused with it, and is re-entered as a `$boundary` (`$boundary_reenter`).
(func $light_captured (param $c (ref $cap)) (param $x exnref) (param $b eqref)
  (local $boundary (ref $boundary))
  (local.set $boundary (ref.cast (ref $boundary) (local.get $b)))
  (struct.set $boundary 8 (local.get $boundary) (i32.const 0))
  (call $capture_pack (local.get $c) (local.get $x) (ref.func $boundary_reenter) (local.get $boundary)))

;; `control.suspend(b, f)`: stops when `b` is not active.
(func $control_suspend (param $b eqref) (param $f eqref) (result eqref)
  (local $boundary (ref $boundary))
  (local.set $boundary (ref.cast (ref $boundary) (local.get $b)))
  (if (i32.eqz (struct.get $boundary 8 (local.get $boundary)))
    (then (call $stop_boundary_inactive) (unreachable)))
  (throw $capture (struct.new $cap (local.get $boundary) (local.get $f) (ref.null $flist) (ref.null $flist)
    (ref.null $flist) (ref.null $flist))))

;; `control.escape(b, v)`: stops when `b` is not active. It throws `$error`,
;; so `resource.ensure` and the trampoline treat it as an error, and the
;; boundary `b` ends with `v` (`$boundary_core`). No capture, no records.
(func $control_escape (param $b eqref) (param $v eqref) (result eqref)
  (local $boundary (ref $boundary))
  (local.set $boundary (ref.cast (ref $boundary) (local.get $b)))
  (if (i32.eqz (struct.get $boundary 8 (local.get $boundary)))
    (then (call $stop_boundary_inactive) (unreachable)))
  (throw $error (struct.new $escaped (local.get $boundary) (local.get $v))))

;; The value of `$error` for `control.escape(b, v)`, thrown by the caller
;; (`Op::Escape`): stops when `b` is not active.
(func $escape_payload (param $b eqref) (param $v eqref) (result (ref $escaped))
  (local $boundary (ref $boundary))
  (local.set $boundary (ref.cast (ref $boundary) (local.get $b)))
  (if (i32.eqz (struct.get $boundary 8 (local.get $boundary)))
    (then (call $stop_boundary_inactive) (unreachable)))
  (struct.new $escaped (local.get $boundary) (local.get $v)))

;; The continuation `k`, used now: stops when it was already used (rule 5).
(func $continuation_take (param $k eqref) (result (ref $cont))
  (local $c (ref $cont))
  (local.set $c (ref.cast (ref $cont) (local.get $k)))
  (if (i32.and (struct.get $cont 8 (local.get $c)) (i32.const 1)) (then (call $stop_continuation_used) (unreachable)))
  (struct.set $cont 8 (local.get $c) (i32.const 1))
  (local.get $c))

;; A read of immutable continuation metadata, without taking the continuation.
;; Boundary identity limits the query to finals inside that delimiter, so a
;; new scope inside a final still has its own cancellable body.
(func $sub_has_final (param $s (ref null $sub)) (result i32)
  (local $node (ref null $flist)) (local $f (ref $frame))
  (if (ref.is_null (local.get $s)) (then (return (i32.const 0))))
  (local.set $node (struct.get $sub 0 (ref.as_non_null (local.get $s))))
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $node)))
    (local.set $f (struct.get $flist 0 (ref.as_non_null (local.get $node))))
    (if (i32.eq (struct.get $frame 3 (local.get $f)) (i32.const 37))
      (then (return (i32.const 1))))
    (if (i32.eq (struct.get $frame 3 (local.get $f)) (i32.const 3))
      (then (if (call $sub_has_final (ref.cast (ref null $sub)
        (array.get $values (struct.get $frame 1 (local.get $f)) (i32.const 1))))
        (then (return (i32.const 1))))))
    (local.set $node (struct.get $flist 1 (ref.as_non_null (local.get $node))))
    (br $each)))
  (i32.const 0))
(func $sub_finalizing_at (param $s (ref null $sub)) (param $b eqref) (result i32)
  (local $node (ref null $flist)) (local $f (ref $frame))
  (local $saved (ref $values)) (local $inner (ref null $sub)) (local $found i32)
  (if (ref.is_null (local.get $s)) (then (return (i32.const -1))))
  (local.set $node (struct.get $sub 0 (ref.as_non_null (local.get $s))))
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $node)))
    (local.set $f (struct.get $flist 0 (ref.as_non_null (local.get $node))))
    (if (i32.eq (struct.get $frame 3 (local.get $f)) (i32.const 3)) (then
      (local.set $saved (ref.as_non_null (struct.get $frame 1 (local.get $f))))
      (local.set $inner (ref.cast (ref null $sub)
        (array.get $values (local.get $saved) (i32.const 1))))
      (if (ref.eq (array.get $values (local.get $saved) (i32.const 0)) (local.get $b))
        (then (return_call $sub_has_final (local.get $inner))))
      (local.set $found (call $sub_finalizing_at (local.get $inner) (local.get $b)))
      (if (i32.ge_s (local.get $found) (i32.const 0)) (then (return (local.get $found))))))
    (local.set $node (struct.get $flist 1 (ref.as_non_null (local.get $node))))
    (br $each)))
  (i32.const -1))
(func $stamp_finalizing (param $s (ref null $sub)) (param $token eqref) (result i32)
  (local $node (ref null $flist)) (local $f (ref $frame)) (local $saved (ref $values))
  (local $owner eqref) (local $inside i32) (local $result i32)
  (if (ref.is_null (local.get $s)) (then (return (i32.const 0))))
  (local.set $node (struct.get $sub 0 (ref.as_non_null (local.get $s))))
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $node)))
    (local.set $f (struct.get $flist 0 (ref.as_non_null (local.get $node))))
    (if (i32.eq (struct.get $frame 3 (local.get $f)) (i32.const 37))
      (then (local.set $result (i32.const 1))))
    (if (i32.eq (struct.get $frame 3 (local.get $f)) (i32.const 3)) (then
      (local.set $saved (ref.as_non_null (struct.get $frame 1 (local.get $f))))
      (local.set $inside (call $stamp_finalizing (ref.cast (ref null $sub)
        (array.get $values (local.get $saved) (i32.const 1))) (local.get $token)))
      (local.set $owner (array.get $values (local.get $saved) (i32.const 0)))
      (if (ref.test (ref $boundary) (local.get $owner)) (then
        (struct.set $boundary 9 (ref.cast (ref $boundary) (local.get $owner)) (local.get $token))
        (struct.set $boundary 10 (ref.cast (ref $boundary) (local.get $owner)) (local.get $inside))))
      (local.set $result (i32.or (local.get $result) (local.get $inside)))))
    (local.set $node (struct.get $flist 1 (ref.as_non_null (local.get $node))))
    (br $each)))
  (local.get $result))
(func $control_finalizing (param $k eqref) (param $b eqref) (result eqref)
  (local $c (ref $cont)) (local $boundary (ref $boundary)) (local $token eqref)
  (local.set $c (ref.cast (ref $cont) (local.get $k)))
  (local.set $boundary (ref.cast (ref $boundary) (local.get $b)))
  (if (i32.and (struct.get $cont 8 (local.get $c)) (i32.const 1))
    (then (call $stop_continuation_used) unreachable))
  (local.set $token (struct.get $cont 11 (local.get $c)))
  (if (ref.is_null (local.get $token)) (then
    (local.set $token (call $key_new))
    (struct.set $cont 11 (local.get $c) (local.get $token))))
  (if (i32.eqz (i32.and (struct.get $cont 8 (local.get $c)) (i32.const 2))) (then
    (drop (call $stamp_finalizing (struct.get $cont 10 (local.get $c)) (local.get $token)))
    (struct.set $cont 8 (local.get $c) (i32.const 2))))
  (if (ref.eq (struct.get $boundary 9 (local.get $boundary)) (local.get $token))
    (then (return (call $bool_new (struct.get $boundary 10 (local.get $boundary))))))
  ;; Another continuation can have refreshed the same boundary's cache, or
  ;; this delimiter is absent from k. The immutable records remain the oracle.
  (call $bool_new (i32.gt_s (call $sub_finalizing_at
    (struct.get $cont 10 (local.get $c)) (local.get $b)) (i32.const 0))))

;; `control.resume(k, v)`: the paused computation goes on in a boundary that
;; is active again, on top of this place (rule 3).
(func $control_resume (param $k eqref) (param $v eqref) (param $env (ref null $env)) (result eqref)
  (local $c (ref $cont)) (local $b (ref $boundary))
  (local.set $c (call $continuation_take (local.get $k)))
  (local.set $b (struct.get $cont 9 (local.get $c)))
  (struct.set $boundary 8 (local.get $b) (i32.const 1))
  (return_call $boundary_core (local.get $b) (local.get $env) (ref.null eq) (struct.get $cont 10 (local.get $c))
    (local.get $v) (i32.const 0)))

;; `control.cancel(k)`: unwinds the paused computation (rule 4). Its
;; boundary stays not active.
(func $control_cancel (param $k eqref) (param $env (ref null $env)) (result eqref)
  (local $c (ref $cont))
  (local.set $c (call $continuation_take (local.get $k)))
  (return_call $cancel_core (local.get $c) (struct.get $cont 10 (local.get $c)) (local.get $env)
    (local.get $c) (i32.const 2)))

;; Re-enters the records `sub` of the continuation `k` with `v` in the
;; mode `mode`, and takes the `$cancel` of `k`.
(func $cancel_core (param $k (ref $cont)) (param $sub (ref null $sub)) (param $env (ref null $env))
    (param $v eqref) (param $mode i32) (result eqref)
  (local $depth i32) (local $e eqref) (local $x exnref) (local $c (ref null $cap))
  (local.set $depth (call $shadow_depth))
  (block $captured (result (ref $cap) exnref)
    (block $cancelled (result eqref exnref)
      (drop (try_table (result eqref) (catch_ref $cancel $cancelled) (catch_ref $capture $captured)
        (call $tramp_run (local.get $sub) (local.get $env) (local.get $v) (local.get $mode))))
      (return (ref.null eq)))
    (local.set $x)
    (local.set $e)
    (if (i32.eqz (ref.eq (local.get $e) (local.get $k))) (then (throw_ref (local.get $x))))
    (call $shadow_restore (local.get $depth))
    (return (ref.null eq)))
  (local.set $x)
  (local.set $c)
  (call $capture_pack (ref.as_non_null (local.get $c)) (local.get $x) (ref.func $cancel_reenter)
    (local.get $k))
  (unreachable))

;; A cancel whose final function suspended: it goes on around the records
;; inside it.
(func $cancel_reenter (type $refn) (param $frame (ref $frame)) (param $env (ref null $env)) (param $v eqref)
    (param $mode i32) (result eqref)
  (local $saved (ref $values))
  (local.set $saved (ref.as_non_null (struct.get $frame 1 (local.get $frame))))
  (return_call $cancel_core
    (ref.cast (ref $cont) (array.get $values (local.get $saved) (i32.const 0)))
    (ref.cast (ref null $sub) (array.get $values (local.get $saved) (i32.const 1)))
    (local.get $env) (local.get $v) (local.get $mode)))
;; Kept slots are ordinary tuple fields, so the general portable walk sees
;; every service, including slots unused by the argument's inferred type.
(func $control_keep_env (param $f eqref) (param $env (ref null $env)) (result eqref)
  (local $slots (ref $values)) (local $i i32) (local $n i32)
  (local.set $n (array.len (local.get $env)))
  (local.set $slots (array.new_default $values (local.get $n)))
  (block $done (loop $copy
    (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
    (array.set $values (local.get $slots) (local.get $i)
      (array.get $env (local.get $env) (local.get $i)))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $copy)))
  (struct.new $clo1 (i32.const 7) (i32.const 0) (i64.const 1) (f64.const 0)
    (ref.null $bytes)
    (array.new_fixed $values 2 (local.get $f) (call $tuple (local.get $slots)))
    (ref.null $limbs) (ref.null $call) (ref.func $keep_call)))

(func $keep_call (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env))
    (param $arg eqref) (result eqref)
  (local $caps (ref $values)) (local $slots (ref $values)) (local $kept (ref $env))
  (local $i i32) (local $n i32)
  (local.set $caps (ref.as_non_null (struct.get $obj 5 (ref.as_non_null (local.get $clo)))))
  (local.set $slots (ref.as_non_null (struct.get $obj 5
    (ref.cast (ref $obj) (array.get $values (local.get $caps) (i32.const 1))))))
  (local.set $n (array.len (local.get $slots)))
  (local.set $kept (array.new_default $env (local.get $n)))
  (block $done (loop $copy
    (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
    (array.set $env (local.get $kept) (local.get $i)
      (array.get $values (local.get $slots) (local.get $i)))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $copy)))
  (return_call $keep_run (array.get $values (local.get $caps) (i32.const 0))
    (local.get $arg) (local.get $kept)))

;; A delimiter packs its inner records. Re-entry always uses the kept base,
;; before applying any environment statements made inside the kept call.
(func $keep_run (param $f eqref) (param $arg eqref) (param $env (ref null $env)) (result eqref)
  (block $captured (result (ref $cap) exnref)
    (return (try_table (result eqref) (catch_ref $capture $captured)
      (call $call1 (local.get $f) (local.get $arg) (local.get $env)))))
  (call $capture_pack (ref.func $keep_reenter) (local.get $env))
  unreachable)

(func $keep_reenter (type $refn) (param $frame (ref $frame)) (param $env (ref null $env))
    (param $v eqref) (param $mode i32) (result eqref)
  (local $saved (ref $values)) (local $kept (ref null $env))
  (local $c (ref $cap)) (local $x exnref)
  (local.set $saved (ref.as_non_null (struct.get $frame 1 (local.get $frame))))
  (local.set $kept (ref.cast (ref null $env) (array.get $values (local.get $saved) (i32.const 0))))
  (block $captured (result (ref $cap) exnref)
    (return (try_table (result eqref) (catch_ref $capture $captured)
      (call $tramp_run
        (ref.cast (ref null $sub) (array.get $values (local.get $saved) (i32.const 1)))
        (local.get $kept) (local.get $v) (local.get $mode)))))
  (local.set $x)
  (local.set $c)
  (call $capture_pack (local.get $c) (local.get $x) (ref.func $keep_reenter) (local.get $kept))
  unreachable)

;; Request waits use the same one-root capture and re-entry machinery.
(func $request_handoff (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env))
    (param $k eqref) (result eqref)
  (call $host_wait (local.get $k)))
(func $request_pause (param $status i32) (param $env (ref null $env)) (result eqref)
  (if (i32.eqz (global.get $async_live)) (then (call $request_fail (i32.const 4)) unreachable))
  (if (i32.and (i32.eq (local.get $status) (i32.const 1)) (i32.eqz (global.get $req_total)))
    (then unreachable))
  (global.set $host_pending_status (local.get $status))
  (return_call $control_suspend (call $host_root_boundary)
    (struct.new $clo1 (i32.const 7) (i32.const 0) (i64.const 1) (f64.const 0)
      (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)
      (ref.func $request_handoff))))
(func $request_wait_loop (param $tickets eqref) (param $env (ref null $env)) (result eqref)
  (if (call $request_any_settled (local.get $tickets)) (then (return (ref.null eq))))
  (block $captured (result (ref $cap) exnref)
    (drop (try_table (result eqref) (catch_ref $capture $captured)
      (call $request_pause (i32.const 1) (local.get $env))))
    (return_call $request_wait_loop (local.get $tickets) (local.get $env)))
  (call $capture_rethrow
    (struct.new $frame (ref.func $wait_requests_reenter)
      (array.new_fixed $values 1 (local.get $tickets)) (i32.const -1) (i32.const 5) (ref.null $values)))
  unreachable)
(func $wait_requests_reenter (type $refn) (param $frame (ref $frame)) (param $env (ref null $env))
    (param $value eqref) (param $mode i32) (result eqref)
  (return_call $request_wait_loop
    (array.get $values (ref.as_non_null (struct.get $frame 1 (local.get $frame))) (i32.const 0))
    (local.get $env)))
(func $request_wait_any (param $tickets eqref) (param $env (ref null $env)) (result eqref)
  (if (call $list_is_empty (local.get $tickets)) (then (call $request_fail (i32.const 3)) unreachable))
  (return_call $request_wait_loop (call $request_unique (local.get $tickets)) (local.get $env)))
(func $request_poll (param $env (ref null $env)) (result eqref)
  (return_call $request_pause (i32.const 2) (local.get $env)))

"#;

/// The types of control (§9.4), after `$env`.
pub(super) const TYPES: &str = r#"
(type $boundary (sub final $obj (struct (field (mut i32)) (field i32) (field i64) (field f64)
  (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs))
  (field (ref null $call)) (field (mut i32)) (field (mut eqref)) (field (mut i32))
  ;; 11 for a light boundary that stands for a provided value: the function
  ;; that makes the value from the boundary; 12 the value once made.
  (field (mut eqref)) (field (mut eqref)))))
;; The value of `$error` for `control.escape(b, v)`: the boundary and the value.
(type $escaped (struct (field (ref $boundary)) (field eqref)))
(rec
  ;; 0 re-entry function, 1 saved values, 2 shadow slot, 3 kind, 4 delta (kind 1 only)
  (type $frame (struct (field (ref $refn)) (field (ref null $values)) (field i32) (field i32)
    (field (ref null $values))))
  ;; record, environment it is given, value, mode (0 value, 1 error, 2 cancel)
  (type $refn (func (param (ref $frame) (ref null $env) eqref i32) (result eqref)))
  ;; 0 record, 1 next record outward, 2 next kind-1 record outward,
  ;; 3 environment given to this record (set at resume), 4 state (0 pending, 1 taken)
  (type $flist (struct (field (ref $frame)) (field (mut (ref null $flist)))
    (field (mut (ref null $flist))) (field (mut (ref null $env))) (field (mut i32)))))
;; The records of a paused computation: first and last record, first and last kind-1 record.
(type $sub (struct (field (ref null $flist)) (field (ref null $flist))
  (field (ref null $flist)) (field (ref null $flist))))
(type $cont (sub final $obj (struct (field (mut i32)) (field i32) (field i64) (field f64)
  (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs))
  (field (ref null $call)) (field (mut i32)) (field (ref $boundary)) (field (ref null $sub)) (field (mut eqref)))))
;; boundary, f, first, last, first kind-1, last kind-1
(type $cap (struct (field (ref $boundary)) (field eqref)
  (field (mut (ref null $flist))) (field (mut (ref null $flist)))
  (field (mut (ref null $flist))) (field (mut (ref null $flist)))))
;; depth of the shadow stack where it runs, its pending records, the trampoline outside it
(type $tramp (struct (field i32) (field (mut (ref null $flist))) (field (ref null $tramp))))
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
        out.push_str(&self.restore_scalar(dst));
    }

    /// Goes on with the segment of the point `q` (§9.4).
    pub(super) fn continue_point(&mut self, out: &mut String, q: PointId) {
        let program: &'p Program = self.p;
        let point = &program.points[q as usize];
        let args: String = point
            .params
            .iter()
            .map(|l| format!(" {}", self.atom(&Atom::Local(*l))))
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
            .map(|l| format!(" {}", self.atom(&Atom::Local(*l))))
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
    /// locals then the values of the environment statements it rebuilds
    /// (numbers are boxed), the slot of its frame on the shadow stack, its
    /// kind, and, for kind 1, the pairs `[service, value]` of the
    /// environment statements between the function's environment and the
    /// environment of the call, outermost first (`Point::delta`).
    pub(super) fn record(&mut self, p: PointId) -> String {
        let program: &'p Program = self.p;
        let point = &program.points[p as usize];
        let f = self.func_of(point);
        assert!(f.frame, "a function with a point has a frame");
        let locals = &f.locals;
        let mut items = Vec::new();
        for l in point.saved(locals) {
            items.push(match locals[l as usize] {
                LocalType::Value => self.atom(&Atom::Local(l)),
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
        let saved = if items.is_empty() {
            "(ref.null $values)".to_string()
        } else {
            format!(
                "(array.new_fixed $values {} {})",
                items.len(),
                items.join(" ")
            )
        };
        let delta = point
            .delta()
            .expect("control.rs checks that the environment of a point is in its links");
        let (kind, delta) = if delta.is_empty() {
            (0, "(ref.null $values)".to_string())
        } else {
            let pairs: Vec<String> = delta
                .iter()
                .map(|link| {
                    format!(
                        "(ref.i31 (i32.const {s})) (array.get $env (local.get $l{}) (i32.const {s}))",
                        link.dst,
                        s = link.service
                    )
                })
                .collect();
            (
                1,
                format!(
                    "(array.new_fixed $values {} {})",
                    2 * pairs.len(),
                    pairs.join(" ")
                ),
            )
        };
        format!(
            "(struct.new $frame (ref.func $r{p}) {saved} (local.get $slot) (i32.const {kind}) {delta})"
        )
    }

    /// A function that goes on with the rest of the function `f`: a
    /// segment of a point, with the value of its call `dst`, or a join.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn rest_func(
        &mut self,
        name: &str,
        id: FuncId,
        f: &Func,
        params: &[Local],
        dst: Option<Local>,
        locals: &[Local],
        code: &[Stmt],
    ) {
        self.numeric_plan = if self.p.numeric == crate::Numeric::Scalars && !self.metered {
            numeric::Plan::for_func(f, self.p.numeric_locals.get(&id), self.p.workers.get(&id))
        } else {
            numeric::Plan::default()
        };
        self.numeric_enabled =
            self.p.numeric != crate::Numeric::Reference && numeric::has_float(&f.body);
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
        if self.p.labels.len() > super::FIXED_MAX / 2 {
            self.out
                .push_str(" (local $record_pairs (ref null $values))");
        }
        if f.frame {
            self.out.push_str(" (local $slot i32)");
        }
        for &l in locals {
            let _ = write!(self.out, " (local $l{l} {})", ty(l));
        }
        self.out.push('\n');
        self.out.push_str(&self.numeric_plan.declarations());
        if self.numeric_enabled {
            self.out.push_str(
                " (local $numeric_a f64) (local $numeric_b f64) (local $numeric_result f64)\n",
            );
        }
        for (j, &l) in params.iter().enumerate() {
            let _ = writeln!(self.out, "(local.set $l{l} (local.get $a{j}))");
        }
        if let Some(dst) = dst {
            let _ = writeln!(self.out, "(local.set $l{dst} (local.get $res))");
        }
        for &l in params.iter().chain(dst.iter()) {
            self.out.push_str(&self.restore_scalar(l));
        }
        let mut body = String::new();
        self.stmts(&mut body, code, f.frame);
        self.out.push_str(&body);
        self.out.push_str("(unreachable))\n");
        self.numeric_plan = numeric::Plan::default();
        self.numeric_enabled = false;
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
            join.func,
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
            point.func,
            f,
            &point.params,
            Some(point.dst),
            &point.locals,
            &point.segment,
        );
        // The re-entry: rebuild this one frame and go on with the segment.
        // The frames inside it have returned (`$tramp_run`).
        assert!(f.frame, "a function with a point has a frame");
        let _ = write!(
            self.out,
            "(func $r{p} (type $refn) (param $frame (ref $frame)) (param $env (ref null $env)) \
             (param $v eqref) (param $mode i32) (result eqref)\n \
             (local $saved (ref null $values))"
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
        // environment that the frame is given.
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
        if self.metered {
            let _ = writeln!(
                self.out,
                "(call $macro_enter (i32.const {}))",
                f.budget_frame
            );
        }
        self.out
            .push_str("(call $shadow_enter (struct.get $frame 2 (local.get $frame)))\n");
        let args: String = point
            .params
            .iter()
            .map(|l| format!(" {}", self.atom(&Atom::Local(*l))))
            .collect();
        let _ = writeln!(
            self.out,
            "(return_call $s{p} (local.get $env){args} (local.get $v)))"
        );
    }
}
