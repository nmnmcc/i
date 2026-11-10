# Design: lazy re-entry of paused frames

Status: draft for review (stream B, step B-D). Base commit `dcb19df`. This note changes no behaviour, so `docs/spec.md` does not change. Every program gives the same values, the same errors, the same stop messages and the same order of final functions as today. The one exception is the stop "There is no more memory or stack" (§11.7): the depth at which the Wasm stack runs out may move (section 7, last bullet). Only the cost of `control.resume` and of a suspend after a resume changes. `control.cancel` keeps the same order of cost as today: it still goes through every pending record (section 10).

The code this note talks about is at `dcb19df`:

- `crates/ilang-wasm/src/emit/control.rs`: the runtime text of control (`EFFECTS`, `TYPES`) and the code made for each point (`suspendable`, `record`, `point`).
- `crates/ilang-wasm/src/control.rs`: the pass that splits functions at points.
- `crates/ilang-wasm/src/ir.rs`: `Point`, `EnvLink`, `Env`, and the module doc about control.
- `crates/ilang-wasm/src/runtime/stops.rs`: the shadow stack and `$stop` (§11.7).
- `crates/ilang-wasm/src/emit/host.rs`: `$host_boundary`, `$host_resume_k`, `$async_run` (§9.7, Appendix B).

## 1. The problem

A suspend throws the exception `$capture`. Each frame it goes through adds a **frame record** to the capture and throws it again. The boundary turns the records into a continuation. Capture is already lazy: a frame pays only when a capture really goes through it.

Re-entry is not lazy. Today the list of records is outermost first. `control.resume` calls `$reenter_next`, which calls the re-entry function of the outermost record. Each re-entry function `$r{p}`:

1. rebuilds its locals and its environment,
2. pushes its slot on the shadow stack,
3. calls `$reenter_next` on the records inside it, under a `$capture` handler of its own, and only then
4. goes on with its segment `$s{p}`.

So a resume at depth N:

- makes N Wasm frames, N `try_table`s and N shadow pushes before the innermost frame runs a single step;
- and when the resumed code suspends again, the capture goes through all N re-entered frames, and each one adds its record again.

A generator that is paused under 1000 frames pays about 1000 steps for every value it gives. The goal of stream B is that resume and the next suspend cost the same at every depth.

## 2. The idea

Go re-enters a paused stack lazily. We do the same:

- The list of records becomes **innermost first**. A capture adds each new record at the tail.
- `resume` re-enters only the innermost record.
- A small runtime loop, the **trampoline** (`$tramp_run`), holds the records that are not re-entered yet (the **pending** records). When the frame it called ends, the trampoline gives the outcome (a value, an error or a cancel) to the next record. A record does exactly what its eager version did after its inner call ended in that way.
- When the resumed code suspends again, the capture holds only the frames entered since the resume. When it reaches the trampoline, the trampoline joins its pending records onto the capture in one step (a **splice**), and throws the capture on.
- A stop (§11.7) puts the pending frames back into the shadow stack before it prints, so the message is the same as today.

## 3. Decisions

Each decision has its reason. Where a review asked for a change, the reason says which choice was made.

**D1. The list is innermost first, with a tail pointer.** A capture appends at the tail in O(1). A splice of the pending records is then one pointer write. *Reason:* the trampoline takes records from the inner end, and a re-suspend must join the pending part without walking it.

**D2. Each record has a kind, and the shadow slot is a field of the record.** Kinds:

| Kind | Records | What the trampoline does with an error or a cancel |
|---|---|---|
| 0 | a code frame whose call uses the function's own environment | skips it, with no call |
| 1 | a code frame whose call is under an environment statement of its function (`point.env` is not `Env::Param`) | skips it, with no call, but takes its environment |
| 2 | a handler: `fail.catch`, `resource.ensure`, and the final function of `resource.ensure` | calls it |
| 3 | a **delimiter**: an inner boundary, or a cancel whose final function suspended | calls it |

*Reason:* a code frame has no handler, so an error or a cancel passes it with nothing to do. The slot moves out of the saved values so that a stop can read it without knowing the layout of the values.

**D3. The environment of §9.4 rule 3 is data on the record.** A kind-1 record keeps its **delta**: the pairs `[service, value]` of the environment statements between the function's environment and the environment of the call, outermost first. At resume, one walk over the kind-1 records works out the environment each record is given, with no call into code made by the compiler. *Reason (review):* asking `$r{p}` "what environment do you give" through a fourth mode mixes data into code. With data, the walk is plain runtime text, and `$refn` keeps three modes.

**D4. Inner boundaries and cancel records are re-entered at resume, by descent.** A delimiter is always the head of its list (section 5.4). Resume re-enters the delimiter at once, with the list inside it, which re-enters the next delimiter, and so on, until the innermost list, which has no delimiter. Each delimiter runs the existing boundary or cancel code, which already handles a capture for its own boundary, `f(k)` in its place, the active flag, and the `ref.eq` test of cancel. *Reason (review):* the other choices were a search over every live trampoline when a suspend names a boundary that is not active yet, or cutting the pending list at a pending boundary. Both are new paths with new invariants. Descent reuses code that already works. Its cost is O(D) per resume, where D is the number of boundaries the continuation is paused inside. The threshold is stated for D = 0.

**D5. A capture that reaches a delimiter is packed into it.** When a capture for another boundary goes through a boundary (or through a cancel), the boundary makes a new delimiter record that holds the whole list of the capture so far, and the capture's list becomes that one record. *Reason:* this keeps every delimiter at the head of its list (invariant D in section 6), so no list is ever cut, and the trampoline never meets a delimiter except as its first record. It costs O(1).

**D6. Pending frames reach a stop through one hook.** `$stop` calls `$control_unpend` through a nullable function reference in a global of `stops.rs`. `$control_unpend` (in `emit/control.rs`) builds a new shadow stack with the pending slots in their eager places. The walker in `$stop`, the "tail site only if first" rule, and `$stop_at` do not change. *Reason (review):* `runtime/stops.rs` is part of `runtime::wat()`, which `tests/runtime_values.rs` validates with only `runtime::TYPES`. A hook with a null default needs only one function type in the runtime types, and none of the control types. The trace is equal to eager by construction, because the rebuilt array is the eager array.

**D7. `$boundary_run` stays, as a shim for `emit/host.rs`.** The real function is `$boundary_core(b, env, body, sub, v, mode)`. `$boundary_run` keeps its 7 parameters, traps if `rest`, `cancel` or `frame` is not null, and calls `$boundary_core(b, env, body, null, v, 0)`. *Reason:* `emit/host.rs` keeps its text byte for byte (the seam of plan P1). No parameter is reused for two meanings (the `ref.test` pun on `body` was rejected).

**D8. An error or a cancel walks the pending records one at a time.** It does not jump over code records through a chain. *Reason:* the walk is a tight loop with no call for kinds 0 and 1. It costs what the unwind of the same frames costs today. A chain of handlers would add fields and a second splice for no gain on the paths the goal is about.

**D9. The CI check is a fuel count, and the pass rule is a wall-clock ratio measured by the coordinator.** `control_lazy.rs` runs the bench program in the `wasmtime` CLI with fuel, as `tests/runtime_values.rs` already runs `wasmtime`, and compares the extra fuel of one more round at depth 1000 and at depth 10. *Reason (review):* a wall-clock test in CI is noisy. Fuel is exact, and needs no new dependency: `ilang-host` is not added as a dev-dependency.

**D10. The bench resumes from a flat loop outside the boundary.** `f` returns the continuation in a value, and a tail-recursive driver resumes it. *Reason (review):* if `f` called `resume` itself, each round would run one boundary deeper, and the bench would measure nesting, not resume. The eager baseline would also hold about 10^6 frames at depth 1000 and might overflow.

**D11. The checks of the list invariants are always on.** Each node has a state field. The trampoline traps (`unreachable`) if it takes a node that was taken before, or meets a delimiter that is not the first record. *Reason:* a mistake in the list code would run a frame twice or lose one silently. The check costs one field read and one write per record. If the coordinator's measurement shows a cost, the checks move behind `cfg!(debug_assertions)` of the compiler (open question 3).

**D12. No change to `docs/spec.md`.** The spec states no costs, and every rule it states holds (section 7). The spec also gives no fixed stack size, so the depth at which "There is no more memory or stack" happens may move; that is the only behaviour that can differ.

## 4. Data

### 4.1 Types in `emit/control.rs` `TYPES`

`$boundary` does not change (host.rs builds it). The type names `$frame` and `$flist` stay, because host.rs writes `(ref.null $flist)` and `(ref.null $frame)`.

```wat
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
;; Fields 0 to 9 as today; field 10 is now the records.
(type $cont (sub final $obj (struct (field i32) (field i32) (field i64) (field f64)
  (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs))
  (field (ref null $call)) (field (mut i32)) (field (ref $boundary)) (field (ref null $sub)))))
;; boundary, f, first, last, first kind-1, last kind-1
(type $cap (struct (field (ref $boundary)) (field eqref)
  (field (mut (ref null $flist))) (field (mut (ref null $flist)))
  (field (mut (ref null $flist))) (field (mut (ref null $flist)))))
;; depth of the shadow stack where it runs, its pending records, the trampoline outside it
(type $tramp (struct (field i32) (field (mut (ref null $flist))) (field (ref null $tramp))))
(type $box (struct (field i64)))
```

`EFFECTS` gets `(global $pend (mut (ref null $tramp)) (ref.null $tramp))`: the live trampolines, innermost first.

### 4.2 Types and globals in the runtime

- `runtime/mod.rs` `TYPES`: `(type $unpend (func))`.
- `runtime/stops.rs`: `(global $stop_unpend (mut (ref null $unpend)) (ref.null $unpend))`.

Nothing else of control goes into the runtime, so the runtime-only module of `tests/runtime_values.rs` still validates.

### 4.3 The lists

- A list is null-terminated and innermost first through field 1.
- The kind-1 records of a list form a second chain through field 2, also innermost first. `$cap` and `$sub` keep its first and last node.
- Invariant N: the last node of a list has field 1 null, and the last kind-1 node has field 2 null.
- Invariant D: a kind-3 record is only ever the first record of its list (proof in section 6).

## 5. Exact rules

### 5.1 Records made by the compiler

`record(p)` builds:

```wat
(struct.new $frame (ref.func $r{p}) VALUES (local.get $slot) (i32.const KIND) DELTA)
```

- `VALUES`: the saved locals, then one value per `EnvLink` of `point.envs`, as today, but without the slot.
- The slot is `(local.get $slot)`, read by `suspendable` before the call, as today. Every function with a point has a frame (invariant S0, section 6). `point()` and `record()` assert `f.frame`.
- `KIND` is 1 when `point.delta()` is not empty, otherwise 0.
- `DELTA` is `(ref.null $values)` for kind 0. For kind 1 it is `(array.new_fixed $values 2n (ref.i31 (i32.const s1)) (array.get $env (local.get $l{d1}) (i32.const s1)) …)`, with one pair per link of `point.delta()`, outermost first.

`Point::delta(&self) -> Option<Vec<EnvLink>>` (new, in `ir.rs`) follows `point.env` back to `Env::Param` through `point.envs`:

- `Env::Param` gives an empty list.
- `Env::Local(l)` finds the link whose `dst` is `l`, and goes on from its `from`.
- If a link is missing, it gives `None`. It never panics. `control.rs` asserts `delta().is_some()` where it builds each point (the place that makes the invariant), so `record()` can rely on it.
- The match on `Env` has no `_` arm, so a new variant of `Env` must be handled here before the code builds.

`$r{p}` becomes "rebuild this one frame and go on":

```wat
(func $r{p} (type $refn) (param $frame (ref $frame)) (param $env (ref null $env))
    (param $v eqref) (param $mode i32) (result eqref)
  (local $saved (ref null $values)) (local $l… …)
  (local.set $saved (struct.get $frame 1 (local.get $frame)))
  ;; unbox the saved locals, as today
  ;; rebuild each EnvLink with $env_with from env_text(link.from), as today
  (call $shadow_enter (struct.get $frame 2 (local.get $frame)))
  (return_call $s{p} (local.get $env) ARGS (local.get $v)))
```

It has no `try_table`, no `call_ref` and no call to the records inside it. The trampoline calls it only with mode 0.

### 5.2 Runtime records (rule: "do what the eager handler did after its inner call ended this way")

| Record | Kind | Values | Mode 0 (value `v`) | Mode 1 (error `v`) | Mode 2 (cancel `v`) |
|---|---|---|---|---|---|
| `$catch_reenter` | 2 | none | `result_ok(v)` | `result_error(v)` | `throw $cancel v` |
| `$ensure_reenter` | 2 | `[with]` | `return_call $ensure_final(with, env, 0, v)` | `… (with, env, 1, v)` | `… (with, env, 2, v)` |
| `$final_reenter` | 2 | `[r, m]` | `return_call $ensure_end(m, r)` | `throw $error v` | `throw $cancel v` |
| `$boundary_reenter` | 3 | `[b, sub]` | set `b` active, `return_call $boundary_core(b, env, null, sub, v, mode)` | same | same |
| `$cancel_reenter` | 3 | `[k, sub]` | `return_call $cancel_core(k, sub, env, v, mode)` | same | same |

Each row is what the eager function did at `dcb19df`:

- `$catch_run` took `$error` and `$capture` only, so a cancel went through.
- `$ensure_run` called `$ensure_final` with mode 0, 1 or 2.
- `$final_reenter` took only `$capture`.
- `$boundary_reenter` set the flag and ran `$boundary_run` on the records inside it.
- `$cancel_reenter` ran `$cancel_run` on the records inside it.

The names in `DECLARED` (emit/mod.rs) stay. `$reenter_next` is deleted.

`$catch_run(env, body)` and `$ensure_run(with, env, body)` keep only the path that calls `body`, and lose `rest`, `v`, `cancel` and `frame`. Their capture paths build a new record each time: `(struct.new $frame (ref.func $catch_reenter) (ref.null $values) (i32.const -1) (i32.const 2) (ref.null $values))`, and the same for ensure with `[with]`. `$ensure_final` builds `$final_reenter` with `[r, mode]`, slot -1 and kind 2. `$ensure_end` does not change.

### 5.3 Capture

`$control_suspend` throws `(struct.new $cap b f null null null null)`.

`$capture_rethrow(c, x, fr)` keeps its signature, so the call sites in `suspendable` do not change:

```text
n = struct.new $flist (fr, null, null, null, 0)
if c.last == null then c.first = n else c.last.next = n
c.last = n
if fr.kind == 1:
  if c.mlast == null then c.mfirst = n else c.mlast.mnext = n
  c.mlast = n
throw_ref x
```

This is one allocation per frame, as today.

### 5.4 Packing at a delimiter

`$boundary_core` takes a capture for another boundary, and `$cancel_core` takes any capture. Each one:

```text
sub = (c.first == null) ? null : struct.new $sub (c.first, c.last, c.mfirst, c.mlast)
fr  = struct.new $frame (ref.func $boundary_reenter) [b, sub] -1 3 null   ;; or $cancel_reenter [k, sub]
n   = struct.new $flist (fr, null, null, null, 0)
c.first = c.last = n ; c.mfirst = c.mlast = null
throw_ref x
```

### 5.5 Splice

`splice(c, P, pm, plast, pmlast)` joins the pending records `P … plast` (kind-1 chain `pm … pmlast`) after the records in `c`:

```text
if P != null:
  if c.last == null then c.first = P else c.last.next = P
  c.last = plast
if pm != null:
  if c.mlast == null then c.mfirst = pm else c.mlast.mnext = pm
  c.mlast = pmlast
```

### 5.6 The trampoline

`$tramp_run(s (ref null $sub), env, v, mode) -> eqref`.

Setup (walk W). No i code runs here.

```text
if s == null or s.first == null: return end(v, mode)
;; Work out the environment of each kind-1 record, outermost first (§9.4 rule 3).
reverse the chain s.mfirst … s.mlast in place (field 2)
e = env
for m in that chain, outermost first:
  m.given = e
  e = $env_apply(e, m.frame.delta)            ;; copy once, set each pair in order
reverse the chain back
cur = e                                        ;; the environment of the innermost record
node = s.first ; nm = s.mfirst ; last = s.last ; mlast = s.mlast
d = $shadow_depth
t = struct.new $tramp (d, node, $pend) ; $pend = t
$stop_unpend = ref.func $control_unpend
first = node
```

`$env_apply(env, null)` is `env`. Otherwise it copies `env` once and does `array.set` for each pair, in order, so a later pair wins, as a chain of `$env_with` does.

Loop:

```text
loop:
  if node == null:
    $pend = t.outer
    return end(v, mode)              ;; mode 0: v; 1: throw $error v; 2: throw $cancel v
  this = node ; fr = this.frame
  check this.state == 0, then this.state = 1      ;; D11
  check fr.kind != 3 or this == first              ;; invariant D
  node = this.next ; t.pending = node
  if this == nm:                     ;; a kind-1 record: it is given its own environment
    given = this.given ; nm = this.mnext ; cur = given
  else:
    given = cur
  if mode != 0 and fr.kind <= 1: continue         ;; a code frame has no handler
  try_table (catch $error E) (catch $cancel X) (catch_ref $capture C):
    v = call_ref $refn (fr, given, v, mode)
  mode = 0 ; continue
E: $shadow_restore(d) ; v = payload ; mode = 1 ; continue
X: $shadow_restore(d) ; v = payload ; mode = 2 ; continue
C (c, x):
  splice(c, node, nm, last, mlast)
  $pend = t.outer
  throw_ref x
```

The trampoline catches only `$error`, `$cancel` and `$capture`. It has no `catch_all`, so a stop, which traps, runs no handler (§9.5).

### 5.7 Entry points

- `$boundary_core(b, env, body, sub (ref null $sub), v, mode)` is today's `$boundary_run`. The only change is the call inside the `try_table`: `body` null gives `call $tramp_run(sub, env, v, mode)`, and otherwise `call $call1(body, b, env)`.
  - Its own capture builds `$cont(…, used 0, b, sub)` with `sub` made from `c` as in 5.4.
  - A capture for another boundary is packed (5.4).
  - The rules for the active flag and the shadow depth are those of today.
- `$boundary_run(b, env, body, rest, v, cancel, frame)` is a shim. It traps if `rest`, `cancel` or `frame` is not null, and does `return_call $boundary_core(b, env, body, null, v, 0)`. Its doc says: "Kept with these parameters for `emit/host.rs`. `rest`, `cancel` and `frame` must be null."
- `$control_boundary` calls `$boundary_core` directly.
- `$control_resume(k, v, env)` (same signature): take `k`, set its boundary active, `return_call $boundary_core(b, env, null, k.sub, v, 0)`.
- `$control_cancel(k, env)`: take `k`, `return_call $cancel_core(k, k.sub, env, k, 2)`.
- `$cancel_core(k, sub, env, v, mode)` is today's `$cancel_run` with `$reenter_next` replaced by `$tramp_run(sub, env, v, mode)`. It takes `$cancel e` only when `e` is `k`, and then restores the depth and gives null. It packs a capture (5.4).
- `$control_reset` sets `$pend` to null. Code that resets `$sp` for a new entry inside the same instance must also call it (section 5.9).

### 5.8 Stops (§11.7)

`$stop` (runtime/stops.rs) gets one change. Right after it sets `$stopping`:

```wat
(if (i32.eqz (ref.is_null (global.get $stop_unpend)))
  (then (call_ref $unpend (ref.as_non_null (global.get $stop_unpend)))))
```

`$control_unpend` builds the shadow stack that eager re-entry would have had:

```text
if $pend == null: return
sp = $sp ; n = 0
for t in $pend: if t.depth <= sp: n += number of code records (kind 0 or 1) in t.pending
new = array.new_default $frames (sp + n)
hi = sp + n ; top = sp
for t in $pend (innermost first), skipping any t with t.depth > sp:
  copy $shadow[t.depth .. top) to new[hi - (top - t.depth) .. hi) ; hi -= top - t.depth ; top = t.depth
  for each code record r in t.pending (innermost first): hi -= 1 ; new[hi] = r.frame.slot
copy $shadow[0 .. top) to new[0 .. top)
$shadow = new ; $sp = sp + n
```

- Runtime records push no slot in eager re-entry either, so they are left out.
- A trampoline whose depth is above `$sp` cannot be live (invariant P3). It could only be left over from an earlier trap in an instance that is used again, which a host may not do (section 5.9). The skip is kept as a cheap defence, for the test harness.
- Trampolines at the same depth (from descent, D4) are handled in order: the inner one's records land above the outer one's, as in eager.
- The walker then prints the array as today, with the same tail-site rule.

### 5.9 Resetting after a trap

- `$pend` is balanced on every exit of the trampoline that is not a trap (invariant P1).
- After a trap, `$pend` is stale, and so are `$sp` and `$shadow` today.
- The spec already rules out using such an instance again. Appendix B.5: "After a trap, the host may not use that instance of the module again; it makes a new instance and calls `ilang_start` on it." §10.2 runs each test in a new instance. So a conforming host never sees a stale `$pend`, and `emit/host.rs` and lane C need no change for it.
- At `dcb19df` the only code that resets `$sp` inside one instance is the test harness in `runtime/tests.rs`. Its modules have no control text, so they have no `$pend` and need no call.
- The rule written into the protocol doc of `runtime/mod.rs` is: "Code that resets `$sp` for a new entry also calls `$control_reset`." It is for internal harnesses, and for the compiler's runner of macro programs if it reuses an instance in a way "a macro cannot tell from a new instance" (meta.md §8). Today no such place exists, so no call is added.
- `$control_unpend` still skips trampolines above `$sp`, and `$stopping` makes a second stop trap at once, as today.

## 6. Invariants

- **O1 (one owner).** Every `$flist` node is pending in at most one live owner. An owner is a `$cap` being thrown, the `$sub` of a `$cont` that is not used yet, the `$sub` in a delimiter record that has not run, or the pending part of a running trampoline. Nodes move only by these steps, and each step removes them from the old owner:
  - take: `$continuation_take` marks the continuation used before `$tramp_run` gets its records;
  - consume: the trampoline sets the state of the node and moves `node` past it;
  - splice: the trampoline hands its pending part to the capture, then exits with `throw_ref`;
  - pack: the capture's records go into the new delimiter record, which is the capture's only record;
  - own capture: the boundary makes the `$cont` from the capture, which is then dropped.

  Every write goes to a node of the writer's own owner:
  - `next` and `mnext` of the capture's last nodes, in append and splice;
  - `given` and the in-place reversal in walk W, while the trampoline owns all its nodes;
  - `state`.

  So no record runs twice and none is lost. The state check (D11) traps if a node is taken twice.
- **D (delimiter first).** A kind-3 record is made only by packing (5.4), where it is the only record of its list. After that, records are added only at the tail: by append, and by splice of a trampoline's pending part. A pending part has no kind-3 record, because a trampoline's only kind-3 record is its first one, and that one is taken before any call. The trampoline checks this.
- **N (ends).** Append makes a node with null `next` and `mnext`. Splice makes the capture's `last` the trampoline's `last`, which ends its list. So the ends stay null.
- **P1 (`$pend` balanced).** The trampoline pushes itself after walk W, which cannot throw, and pops itself at the end of the list (before it returns or throws) and before a splice's `throw_ref`. Inside the loop it takes all three tags of the module, so no exception leaves it another way. A trap runs no code.
- **P2 (order).** A trampoline that starts while another is live runs above one of that one's calls, so its depth is at least as large. `$pend` is innermost first, so it is sorted by depth, highest first.
- **P3 (depth).** Whenever the trampoline calls a record, `$sp` is `d`:
  - A code record pushes its slot, and its segment pops it on return (`$shadow_leave`, or `$shadow_leave_tail` before a tail call whose callee pops its own).
  - Runtime records and delimiters push nothing.
  - After an error or a cancel, the trampoline restores `d`.
- **S0 (frames).** Every function that has a point has a frame. At `dcb19df`, `lower.rs` and `control.rs` set `frame: true` for every code function. `point()` and `record()` assert it. A frameless segment could `$shadow_site` into a slot below the trampoline and break the equal-trace argument.

## 7. Why the behaviour does not change

We compare a lazy run with the eager run of the same program at `dcb19df`. A lazy state is: a trampoline over records 1..n, the next record is j+1, and code X is running. It matches the eager state where Wasm holds the handlers of records n..j+1 with X on top, when all of these hold:

- (a) The eager shadow stack is `base ++ slots(n..j+1) ++ S_X`. The lazy one is `base ++ S_X`, with a `$tramp` at depth `|base|` whose pending part is j+1..n. `$control_unpend` turns the lazy array into the eager one, so `$stop` prints the same text with the same tail-site rule. `$tail` is the same in both (section 8).
- (b) Each record is given the same environment in both (lemma Env).
- (c) Each boundary has the same active flag in both (lemma Active).
- (d) The pending records are the records the eager handlers j+1..n would add again on a capture. They are the same `$frame` contents in the same order, innermost first.

**Lemma Env.** In eager, record i is given `given(i) = F_{i+1}(given(i+1))`, and the outermost one is given the environment of `resume` or `cancel`. `F_i` is the identity unless record i has kind 1. For kind 1, `F_i` applies the environment statements on the chain from the function's environment to `point.env`, which is `env_text(point.env)` built by `$env_with` from `env_text(link.from)`. `$env_apply` with `point.delta()` gives the same array contents. Arrays of the environment have no identity that i code can see.

Walk W visits the kind-1 records outermost first and stores `given` on each. For any other record, `given` is `cur`, which is reset at each kind-1 record. This holds when the trampoline skips records in mode 1 or 2 too, because it updates `nm` and `cur` before the skip. Inside a delimiter, the base is the environment the delimiter is given, as in eager `$boundary_reenter`. So §9.4 rule 3 holds for resume and cancel. The spec cases `control.i` (`environment`, `deep`) and `control_cancel_final` check this.

**Lemma Active.** Eager sets the flag of each inner boundary to 1 when it re-enters it, at resume or cancel time, before any inner code runs. Descent does the same, by the same code. Each boundary is switched off by the same code as today (`$boundary_core` is `$boundary_run`), on a normal end, an error, a cancel and a capture.

Each path:

- **(V) A value from X.** In eager, handler j+1 gets it. In lazy, record j+1 is called in mode 0, and table 5.2 is that handler's normal path. For a code record: eager `$r` gets the value from `$reenter_next` and does `return_call $s{p}` with it; lazy `$r` does the same after pushing its slot. Pushing the slot later is not visible to i code: only a stop reads the shadow stack, and (a) covers stops. An empty list gives `v`, as `$reenter_next(null)` did.
- **(E) An error or a cancel from X.** In eager, the code handlers have no `try_table` for `$error` or `$cancel`, so the exception goes through them. The first runtime handler takes it, after it restores its depth (`base ++ slots(n..i+1)`). In lazy, the trampoline restores `d` (equal under (a), because records i+1..n are still pending), skips kinds 0 and 1, and calls the first kind-2 or kind-3 record with mode 1 or 2. Table 5.2 is what that handler did:
  - `fail.catch` gives `result_error`;
  - ensure runs `with` with `given`, then `$ensure_end` throws again, so the final functions run innermost first (`control_cancel_inner_first`, `ensure_inner_first`);
  - a final record lets it through;
  - a delimiter runs `$boundary_core` or `$cancel_core`, which switch off or take, as before.

  If no record takes it, the trampoline pops and throws it, as the bottom of the eager stack did.
- **(K) `cancel`.** It starts the trampoline in mode 2 with nothing running. Eager re-entered the outer frames first, but they had no effects apart from shadow pushes and handlers that a cancel passes. The pushes are undone by whatever takes the cancel.
- **(C) A capture from X.** In eager, each handler it goes through adds its record, a boundary is switched off and adds `$boundary_reenter`, and the boundary of the capture restores its depth and does `f(k)` in its place. In lazy:
  - The pending code and handler records of the trampoline are spliced in their order. By (d) that is the order eager would add them.
  - A delimiter that the capture goes through is a real `$boundary_core` or `$cancel_core` call, which packs. The packed record holds the same records that eager's `$boundary_reenter` would hold in its `rest` when re-entered, and gives them to the same boundary code.
  - The capture's own boundary is a real `$boundary_core` call, as in eager. So `f(k)`, its depth and its environment are those of today.
- **(R) Resume, suspend, resume.** By O1, the second continuation owns the new records followed by the old pending ones. A second resume runs them as a first resume would. The spec case `control_lazy_resuspend` checks every frame once.
- **(S) A final function that suspends during a cancel.** `$ensure_final` adds `$final_reenter`. The trampoline splices, and `$cancel_core` packs with `$cancel_reenter`. A later resume re-enters the cancel by descent. `$final_reenter` in mode 0 calls `$ensure_end(2, k)`, which throws `$cancel k`. The trampoline passes it outward to the end of the inner list, and `$cancel_core` takes it because it is `k`. This is `cancel_runs_final_functions` in `control.i`.
- **(H) The host.** See section 9.
- **(M) Running out of stack.** This is the one stop whose place may move. A deep resume now uses less Wasm stack: eager made N frames and N `try_table`s at once, lazy makes one record frame at a time. But each level of nested resume holds one more frame than before (`$tramp_run`, with its `try_table` and about 10 locals): `$boundary_core` + `$tramp_run` + the record frame, where eager held `$boundary_run` + the `$r` chain. So a program that recursively resumes inside resumes (an in-order tree walk where each level resumes inside the previous resume) and is just under the stack limit today may stop with "There is no more memory or stack" after B, and a program that stopped there today may now finish. The spec gives no fixed stack size (§11.7), so both are allowed. Test 10 of 13.2 pins how much this moves.

## 8. Tail calls

- The trampoline calls a record with a plain call, not a tail call. There is one trampoline frame per resume, and none per pending record.
- Inside a record, segments chain with `return_call` as today. A segment that ends in a tail call returns its callee's value to the trampoline. So a loop of 1,000,000 tail calls in a resumed computation keeps O(1) Wasm stack (`control.i` `tail_calls`).
- The trampoline and walk W never read or write `$tail`. Eager re-entry used `$shadow_enter`, not `$shadow_enter_call`, so it did not touch `$tail` either. The segments run in the same order with the same calls between them, so every read of `$tail`, and every stop, sees the same value. This keeps the 11:28 rule of `stop_tail_resume`.
- Slots are stored as read, tail sites included (`-2 - site`).

## 8a. The seam with M2: call depth

M2 adds limits to `emit/control.rs` after B (plan merge order B → M2), counted "in units of the language, independent of how the compiler optimizes" (meta.md §4.4, §11 question 3), and one of them is how deep calls may go. B adds no limit, but it changes what `$sp` means:

- Eager re-entry rebuilt every paused frame at resume, so `$sp` was the logical depth.
- Lazy re-entry leaves the paused frames in `$pend`. If a computation is paused under 900 frames and resumed from a place 200 frames deep, and then goes 50 frames deeper, eager had `$sp` = 1150. Lazy has `$sp` about 251, and 900 frames wait in `$pend`.

**Rule.** The call depth of a running program is `$sp` plus the number of pending code records (kind 0 or 1) in the live trampolines of `$pend` whose depth is at most `$sp`. This is the size of the array `$control_unpend` builds, and it equals the eager `$sp`. Runtime records and delimiters do not count, as they pushed no slot in eager either. A depth limit built on `$sp` alone would let a program go deeper after B than before, and so would change behaviour.

M2 can compute this in O(1) per check if it needs to: `$cap` and `$sub` keep a count of their code records (append adds 1 for kind 0 or 1, splice adds the trampoline's remaining count, pack moves the count into the delimiter's `$sub`), each `$tramp` keeps its remaining count, and a global keeps the sum over `$pend`. B does not add these fields, because nothing reads them yet. M2 adds them with the limit, and its review checks the rule above.

## 9. The seam with `emit/host.rs`

`emit/host.rs` does not change:

- `$host_boundary` calls `$boundary_run` with today's 7 arguments, through the shim (D7).
- `$host_resume_k` calls `$control_resume(k, null, $root_env)`, whose signature is the same.
- A re-suspend to the root boundary during a lazy resume goes through the trampoline, which splices and throws it on. The root boundary is the resumed boundary, so it is a real `$boundary_core` call below the trampoline. Its own-capture path calls `f(k)`, which is `$host_wait`. `$host_waiting` comes back to `$async_run` as before. `$host_k` then holds the new frames followed by the pending records.
- An error from `HostReply` after a resume goes to the pending `fail.catch` or `resource.ensure` by path (E). `$pend` is already popped when it reaches `$async_run` (P1).
- Stream C edits host.rs at the same time. It must keep passing null `rest`, `cancel` and `frame`. The shim traps if not, and a test pins it (section 12).

`tests/browser/fixtures/compiled.wasm` is compiler output with the control runtime in it, so it changes. It belongs to lane C (`scripts/build-abi-fixture.sh`). The branch of B and C that lands second rebuilds it in its rebase.

## 10. Cost

Terms, for one continuation:

- F is the number of frames entered since the resume.
- D is the number of boundaries and paused cancels it is paused inside (the delimiters).
- E is the number of its frames whose call is under an environment statement of their function.

Costs:

- `resume`: O(1 + D + E). Walk W uses no allocation for the reversal; it makes one array copy per kind-1 record.
- `cancel`: O(1 + D + E + P), where P is the number of pending records. The trampoline runs in mode 2 from the start, and the loop visits every pending record: it skips code records with no call, but it still takes each one (state check, `continue`). Only after the last record does it throw `$cancel k` to `$cancel_core`. Eager cancel re-entered all N frames first, so this is the same order as today (D8); B does not make cancel cheaper. The R2 task scheduler cancels tasks with `control.cancel`, so later tracks should not count on O(1) cancel.
- A suspend after a resume: O(1 + F + D). The splice is O(1), and each delimiter it goes through packs in O(1).
- `fail.catch` and `resource.ensure` records cost nothing at resume or at a suspend. They cost one call each when an error or a cancel goes through them.
- An error or a cancel that goes through pending records: O(records it goes through), in a loop with no call for code records. Today it is the same order, as a Wasm unwind.
- The first capture of a frame: O(1), as today.
- A stop: O(depth), once.

A program pays more than O(1) per resume only when it has frames with environment statements or nested boundaries in the paused part. A cancel always pays for each pending record.

**Threshold.** The pass rule of the plan ("恢复耗时不随栈深增长") is:

- `bench/test/control_test.i`, `--opt 2`, run by the coordinator in a quiet window, the median of 5 runs. The frames have no environment statements, `fail.catch`, `resource.ensure` or inner boundaries (D = E = 0).
- t(depth 1000) / t(depth 10) ≤ 1.5, and t(depth 100) / t(depth 10) ≤ 1.5.
- The coordinator measures the eager baseline at the bench commit first (section 13). We expect a ratio of 20 or more, but this is not measured yet.

CI checks a stricter, exact number instead (D9): with fuel, the extra fuel of one more round at depth 1000 divided by the same at depth 10 is at most 1.05. Lazy re-entry runs the same instructions per round at every depth, so the ratio should be 1.00. The margin is for the bisection.

## 11. Text changes

### 11.1 `docs/spec.md`, section by section

| Section | Change |
|---|---|
| §9.4 rules 1 to 5 | none (lemmas Env and Active, paths V, E, C, R) |
| §9.5 | none: final functions run innermost first; a stop runs no handler (no `catch_all`) |
| §9.7, Appendix B.4 to B.6 | none (section 9) |
| §11.6 | none (section 8) |
| §11.7 | none: traces are equal by (a) |

### 11.2 Doc text in the code

Doc comment of `EFFECTS` in `emit/control.rs`, the bullets of "Control (§9.4)" become:

> - A boundary is a `$boundary` whose field 8 is 1 while it is active.
> - `suspend` throws `$capture` with a `$cap`: the boundary, `f`, and the frame records so far. Each frame that the exception goes through adds its record at the end (`$capture_rethrow`), so the list is innermost first.
> - A frame record is a `$frame`: its re-entry function, the values it needs, its slot on the shadow stack, and its kind. A frame whose call is under an environment statement also keeps the statements' services and values.
> - `resume` and `cancel` re-enter only the innermost record. `$tramp_run` keeps the others, and gives each one what the frames inside it ended with: a value, an error or a cancel. A record then does what its frame would have done at that point. Code frames have nothing to do with an error or a cancel, so they are passed over.
> - A capture that leaves `$tramp_run` takes the records that are still waiting with it, in one step.
> - A boundary or a cancel that a capture goes through keeps the records inside it in one record of its own. `resume` re-enters such records at once, so their boundaries are active again.
> - `$boundary_run` is kept for `host.rs` only; `$boundary_core` does the work.

`ir.rs` module doc, the sentence about re-entry becomes:

> …and a **re-entry** function that rebuilds the frame when the frames inside it have returned: it rebuilds the environment, puts the frame on the shadow stack with the slot that the frame record keeps, and goes on with the segment.

`ir.rs`, the doc of `Point::env`:

> The environment of the call. The environment statements between the function's environment and this one (`Point::delta`) are kept in the frame record, so that the frames inside it get them when they are re-entered (§9.4 rule 3).

`control.rs` module doc: "Frames are captured lazily" becomes "Frames are captured lazily, and re-entered lazily: a resume rebuilds only the innermost frame."

`runtime/mod.rs`, the last bullet of the shadow stack protocol becomes:

> - When a continuation is resumed or cancelled, a frame is rebuilt only when the frames inside it have returned, and it enters again with the site of the call it was paused at. The frames not rebuilt yet wait in the trampolines of `$pend` (`emit/control.rs`); a stop puts their sites back into the shadow stack through `$stop_unpend` before it prints. Code that resets `$sp` for a new entry also calls `$control_reset`.

### 11.3 `docs/performance.md`, a new section "Resuming a continuation"

> A paused computation keeps one record for each of its frames, innermost first. `control.resume` rebuilds only the innermost frame. The others are rebuilt one at a time, when the frames inside them return. When the resumed code suspends again, only the frames it entered since the resume are captured, and the records still waiting are joined on in one step. So a resume and the next suspend cost the same at any depth of the paused stack.
>
> Some frames still cost something at each resume:
>
> - a frame whose call is under an environment statement (`! S = v`) of its function: its environment is worked out again;
> - a boundary that the computation is paused inside, and a cancel whose final function suspended: they are re-entered at once, and a later suspend through them packs the frames inside them.
>
> `fail.catch` and `resource.ensure` cost nothing at a resume. An error or a cancel goes through the waiting frames one at a time, as an unwind does. So `control.cancel` costs time in proportion to the number of waiting frames, as it did before.
>
> `bench/test/control_test.i` resumes at depths 10, 100 and 1000 (`scripts/bench control`). The pass rule is that, at `--opt 2`, the median of 5 runs at depth 1000, and at depth 100, is at most 1.5 times the one at depth 10. CI checks with fuel that one more round costs the same at depth 1000 and at depth 10 (`crates/ilang-wasm/tests/control_lazy.rs`).

The table of benchmarks gets three rows: `resume_depth_10`, `resume_depth_100` and `resume_depth_1000`, "100k resumes of a computation paused under 10, 100 or 1000 frames". The results section gets the eager and lazy numbers once they are measured.

## 12. Files to change

| File | Region | Change |
|---|---|---|
| `crates/ilang-wasm/src/emit/control.rs` | `TYPES` | section 4.1 |
| | `EFFECTS` and its doc | `$pend`, `$capture_rethrow`, `$env_apply`, `$tramp_run`, `$control_unpend`, `$control_reset`; the five `*_reenter` (5.2); `$catch_run` and `$ensure_run` lose the re-entry path; `$ensure_final` record; `$boundary_core` and the `$boundary_run` shim; `$control_boundary`, `$control_resume`, `$control_cancel`, `$cancel_core`; delete `$reenter_next` |
| | `record()` | slot field, kind, delta; assert `f.frame` |
| | `point()` | the short `$r{p}` of 5.1; assert `f.frame` |
| | `suspendable()`, `continue_point()`, `join_call()`, `rest_func()` | no change |
| `crates/ilang-wasm/src/emit/mod.rs` | `DECLARED` | add `"$control_unpend"` (for `ref.func`) |
| `crates/ilang-wasm/src/ir.rs` | module doc lines 51 to 56, `Point` doc, `impl Point` | text of 11.2; `Point::delta` |
| `crates/ilang-wasm/src/control.rs` | module doc; where points are built (about line 117); tests | text; `assert!(point.delta().is_some())`; unit tests of `delta` |
| `crates/ilang-wasm/src/runtime/mod.rs` | `TYPES`; protocol doc | `(type $unpend (func))`; text of 11.2 |
| `crates/ilang-wasm/src/runtime/stops.rs` | globals; `$stop` | `$stop_unpend`; the call of 5.8 |
| `docs/performance.md` | new section, table rows | 11.3 |
| `tests/browser/fixtures/compiled.wasm` | whole file | rebuilt with `scripts/build-abi-fixture.sh` (lane C's file, section 9) |
| new `bench/test/control_test.i` | | section 13 |
| new `crates/ilang-wasm/tests/control_lazy.rs` | | section 13 |
| new spec cases in `tests/spec/9`, `tests/spec/9.7`, `tests/spec/11` | | section 13 |

`emit/host.rs`, `lower.rs` and `Cargo.toml` do not change.

Outside B's list in the plan: `runtime/stops.rs`, `runtime/mod.rs`, `emit/mod.rs`, the new spec cases, and `compiled.wasm`. They need the coordinator's consent (open question 1). The stops.rs change is one global and three lines. The runtime/mod.rs change is one type and one doc bullet.

## 13. Tests

### 13.1 `bench/test/control_test.i`

This uses the new syntax, so it is committed after A1+A2 land.

```i
//// Resuming a paused computation at depth 10, 100 and 1000. `scripts/bench`
//// runs these with `i test --time` at `--opt 2`. A resume and the next
//// suspend should take the same time at every depth (docs/performance.md).

(
  i/control
  i/core
)

rounds = 100_000

// What the paused computation gives: each number, and at the end the
// number of frames it went back through.
Step { Done(Int)  More(Int, Continuation(Nil, Step, Never, Never)) }

check = (ok: Bool, what: String) -> Nil {
  ok |> {
    True -> Nil
    False -> panic(what)
  }
}

// Suspends once for each number from `n` down to 1. The call after the
// suspend is a tail call, so this is one frame.
yields = (b, n) {
  n |> {
    0 -> Done(0)
    _ -> {
      control.suspend(b, (k) { More(n, k) })
      yields(b, n - 1)
    }
  }
}

// Counts one more frame on the way back.
deeper = {
  Done(d) -> Done(d + 1)
  more -> more
}

// `depth` frames that wait under the suspends.
down = (b, depth) {
  depth |> {
    0 -> yields(b, rounds)
    _ -> deeper(down(b, depth - 1))
  }
}

// Resumes until the end. It runs outside the boundary and calls itself as
// a tail call, so the place of resume is always at the same depth.
sum = (step, acc, depth) {
  step |> {
    Done(d) -> {
      check(d == depth, "a frame was lost or ran twice")
      acc
    }
    More(v, k) -> sum(control.resume(k, Nil), acc + v, depth)
  }
}

run = (depth) {
  got = sum(control.boundary((b) { down(b, depth) }), 0, depth)
  check(got == 5_000_050_000, "the sum of 1 to rounds is wrong")
}

@test
resume_depth_10 = () { run(10) }

@test
resume_depth_100 = () { run(100) }

@test
resume_depth_1000 = () { run(1000) }
```

Before it is committed, it must check under A1+A2: the imports group, the `|>` clause blocks, `@test` on bindings, a top-level clause block (`deeper`), and `b`, `k` and `step` with no type. `tests/spec/9/control.i` (line 205) has a similar `Step`, `Step { Done  More(Int, Continuation(Nil, Step, Never, Never)) }`. Only its `More` matches: its `Done` has no field, and the bench's `Done` has an `Int`. So the bench's type is checked on its own, not taken from that file. `Continuation(Nil, Step, Never, Never)` is written the same in the new syntax: syntax.md renames only type variables, and `Nil`, `Step` and `Never` are types (syntax.md §3: an `UpName` is a type variable only when a type variable with that name is declared where the type is written). If `deeper` were inlined, the frames would still stay, because it does work after `down` returns.

### 13.2 `crates/ilang-wasm/tests/control_lazy.rs`

It uses `ilang_wasm::compile_wat` and `compile` as `tests/codegen.rs` does, `wasmparser` to validate, and the `wasmtime` CLI as `tests/runtime_values.rs` does (`WASMTIME`, default `wasmtime`). It needs no new dependency.

Shape tests (no engine):

1. `reentry_rebuilds_one_frame`: no `$r…` function contains `try_table`, `call_ref` or `$reenter_next`; each ends in `return_call $s…`; `$reenter_next` is gone; `$control_resume` reaches `$boundary_core`, and `$boundary_core` reaches `$tramp_run`.
2. `record_kinds`: a plain point has kind 0 and a null delta; a point under `! S = v` has kind 1 and a delta of `(ref.i31 (i32.const S)) …`; two nested statements give two pairs, outermost first; the slot is `(local.get $slot)` in field 2. The runtime records have kinds 2 (catch, ensure, final) and 3 (boundary, cancel).
3. `host_seam_kept`: the texts of `$host_boundary` and `$host_resume_k` equal a copy taken at `dcb19df`. If lane C changes them, C updates the copy, and the scope review checks the seam. Every call of `$boundary_run` passes `(ref.null $flist)`, `(ref.null eq)` for `cancel` and `(ref.null $frame)`. `$boundary_run` holds the null checks and one `return_call $boundary_core`.
4. `runtime_alone_validates`: `runtime::wat()` with `runtime::TYPES` and no control text still validates (the hook has a null default).
5. `modules_validate`: programs that use every control operation, `fail.catch`, `resource.ensure` and environment statements validate at `--opt 0`. `--opt 2` is covered by the spec suite.

Run tests (wasmtime CLI). The test writes a small module of stub imports and a driver module that calls the program's start and main exports, and links them with `--preload`. It reads the imports of the compiled module with `wasmparser` and fails with a clear message if one has no stub. The export names are those of the ABI at the bench commit; lane C may change them in ABI v3, so the test takes them from the compiler's metadata where it can.

6. `resume_cost_is_flat` (D9): the bench program as `@main`, with `rounds` and the expected sum filled in. For depth 10 and depth 1000, and rounds R1 = 1000 and R2 = 4000:
   - bisect the smallest fuel (`-W fuel=N`) that finishes, until the bracket is under 0.1%;
   - the marginal fuel is (F(R2) − F(R1)) / (R2 − R1);
   - assert marginal(1000) / marginal(10) ≤ 1.05.

   The program is compiled once to a `.cwasm`. The same check for depth 100 is also run.
7. `deep_resume`: depth 100,000, 1000 rounds, with the Wasm stack size the spec runner uses. It must finish. Eager would do about 10^8 re-entries here. If the stack does not hold 100,000 frames at the first capture, the depth is 20,000.
8. `resume_twice_from_new_places`: kind-1 frames are pending; the computation is resumed from two places with different environments, and the second resume sees the second environment (walk W starts from the new base each time).
9. `counted_frames`: a resume where 3 inner frames return, so outer frames are re-entered; then a suspend, a resume, and at the end a cancel. Each frame counts itself, and every count is 1.
10. `nested_resume_stack`: a recursive generator composition at `--opt 0`: an in-order walk of a tree of depth L, where each level resumes the generator of the level below inside its own resume. The test runs it with a fixed `-W max-wasm-stack=S` and bisects the largest L that finishes. It asserts L_lazy ≥ 0.8 × L_eager, where L_eager is a constant measured on the eager compiler at the bench commit (section 14, step 2). A second case pauses at depth 50,000 and resumes from inside a 50,000-deep recursion, and asserts that it finishes with the same S (eager may stop here; lazy must not). This pins how far the "no more stack" point moves (section 7, path M).

### 13.3 New spec cases

These use the new syntax and are added after A1+A2. Each runs at O0 and O2.

- `tests/spec/9/control_lazy_error.i`: after a resume at depth 50, the innermost frame raises an error, and a pending `fail.catch` near the boundary catches it.
- `tests/spec/9/control_lazy_ensure.i`: ensures at 3 depths; an error after a resume, and a cancel, run the finals innermost first.
- `tests/spec/9/control_lazy_inner_boundary.i`: a suspend to an inner boundary that was paused inside the continuation, after earlier re-suspends spliced the list; then a resume of that `k`; then a suspend to the outer boundary.
- `tests/spec/9/control_lazy_inactive.i`: the `f(k)` of a re-entered inner boundary suspends to that same boundary, and the run stops with `control: boundary is not active` (regression).
- `tests/spec/9/control_lazy_resuspend.i`: resume, suspend, resume with a pending ensure; every frame runs once.
- `tests/spec/9/control_lazy_env.i`: environment statements in pending frames, nested two deep, and a service changed at the place of resume.
- `tests/spec/9/control_lazy_cancel.i`: a cancel at depth 100 with 3 ensures, one of whose final functions suspends and is resumed later.
- `tests/spec/9/control_lazy_tail.i`: a loop of 1,000,000 tail calls in a resumed computation with 1000 pending frames.
- `tests/spec/11/stop_lazy_pending.i`: a panic after a resume at depth 3. The `at:` list is innermost, the 3 pending frames, the place of resume, then main. It is exactly what eager prints at `dcb19df`.
- `tests/spec/11/stop_lazy_nested.i`: a stop inside a resume that runs inside a resumed computation, with pending frames in both trampolines (P2).
- `tests/spec/11/stop_lazy_descent.i`: a stop inside a re-entered inner boundary, with two trampolines at the same depth.
- `tests/spec/11/stop_lazy_tail.i`: a tail call in a resumed segment, then a stop.
- `tests/spec/9.7/host_lazy_twice.i`: two async imports after a resume while outer frames are pending, and a `fail.catch` around an import that errors.

The expected `at:` lists are taken by running each case on the eager compiler at the bench commit, before the implementation is rebased onto it.

### 13.4 Regression

All of `tests/spec/9` and `tests/spec/11` at O0 and O2 pass unchanged, in particular:

- `control.i`, `control_stop_after_resume`, `stop_tail_resume`, `control_cancel_final`, `control_cancel_inner_first`, `control_cancel_then_resume`, `control_inactive_paused`;
- `host_value_async_stop`, `stop_tail_async_service`;
- the `ilang_resume` tests of `ilang-cli` and `ilang-host`.

## 14. Order of work (plan B-I)

1. Before A1+A2 land: the Rust changes and the doc text on B's implementation branch, kept green on the old-syntax spec cases at O0 and O2. The shape tests of 13.2 that need i source (2 `record_kinds`, 5 `modules_validate`) are written on this branch in `crates/ilang-wasm/tests/control_lazy.rs`, with old-syntax sources in Rust strings. Tests 1, 3 and 4 need no i source and go in the same file. Nothing of this file is on `redesign` yet, so A2's migration does not see it, and B owns converting it.
2. After A1+A2 land: check that the bench forms type-check, then commit `bench/test/control_test.i` and `control_lazy.rs` on the tip of `redesign`, with every i source in the new syntax. On eager code these tests fail, so they are marked `#[ignore]` with the note "turned on by the lazy re-entry commit":
   - 1 `reentry_rebuilds_one_frame` and 2 `record_kinds`: the eager record has no kind, no slot field and no delta, and `$r` holds a `try_table`;
   - 3 `host_seam_kept`: at `dcb19df` `$control_resume` passes a non-null `rest` to `$boundary_run`, `$boundary_reenter` passes `rest` and `frame`, and `$boundary_core` does not exist;
   - 6 `resume_cost_is_flat`: the ratio grows with depth;
   - 7 `deep_resume`: about 10^8 eager re-entries and as many capture rethrows, far over a CI time budget.

   Tests 4, 5, 8, 9 and 10 pass on eager code and stay on. The coordinator measures the eager baseline at this commit; the expected `at:` lists of 13.3 and L_eager of test 10 are taken here.
3. Rebase the implementation onto that commit. In the conflict on `control_lazy.rs`, B keeps step 2's file (new syntax) and drops the old-syntax copy of step 1, then turns the ignored tests on. After this step no old-syntax i source of B is left in a Rust string, and B runs `$C/tools/old-syntax-check` on the branch before it lands.
4. Rebuild `tests/browser/fixtures/compiled.wasm` if B lands after C. If C lands after B, C rebuilds it.
5. B-RF: the coordinator measures again; `tests/spec/9` and `tests/spec/11` pass at O0 and O2; the scope review checks that `emit/host.rs` has no diff.

## 15. Risks

- **List mutation.** O1 and invariant D are new. A slip would run a frame twice or lose one. The mitigations are the state check (D11), the delimiter check, the run test that counts frames, and the re-suspend, inner-boundary and cancel spec cases at O0 and O2.
- **Files outside B's list.** `runtime/stops.rs`, `runtime/mod.rs` and `emit/mod.rs` change a little, and `compiled.wasm` is lane C's file.
- **Exact traces.** These rest on P2, P3 and S0. A future frameless function with a point would break them, which is why `point()` asserts S0.
- **State after a trap.** `$pend` is stale after a trap, as `$sp` is. The spec forbids reusing the instance (Appendix B.5), so only an internal harness could see it. The reset rule is written down for such harnesses (section 5.9).
- **The shim.** If stream C starts to pass a non-null `rest`, `cancel` or `frame` to `$boundary_run`, the shim traps. The trap is loud, but it is a run-time failure.
- **Delimiters cost O(D).** Code that resumes inside many nested boundaries keeps a linear cost. The threshold does not cover it, and performance.md says so.
- **Environment statements cost O(E).** A frame whose call is under its own environment statement costs two environment copies per resume: one in walk W, and one when `$r` rebuilds its locals.
- **Wasm stack per nested resume.** Each level of nested resumes now uses `$boundary_core`, `$tramp_run` and the record call. Eager used `$boundary_run`, `$reenter_next` and the `$r` chain. So the point of "There is no more memory or stack" may move (section 7, path M). Test 10 of 13.2 pins how far, at O0.
- **Call depth for M2.** Pending frames are not on `$sp`. A depth limit that reads only `$sp` would change behaviour. Section 8a gives the rule M2 must follow.
- **Wall-clock noise.** GC of the larger live heap at depth 1000 may push the ratio above 1.0. The 1.5 bound and the quiet window leave room. The fuel check in CI ignores GC.
- **Fuel in the CLI.** The CI check needs `-W fuel` together with GC, exceptions and tail calls in the pinned `wasmtime`. If that does not work, see open question 4.
- **Bench and A1+A2.** The bench and the run tests depend on the new syntax, so they can only land after A1+A2.
- **Errors through many handlers.** An error through a long chain of `fail.catch` or `ensure` records costs one call and one throw per handler. That is as today, but it is not free.
- **New `Env` variants.** `Point::delta` must follow every variant. The match has no `_` arm, so a new variant does not build until it is handled.

## 16. Open questions

1. **Scope.** Does the coordinator agree to these changes outside B's file list: `runtime/stops.rs` (one global and the call in `$stop`), `runtime/mod.rs` (one type and the protocol bullet), `emit/mod.rs` (one `DECLARED` name), the new spec cases in `tests/spec/9`, `tests/spec/9.7` and `tests/spec/11`, and the rebuilt `tests/browser/fixtures/compiled.wasm` (lane C's)?
2. **D4.** Is O(D) for resumes inside nested boundaries accepted? The alternative, cutting the pending list at a boundary that a suspend names, makes resume O(1 + E) but adds a new path that the existing boundary code does not cover.
3. **D11.** Should the list checks stay on in release builds? They cost one read and one write per record. The coordinator's measurement decides.
4. **Fuel.** If the pinned `wasmtime` CLI cannot run fuel with GC and exceptions, the fallback is a wall-clock check in `control_lazy.rs`: the minimum of 5 runs, a ratio of at most 2.0, and `#[ignore]` by default. Is that acceptable?
