//! The host boundary in the generated module (§9.7, §10.2, B.4, B.5):
//! import calls and their answers, async entry points, and the
//! `ilang_resume:…` exports. It calls control (`control.rs`) only through
//! `$host_boundary` and `$host_resume_k`.

use super::*;

/// One host continuation drives all pending request cells. The registry below
/// counts live Root/Owner groups separately from the home worker's typed cells.
/// Only outstanding cells occupy slots; Ready answers belong to their handles.
///
/// Async adapters call ordinary request.await_owned. Root wait/poll uses the
/// existing host/control seam; answers are decoded before resuming that root.
/// Status and async-live are separate: an entry can yield with no requests.
pub(super) const EFFECTS: &str = r#"
;; The host boundary (§9.7, B.4, B.5). The notes for stream R2 are on
;; `EFFECTS` in emit/host.rs.
(global $host_root (mut (ref null $boundary)) (ref.null $boundary))
(global $host_k (mut eqref) (ref.null eq))
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
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call) (i32.const 1) (ref.null eq) (i32.const 0)))
  (global.set $host_root (local.get $b))
  (return_call $boundary_run (local.get $b) (local.get $env) (local.get $body) (ref.null $flist)
    (ref.null eq) (ref.null eq) (ref.null $frame)))

;; Typed cells, free slots and group accounting are defined in REQUESTS.






;; The entry point has ended: the host drops what is left (B.5), so forget it.


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
  (global.set $host_last_status (i32.const 0))
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
      (if (ref.eq (local.get $r) (global.get $host_waiting)) (then
        (if (i32.and (i32.eq (global.get $host_pending_status) (i32.const 1)) (i32.eqz (call $req_outstanding)))
          (then unreachable))
        (global.set $host_last_status (global.get $host_pending_status))
        (return (global.get $host_pending_status))))
      (global.set $async_value (local.get $r))
      (call $req_clear)
      (global.set $async_live (i32.const 0))
      (global.set $host_last_status (i32.const 0))
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
      (global.set $host_last_status (i32.const 0))
  (i32.const 0))
"#;

/// Request cells and Root/Owner registries are private runtime storage.
pub(super) const TYPES: &str = r#"(type $waiting (struct (field i32)))
(type $request_root (struct (field eqref) (field (mut i32))))
(type $request_owner (struct (field eqref) (field (ref $request_root)) (field (mut i32))))
(type $request (sub final $obj (struct
  (field (mut i32)) (field i32) (field i64) (field f64)
  (field (ref null $bytes)) (field (ref null $values))
  (field (ref null $limbs)) (field (ref null $call))
  ;; state, answer, slot, generation, owner, observer kind, observer target
  ;; initiating trace, nominal result identity
  (field (mut i32)) (field (mut eqref)) (field (mut i32)) (field eqref)
  (field (mut (ref null $request_owner))) (field (mut i32)) (field (mut eqref))
  (field (mut (ref null $frames))) (field (mut i32)))))
(type $request_slot (struct (field (mut i32)) (field (mut eqref))
  (field (mut (ref null $request))) (field (mut i32))))
(type $request_slots (array (mut (ref null $request_slot))))
"#;

const REQUESTS: &str = r#"(global $req_roots (mut eqref) (ref.null eq))
(global $req_owners (mut eqref) (ref.null eq))
(global $req_observers (mut eqref) (ref.null eq))
(global $req_slots (mut (ref null $request_slots)) (ref.null $request_slots))
(global $req_size (mut i32) (i32.const 0))
(global $req_free (mut i32) (i32.const -1))
(global $req_total (mut i32) (i32.const 0))
(global $req_entry_live (mut i32) (i32.const 0))

(func $request_fail (param $reason i32)
  (if (i32.eq (local.get $reason) (i32.const 0)) (then (call $panic {{cancelled}}) unreachable))
  (if (i32.eq (local.get $reason) (i32.const 1)) (then (call $panic {{taken}}) unreachable))
  (if (i32.eq (local.get $reason) (i32.const 2)) (then (call $panic {{not_ready}}) unreachable))
  (if (i32.eq (local.get $reason) (i32.const 3)) (then (call $panic {{empty_wait}}) unreachable))
  (if (i32.eq (local.get $reason) (i32.const 4)) (then (call $panic {{no_entry}}) unreachable))
  (if (i32.eq (local.get $reason) (i32.const 5)) (then (call $panic {{root_gone}}) unreachable))
  (call $panic {{owner_gone}})
  unreachable)

(func $request_new (param $state i32) (param $answer eqref) (result (ref $request))
  (struct.new $request (i32.const 17) (i32.const 0) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)
    (local.get $state) (local.get $answer) (i32.const -1) (call $key_new)
    (ref.null $request_owner) (i32.const 0) (ref.null eq) (ref.null $frames) (i32.const 0)))
(func $request_ready (param $value eqref) (result eqref)
  (call $request_new (i32.const 1) (local.get $value)))
(func $request_ticket (param $value eqref) (result eqref)
  (struct.new $obj (i32.const 18) (i32.const 0) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (array.new_fixed $values 1 (local.get $value))
    (ref.null $limbs) (ref.null $call)))
(func $request_cell (param $ticket eqref) (result (ref $request))
  (ref.cast (ref $request) (call $field (local.get $ticket) (i32.const 0))))
(func $request_settled (param $ticket eqref) (result eqref)
  (call $bool_new (struct.get $request 8 (call $request_cell (local.get $ticket)))))
(func $request_state (param $value eqref) (result eqref)
  (local $r (ref $request)) (local $s i32)
  (local.set $r (ref.cast (ref $request) (local.get $value)))
  (local.set $s (struct.get $request 8 (local.get $r)))
  (if (i32.eq (local.get $s) (i32.const 1))
    (then (return_call $result_ok (struct.get $request 9 (local.get $r)))))
  (call $result_error (ref.i31 (select (i32.sub (local.get $s) (i32.const 1))
    (i32.const 0) (local.get $s)))))
(func $request_take (param $value eqref) (result eqref)
  (local $r (ref $request)) (local $s i32) (local $answer eqref)
  (local.set $r (ref.cast (ref $request) (local.get $value)))
  (local.set $s (struct.get $request 8 (local.get $r)))
  (if (i32.ne (local.get $s) (i32.const 1)) (then
    (call $request_fail (select (i32.const 2) (i32.sub (local.get $s) (i32.const 2))
      (i32.eqz (local.get $s)))) unreachable))
  (local.set $answer (struct.get $request 9 (local.get $r)))
  (struct.set $request 8 (local.get $r) (i32.const 3))
  (struct.set $request 9 (local.get $r) (ref.null eq))
  (local.get $answer))

(func $request_find_root (param $key eqref) (result (ref null $request_root))
  (local $xs eqref) (local $r (ref $request_root))
  (local.set $xs (global.get $req_roots))
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $xs)))
    (local.set $r (ref.cast (ref $request_root) (call $list_head (local.get $xs))))
    (if (ref.eq (local.get $key) (struct.get $request_root 0 (local.get $r)))
      (then (return (local.get $r))))
    (local.set $xs (call $list_tail (local.get $xs))) (br $each)))
  (ref.null $request_root))
(func $request_find_owner (param $key eqref) (result (ref null $request_owner))
  (local $xs eqref) (local $o (ref $request_owner))
  (local.set $xs (global.get $req_owners))
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $xs)))
    (local.set $o (ref.cast (ref $request_owner) (call $list_head (local.get $xs))))
    (if (ref.eq (local.get $key) (struct.get $request_owner 0 (local.get $o)))
      (then (return (local.get $o))))
    (local.set $xs (call $list_tail (local.get $xs))) (br $each)))
  (ref.null $request_owner))
(func $request_check_owner (param $owner eqref) (result eqref)
  (if (ref.is_null (call $request_find_owner (call $field (local.get $owner) (i32.const 0))))
    (then (call $request_fail (i32.const 6)) unreachable))
  (ref.null eq))

(func $request_add_root (result eqref)
  (local $key eqref)
  (local.set $key (call $key_new))
  (global.set $req_roots (call $list_cons
    (struct.new $request_root (local.get $key) (i32.const 0)) (global.get $req_roots)))
  (local.get $key))
(func $request_entry_begin (result eqref)
  (if (global.get $req_entry_live) (then unreachable))
  (global.set $req_entry_live (i32.const 1))
  (ref.null eq))
(func $request_entry_root (result eqref)
  (if (i32.eqz (global.get $req_entry_live)) (then unreachable))
  (call $request_add_root))
(func $request_open_root (param $parent eqref) (result eqref)
  (if (ref.is_null (call $request_find_root (local.get $parent)))
    (then (call $request_fail (i32.const 5)) unreachable))
  (call $request_add_root))
(func $request_open_owner (param $key eqref) (result eqref)
  (local $r (ref null $request_root)) (local $new eqref)
  (local.set $r (call $request_find_root (local.get $key)))
  (if (ref.is_null (local.get $r)) (then (call $request_fail (i32.const 5)) unreachable))
  (local.set $new (call $key_new))
  (global.set $req_owners (call $list_cons
    (struct.new $request_owner (local.get $new) (ref.as_non_null (local.get $r)) (i32.const 0))
    (global.get $req_owners)))
  (local.get $new))
(func $request_count_i32 (param $key eqref) (result i32)
  (local $o (ref null $request_owner))
  (local.set $o (call $request_find_owner (local.get $key)))
  (if (ref.is_null (local.get $o)) (then (return (i32.const 0))))
  (struct.get $request_owner 2 (ref.as_non_null (local.get $o))))
(func $request_count_root_i32 (param $key eqref) (result i32)
  (local $r (ref null $request_root))
  (local.set $r (call $request_find_root (local.get $key)))
  (if (ref.is_null (local.get $r)) (then (return (i32.const 0))))
  (struct.get $request_root 1 (ref.as_non_null (local.get $r))))
(func $request_count (param $key eqref) (result eqref)
  (call $int_from_i64 (i64.extend_i32_u (call $request_count_i32 (local.get $key)))))
(func $request_count_root (param $key eqref) (result eqref)
  (call $int_from_i64 (i64.extend_i32_u (call $request_count_root_i32 (local.get $key)))))

;; Notification never calls i code. An observer is not a producer and is
;; absent from every Root/Owner count.
(func $request_notify (param $owner eqref) (param $root eqref)
  (local $xs eqref) (local $keep eqref) (local $r (ref $request))
  (local $kind i32) (local $ready i32) (local $target eqref)
  (local.set $xs (global.get $req_observers))
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $xs)))
    (local.set $r (ref.cast (ref $request) (call $list_head (local.get $xs))))
    (local.set $kind (struct.get $request 13 (local.get $r)))
    (local.set $target (struct.get $request 14 (local.get $r)))
    (local.set $ready (i32.const 0))
    (if (i32.eq (local.get $kind) (i32.const 1)) (then
      (local.set $ready (i32.eqz (call $request_count_i32 (local.get $target))))))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then
      (local.set $ready (i32.eqz (call $request_count_root_i32 (local.get $target))))))
    (if (i32.eq (local.get $kind) (i32.const 3)) (then
      (local.set $ready (ref.eq (local.get $root) (local.get $target)))))
    (if (local.get $ready)
      (then
        (struct.set $request 8 (local.get $r) (i32.const 1))
        (struct.set $request 13 (local.get $r) (i32.const 0))
        (struct.set $request 14 (local.get $r) (ref.null eq)))
      (else (local.set $keep (call $list_cons (local.get $r) (local.get $keep)))))
    (local.set $xs (call $list_tail (local.get $xs))) (br $each)))
  (global.set $req_observers (local.get $keep)))

(func $request_observe (param $key eqref) (param $kind i32) (result eqref)
  (local $r (ref $request)) (local $count i32)
  (local.set $count (if (result i32) (i32.eq (local.get $kind) (i32.const 1))
    (then (call $request_count_i32 (local.get $key)))
    (else (call $request_count_root_i32 (local.get $key)))))
  (if (i32.eqz (local.get $count)) (then (return_call $request_ready (ref.null eq))))
  (local.set $r (call $request_new (i32.const 0) (ref.null eq)))
  (struct.set $request 13 (local.get $r) (local.get $kind))
  (struct.set $request 14 (local.get $r) (local.get $key))
  (global.set $req_observers (call $list_cons (local.get $r) (global.get $req_observers)))
  (local.get $r))
(func $request_observe_owner (param $key eqref) (result eqref)
  (call $request_observe (local.get $key) (i32.const 1)))
(func $request_observe_root (param $key eqref) (result eqref)
  (call $request_observe (local.get $key) (i32.const 2)))
(func $request_observe_change (param $key eqref) (result eqref)
  (call $request_observe (local.get $key) (i32.const 3)))

(func $request_forget_observer (param $r (ref $request))
  (local $xs eqref) (local $keep eqref) (local $v eqref)
  (local.set $xs (global.get $req_observers))
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $xs)))
    (local.set $v (call $list_head (local.get $xs)))
    (if (i32.eqz (ref.eq (local.get $v) (local.get $r)))
      (then (local.set $keep (call $list_cons (local.get $v) (local.get $keep)))))
    (local.set $xs (call $list_tail (local.get $xs))) (br $each)))
  (global.set $req_observers (local.get $keep))
  (struct.set $request 13 (local.get $r) (i32.const 0))
  (struct.set $request 14 (local.get $r) (ref.null eq)))

;; Free slots retain no cell or generation. A stale handle cannot act on a
;; reused index: retire checks both the cell and its fresh generation token.
(func $request_slot_new (param $r (ref $request)) (param $import i32) (result i32)
  (local $index i32) (local $s (ref null $request_slot))
  (local $slots (ref $request_slots)) (local $grown (ref $request_slots)) (local $n i32)
  (if (i32.ge_s (global.get $req_free) (i32.const 0))
    (then
      (local.set $index (global.get $req_free))
      (local.set $s (ref.as_non_null (array.get $request_slots (global.get $req_slots) (local.get $index))))
      (global.set $req_free (struct.get $request_slot 3 (ref.as_non_null (local.get $s)))))
    (else
      (local.set $index (global.get $req_size))
      (if (i32.eq (local.get $index) (i32.const 0x7fffffff)) (then unreachable))
      (if (ref.is_null (global.get $req_slots))
        (then (global.set $req_slots (array.new_default $request_slots (i32.const 16)))))
      (local.set $slots (ref.as_non_null (global.get $req_slots)))
      (local.set $n (array.len (local.get $slots)))
      (if (i32.ge_u (local.get $index) (local.get $n)) (then
        (if (i32.ge_u (local.get $n) (i32.const 0x40000000)) (then unreachable))
        (local.set $grown (array.new_default $request_slots (i32.shl (local.get $n) (i32.const 1))))
        (array.copy $request_slots $request_slots (local.get $grown) (i32.const 0)
          (local.get $slots) (i32.const 0) (local.get $n))
        (global.set $req_slots (local.get $grown))))
      (local.set $s (struct.new $request_slot (i32.const -1) (ref.null eq) (ref.null $request) (i32.const -1)))
      (array.set $request_slots (global.get $req_slots) (local.get $index) (ref.as_non_null (local.get $s)))
      (global.set $req_size (i32.add (local.get $index) (i32.const 1)))))
  (struct.set $request_slot 0 (ref.as_non_null (local.get $s)) (local.get $import))
  (struct.set $request_slot 1 (ref.as_non_null (local.get $s)) (struct.get $request 11 (local.get $r)))
  (struct.set $request_slot 2 (ref.as_non_null (local.get $s)) (local.get $r))
  (struct.set $request 10 (local.get $r) (local.get $index))
  (global.set $req_total (i32.add (global.get $req_total) (i32.const 1)))
  (local.get $index))

(func $req_start (param $import i32) (param $owner eqref) (param $nominal i32) (result (ref $request))
  (local $o (ref null $request_owner)) (local $root (ref $request_root)) (local $r (ref $request))
  (if (i32.eqz (global.get $async_live)) (then (call $request_fail (i32.const 4)) unreachable))
  (local.set $o (call $request_find_owner (call $field (local.get $owner) (i32.const 0))))
  (if (ref.is_null (local.get $o)) (then (call $request_fail (i32.const 6)) unreachable))
  (local.set $root (struct.get $request_owner 1 (ref.as_non_null (local.get $o))))
  (local.set $r (call $request_new (i32.const 0) (ref.null eq)))
  (struct.set $request 12 (local.get $r) (local.get $o))
  (struct.set $request 15 (local.get $r) (call $shadow_snapshot))
  (struct.set $request 16 (local.get $r) (local.get $nominal))
  (drop (call $request_slot_new (local.get $r) (local.get $import)))
  (struct.set $request_owner 2 (ref.as_non_null (local.get $o))
    (i32.add (struct.get $request_owner 2 (ref.as_non_null (local.get $o))) (i32.const 1)))
  (struct.set $request_root 1 (local.get $root)
    (i32.add (struct.get $request_root 1 (local.get $root)) (i32.const 1)))
  (call $request_notify (struct.get $request_owner 0 (ref.as_non_null (local.get $o)))
    (struct.get $request_root 0 (local.get $root)))
  (local.get $r))

(func $request_retire (param $r (ref $request))
  (local $id i32) (local $s (ref $request_slot)) (local $o (ref $request_owner))
  (local $root (ref $request_root))
  (local.set $id (struct.get $request 10 (local.get $r)))
  (if (i32.lt_s (local.get $id) (i32.const 0)) (then (return)))
  (local.set $s (ref.as_non_null (array.get $request_slots (global.get $req_slots) (local.get $id))))
  (if (i32.eqz (i32.and (ref.eq (struct.get $request_slot 2 (local.get $s)) (local.get $r))
    (ref.eq (struct.get $request_slot 1 (local.get $s)) (struct.get $request 11 (local.get $r)))))
    (then unreachable))
  (struct.set $request_slot 2 (local.get $s) (ref.null $request))
  (struct.set $request_slot 1 (local.get $s) (ref.null eq))
  (struct.set $request_slot 0 (local.get $s) (i32.const -1))
  (struct.set $request_slot 3 (local.get $s) (global.get $req_free))
  (global.set $req_free (local.get $id))
  (global.set $req_total (i32.sub (global.get $req_total) (i32.const 1)))
  (struct.set $request 10 (local.get $r) (i32.const -1))
  (local.set $o (ref.as_non_null (struct.get $request 12 (local.get $r))))
  (local.set $root (struct.get $request_owner 1 (local.get $o)))
  (struct.set $request_owner 2 (local.get $o)
    (i32.sub (struct.get $request_owner 2 (local.get $o)) (i32.const 1)))
  (struct.set $request_root 1 (local.get $root)
    (i32.sub (struct.get $request_root 1 (local.get $root)) (i32.const 1)))
  (struct.set $request 12 (local.get $r) (ref.null $request_owner))
  (struct.set $request 15 (local.get $r) (ref.null $frames))
  (call $request_notify (struct.get $request_owner 0 (local.get $o))
    (struct.get $request_root 0 (local.get $root))))

;; A wrong import/id is a bare host fault, before any decoding or delivery.
(func $req_answer (param $import i32) (param $id i32) (result (ref $request))
  (local $s (ref $request_slot))
  (if (i32.ge_u (local.get $id) (global.get $req_size)) (then unreachable))
  (local.set $s (ref.as_non_null (array.get $request_slots (global.get $req_slots) (local.get $id))))
  (if (i32.or (i32.ne (struct.get $request_slot 0 (local.get $s)) (local.get $import))
      (ref.is_null (struct.get $request_slot 2 (local.get $s)))) (then unreachable))
  (ref.as_non_null (struct.get $request_slot 2 (local.get $s))))
(func $request_deliver (param $r (ref $request)) (param $answer eqref)
  (if (struct.get $request 8 (local.get $r)) (then unreachable))
  (struct.set $request 9 (local.get $r) (local.get $answer))
  (struct.set $request 8 (local.get $r) (i32.const 1))
  (call $request_retire (local.get $r)))
(func $req_outstanding (result i32) (global.get $req_total))

(func $request_check_slot (param $r (ref $request))
  (local $s (ref $request_slot))
  (local.set $s (ref.as_non_null (array.get $request_slots (global.get $req_slots)
    (struct.get $request 10 (local.get $r)))))
  (if (i32.eqz (i32.and (ref.eq (struct.get $request_slot 2 (local.get $s)) (local.get $r))
    (ref.eq (struct.get $request_slot 1 (local.get $s)) (struct.get $request 11 (local.get $r)))))
    (then unreachable)))

(func $request_cancel_cell (param $r (ref $request))
  (local $state i32) (local $id i32)
  (local.set $state (struct.get $request 8 (local.get $r)))
  (if (i32.ge_u (local.get $state) (i32.const 2)) (then (return)))
  (local.set $id (struct.get $request 10 (local.get $r)))
  (if (i32.ge_s (local.get $id) (i32.const 0)) (then
    (call $request_check_slot (local.get $r))
    (call $host_cancel (local.get $id))
    (struct.set $request 8 (local.get $r) (i32.const 2))
    (call $request_retire (local.get $r))))
  (if (struct.get $request 13 (local.get $r)) (then (call $request_forget_observer (local.get $r))))
  (struct.set $request 8 (local.get $r) (i32.const 2))
  (struct.set $request 9 (local.get $r) (ref.null eq)))
(func $request_cancel (param $ticket eqref) (result eqref)
  (call $request_cancel_cell (call $request_cell (local.get $ticket)))
  (ref.null eq))

(func $request_cancel_members (param $key eqref) (param $root i32)
  (local $i i32) (local $r (ref null $request)) (local $owner (ref $request_owner))
  (local $target eqref)
  (block $done (loop $each
    (br_if $done (i32.ge_u (local.get $i) (global.get $req_size)))
    (local.set $r (struct.get $request_slot 2
      (ref.as_non_null (array.get $request_slots (global.get $req_slots) (local.get $i)))))
    (if (i32.eqz (ref.is_null (local.get $r))) (then
      (local.set $owner (ref.as_non_null (struct.get $request 12 (ref.as_non_null (local.get $r)))))
      (local.set $target (if (result eqref) (local.get $root)
        (then (struct.get $request_root 0 (struct.get $request_owner 1 (local.get $owner))))
        (else (struct.get $request_owner 0 (local.get $owner)))))
      (if (ref.eq (local.get $key) (local.get $target))
        (then (call $request_cancel_cell (ref.as_non_null (local.get $r)))))))
    (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $each))))
(func $request_cancel_all (param $key eqref) (result eqref)
  (call $request_cancel_members (local.get $key) (i32.const 0)) (ref.null eq))
(func $request_cancel_all_root (param $key eqref) (result eqref)
  (call $request_cancel_members (local.get $key) (i32.const 1)) (ref.null eq))

(func $request_close_owner (param $key eqref) (result eqref)
  (local $xs eqref) (local $keep eqref) (local $o (ref $request_owner))
  (call $request_cancel_members (local.get $key) (i32.const 0))
  (local.set $xs (global.get $req_owners))
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $xs)))
    (local.set $o (ref.cast (ref $request_owner) (call $list_head (local.get $xs))))
    (if (i32.eqz (ref.eq (local.get $key) (struct.get $request_owner 0 (local.get $o))))
      (then (local.set $keep (call $list_cons (local.get $o) (local.get $keep)))))
    (local.set $xs (call $list_tail (local.get $xs))) (br $each)))
  (global.set $req_owners (local.get $keep))
  (ref.null eq))
(func $request_close_root (param $key eqref) (result eqref)
  (local $xs eqref) (local $keep eqref) (local $r (ref $request_root)) (local $o (ref $request_owner))
  (call $request_cancel_members (local.get $key) (i32.const 1))
  (local.set $xs (global.get $req_owners))
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $xs)))
    (local.set $o (ref.cast (ref $request_owner) (call $list_head (local.get $xs))))
    (if (i32.eqz (ref.eq (local.get $key)
        (struct.get $request_root 0 (struct.get $request_owner 1 (local.get $o)))))
      (then (local.set $keep (call $list_cons (local.get $o) (local.get $keep)))))
    (local.set $xs (call $list_tail (local.get $xs))) (br $each)))
  (global.set $req_owners (local.get $keep))
  (local.set $keep (ref.null eq))
  (local.set $xs (global.get $req_roots))
  (block $done (loop $each
    (br_if $done (ref.is_null (local.get $xs)))
    (local.set $r (ref.cast (ref $request_root) (call $list_head (local.get $xs))))
    (if (i32.eqz (ref.eq (local.get $key) (struct.get $request_root 0 (local.get $r))))
      (then (local.set $keep (call $list_cons (local.get $r) (local.get $keep)))))
    (local.set $xs (call $list_tail (local.get $xs))) (br $each)))
  (global.set $req_roots (local.get $keep))
  (ref.null eq))

;; Normal entry cleanup cancels every root, including forgotten independent
;; dispatchers. Status-0 clearing also detaches cells without retaining answers.
(func $request_entry_end (result eqref)
  (block $done (loop $each
    (br_if $done (ref.is_null (global.get $req_roots)))
    (drop (call $request_close_root (struct.get $request_root 0
      (ref.cast (ref $request_root) (call $list_head (global.get $req_roots))))))
    (br $each)))
  (global.set $req_entry_live (i32.const 0))
  (ref.null eq))
(func $req_clear
  (local $i i32) (local $r (ref null $request))
  (block $done (loop $each
    (br_if $done (i32.ge_u (local.get $i) (global.get $req_size)))
    (local.set $r (struct.get $request_slot 2
      (ref.as_non_null (array.get $request_slots (global.get $req_slots) (local.get $i)))))
    (if (i32.eqz (ref.is_null (local.get $r))) (then
      (struct.set $request 8 (ref.as_non_null (local.get $r)) (i32.const 2))
      (call $request_retire (ref.as_non_null (local.get $r)))))
    (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $each)))
  (global.set $req_roots (ref.null eq))
  (global.set $req_owners (ref.null eq))
  (global.set $req_observers (ref.null eq))
  (global.set $req_entry_live (i32.const 0)))

(func $request_any_settled (param $tickets eqref) (result i32)
  (block $done (loop $each
    (br_if $done (call $list_is_empty (local.get $tickets)))
    (if (struct.get $request 8 (call $request_cell (call $list_head (local.get $tickets))))
      (then (return (i32.const 1))))
    (local.set $tickets (call $list_tail (local.get $tickets))) (br $each)))
  (i32.const 0))
(func $request_unique (param $tickets eqref) (result eqref)
  (local $unique eqref) (local $check eqref) (local $ticket eqref)
  (local $cell (ref $request)) (local $found i32)
  (local.set $unique (call $list_nil))
  (block $done (loop $each
    (br_if $done (call $list_is_empty (local.get $tickets)))
    (local.set $ticket (call $list_head (local.get $tickets)))
    (local.set $cell (call $request_cell (local.get $ticket)))
    (local.set $found (i32.const 0))
    (local.set $check (local.get $unique))
    (block $checked (loop $scan
      (br_if $checked (call $list_is_empty (local.get $check)))
      (if (ref.eq (local.get $cell) (call $request_cell (call $list_head (local.get $check))))
        (then (local.set $found (i32.const 1)) (br $checked)))
      (local.set $check (call $list_tail (local.get $check))) (br $scan)))
    (if (i32.eqz (local.get $found))
      (then (local.set $unique (call $list_cons (local.get $ticket) (local.get $unique)))))
    (local.set $tickets (call $list_tail (local.get $tickets))) (br $each)))
  (local.get $unique))

;; Observers contribute their producers, never one unit for themselves.
(func $request_has_work (param $ticket eqref) (result eqref)
  (local $r (ref $request)) (local $kind i32)
  (local.set $r (call $request_cell (local.get $ticket)))
  (if (struct.get $request 8 (local.get $r)) (then (return (global.get $value_false))))
  (local.set $kind (struct.get $request 13 (local.get $r)))
  (if (i32.eqz (local.get $kind)) (then (return (call $bool_new
    (i32.ge_s (struct.get $request 10 (local.get $r)) (i32.const 0))))))
  (if (i32.eq (local.get $kind) (i32.const 1)) (then (return (call $bool_new
    (call $request_count_i32 (struct.get $request 14 (local.get $r)))))))
  (call $bool_new (call $request_count_root_i32 (struct.get $request 14 (local.get $r)))))

(global $host_pending_status (mut i32) (i32.const 1))
(global $host_last_status (mut i32) (i32.const 0))

(global $host_start_cell (mut (ref null $request)) (ref.null $request))
(global $host_decoded (mut eqref) (ref.null eq))
(func $host_start_done (param $r eqref) (result eqref)
  (global.set $host_start_cell (ref.null $request)) (local.get $r))
(func $host_decode_done (param $r eqref) (result eqref)
  (global.set $host_decoded (ref.null eq)) (local.get $r))
"#;

pub(super) fn effects() -> String {
    let mut out = EFFECTS.to_string();
    out.push_str(&runtime::with_literals(
        REQUESTS.to_string(),
        &[
            ("cancelled", "request: cancelled"),
            ("taken", "request: taken"),
            ("not_ready", "request: not ready"),
            ("empty_wait", "request: empty wait"),
            ("no_entry", "request: no active entry"),
            ("root_gone", "request: root is gone"),
            ("owner_gone", "request: owner is gone"),
        ],
    ));
    out.push_str(&runtime::with_literals(
        PROVIDERS.to_string(),
        &[
            ("provider_called", "provider: body already called"),
            ("provider_inactive", "provider: body is not active"),
            ("provider_incomplete", "provider: body was not completed"),
        ],
    ));
    out
}

const PROVIDERS: &str = r#"
;; An entry-lifetime continuation token. Its ordinary value header lets the
;; shared portability walk classify the generated body's capture as local.
(type $provider_guard (sub $obj (struct
  (field (mut i32)) (field i32) (field i64) (field f64)
  (field (ref null $bytes)) (field (ref null $values))
  (field (ref null $limbs)) (field (ref null $call))
  (field (mut i32)) (field (mut i32)))))
(func $provider_guard_new (result eqref)
  (struct.new $provider_guard (i32.const 19) (i32.const 0) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)
    (i32.const 1) (i32.const 0)))
(func $provider_guard_enter (param $value eqref) (result eqref)
  (local $g (ref $provider_guard))
  (local.set $g (ref.cast (ref $provider_guard) (local.get $value)))
  (if (i32.eqz (struct.get $provider_guard 8 (local.get $g)))
    (then (call $panic {{provider_inactive}}) unreachable))
  (if (struct.get $provider_guard 9 (local.get $g))
    (then (call $panic {{provider_called}}) unreachable))
  (struct.set $provider_guard 9 (local.get $g) (i32.const 1))
  (ref.null eq))
(func $provider_guard_complete (param $value eqref) (result eqref)
  (struct.set $provider_guard 9 (ref.cast (ref $provider_guard) (local.get $value)) (i32.const 2))
  (ref.null eq))
(func $provider_guard_leave (param $value eqref) (result eqref)
  (struct.set $provider_guard 8 (ref.cast (ref $provider_guard) (local.get $value)) (i32.const 0))
  (ref.null eq))
(func $provider_guard_check (param $value eqref) (result eqref)
  (if (i32.ne (struct.get $provider_guard 9 (ref.cast (ref $provider_guard) (local.get $value))) (i32.const 2))
    (then (call $panic {{provider_incomplete}}) unreachable))
  (ref.null eq))
"#;

impl<'p> Emitter<'p> {
    /// Starts an import of an async host service (B.4) with a new request
    /// id from the request table, returning the cell immediately.
    pub(super) fn host_start(
        &mut self,
        import: ImportId,
        args: &[Atom],
        result: AbiType,
    ) -> String {
        let nominal = match result {
            AbiType::Externref(id) => id,
            _ => 0,
        };
        let owner = self.p.request_owner.expect("request Owner is registered");
        let id = " (struct.get $request 10 (ref.as_non_null (global.get $host_start_cell)))";
        let call = self.host_call_with(import, id, args);
        format!(
            "(block (result eqref)\n(global.set $host_start_cell (call $req_start (i32.const {import}) (array.get $env (local.get $env) (i32.const {owner})) (i32.const {nominal})))\n{call}\n(call $host_start_done (global.get $host_start_cell)))"
        )
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
        if x.awaited {
            return self.host_answer(
                i,
                &match result {
                    AbiType::Externref(id) => format!("(i32.const {id})"),
                    _ => "(i32.const 0)".into(),
                },
            );
        }
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

    fn host_answer(&self, i: ImportId, nominal: &str) -> String {
        let x = &self.p.imports[i as usize];
        let decode = |t: AbiType, k: usize| -> String {
            let g = |j: usize| format!("(global.get $hr{i}_{})", k + j);
            match t {
                AbiType::Nil => "(ref.null eq)".to_string(),
                AbiType::Bool => format!("(call $abi_bool_in {})", g(0)),
                AbiType::Float => format!("(call $abi_float_in {})", g(0)),
                AbiType::Int => format!("(call $abi_int_in {} {} {})", g(0), g(1), g(2)),
                AbiType::String => format!("(call $abi_string_in {} {})", g(0), g(1)),
                AbiType::BitArray => format!("(call $abi_bits_in {} {})", g(0), g(1)),
                AbiType::Externref(_) => format!("(call $abi_ref_in {} {nominal})", g(0)),
            }
        };
        let mut offsets = Vec::new();
        let mut k = usize::from(!x.abi.errors.is_empty());
        for t in std::iter::once(x.abi.result).chain(x.abi.errors.iter().copied()) {
            offsets.push((t, k));
            k += abi_in(t).0.len();
        }
        let mut value = decode(offsets[0].0, offsets[0].1);
        if x.request_result {
            value = format!("(call $result_ok {value})");
        }
        for m in (1..offsets.len()).rev() {
            value = format!(
                "(if (result eqref) (i32.eq (global.get $hr{i}_0) (i32.const {m})) (then (call $result_error {})) (else {value}))",
                decode(offsets[m].0, offsets[m].1)
            );
        }
        let mut out = "(call $host_decode_done (block (result eqref)\n".to_string();
        if !x.abi.errors.is_empty() {
            let _ = writeln!(
                out,
                "(global.set $hr{i}_0 (call $abi_tag_in (global.get $hr{i}_0) (i32.const {})))",
                x.abi.errors.len()
            );
        }
        let _ = writeln!(out, "(global.set $host_decoded {value})");
        for (t, k) in offsets {
            let address = match t {
                AbiType::Int => Some(k + 1),
                AbiType::String | AbiType::BitArray => Some(k),
                _ => None,
            };
            if let Some(j) = address {
                let _ = writeln!(out, "(call $free (global.get $hr{i}_{j}) (i32.const 0))");
            }
        }
        for (j, t) in import_results(&x.abi).iter().enumerate() {
            let _ = writeln!(out, "(global.set $hr{i}_{j} {})", zero(t));
        }
        out.push_str("(global.get $host_decoded)))");
        out
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
            closure(1, body, &captures, false)
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
            let answer =
                self.host_answer(i as ImportId, "(struct.get $request 16 (local.get $cell))");
            let _ = writeln!(
                self.out,
                "(func (export \"ilang_resume:{}:{}\") (param $id i32){params} (result i32) (local $cell (ref $request)) (local $old_shadow (ref $frames)) (local $old_sp i32) (local $answer eqref)\n\
                 (if (i32.eqz (i32.and (global.get $async_live) (i32.gt_s (global.get $host_last_status) (i32.const 0)))) (then unreachable))\n\
                 (local.set $cell (call $req_answer (i32.const {i}) (local.get $id))){sets}\n\
                 (local.set $old_shadow (global.get $shadow)) (local.set $old_sp (global.get $sp))\n\
                 (global.set $shadow (ref.as_non_null (struct.get $request 15 (local.get $cell))))\n\
                 (global.set $sp (array.len (global.get $shadow)))\n\
                 (local.set $answer {answer})\n\
                 (global.set $shadow (local.get $old_shadow)) (global.set $sp (local.get $old_sp))\n\
                 (call $request_deliver (local.get $cell) (local.get $answer))\n\
                 (global.set $host_last_status (i32.const 0))\n\
                 (call $async_run (ref.null eq)))",
                wat_string(&x.module),
                wat_string(&x.name)
            );
        }
        if program.exports.iter().any(|e| e.async_body.is_some()) {
            self.out.push_str("(func (export \"ilang_continue\") (result i32)\n\
                (if (i32.eqz (i32.and (global.get $async_live) (i32.eq (global.get $host_last_status) (i32.const 2)))) (then unreachable))\n\
                (global.set $host_last_status (i32.const 0))\n\
                (call $async_run (ref.null eq)))\n");
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
