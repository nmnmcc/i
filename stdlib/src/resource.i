//// Final functions and one resource lifetime service.
(
	i/control
)

(
	Scope
	ensure
	acquire
	scoped
)

/// The lifetime of resources: a service made only by `scoped`. Each
/// `acquire` attaches a release to the current `Scope`. `gate` is `Ready`
/// while the scope is open and `Taken` from the moment its body ends, which
/// is how `acquire` checks the state before it calls `open`.
Scope = (attach = (() -> ()) -> (), gate = Request(()))

/// Runs `with` after `body`, whether `body` ends normally, ends with an
/// error, or is unwound.
@external("ilang:resource", "ensure")
ensure : <A, N>(() <- N -> A, () <- N -> ()) <- N -> A

// The scope's one-way state cell. These are the ordinary request operations;
// a completed fake request is a mutable cell that only goes from `Ready` to
// `Taken` (i/request).
@external("ilang:request", "ready")
gate_ready : <A>(A) -> Request(A)

@external("ilang:request", "state")
gate_state : <A>(Request(A)) -> Result(A, Int)

@external("ilang:request", "take")
gate_take : <A>(Request(A)) -> A

/// Makes `release` a function that needs nothing: its services are the row
/// of the resource's `open`, captured here. A release has no `Fail` need, so
/// one that can raise an error does not type.
pure_release = <A, N>(release : (A) <- N -> ()) <- (..env : N) -> (A) -> () {
	(x) { release(x) <- env }
}

/// Calls `open`, attaches `release(resource)` to the current `Scope`, and
/// gives the resource. When the scope is not open, it stops with
/// `panic("resource: scope is not open")` before calling `open`.
///
/// `open` is the only part that can wait. From the moment `open` returns
/// until the release is attached, nothing yields to the scheduler: attaching
/// is one synchronous `control.suspend` and `control.resume`, so a cancel
/// cannot come between them and lose the resource.
acquire = <A, E, N>(
	open : () <- (..N, Fail(E)) -> A,
	release : (A) <- N -> ()
) <- (scope : Scope, ..env : N) -> A {
	scope.gate
		|> gate_state |>
		(Ok(_)) { () }
		& (Error(_)) { panic("resource: scope is not open") }
	value = open()
	free = pure_release(release)
	scope.attach(() { free(value) })
	value
}

/// Runs `body` with a new `Scope` and gives its value. The releases attached
/// to the scope run when `body` ends, in reverse order, also when it ends by
/// an error, an escape or a cancel. The scope becomes closing before the
/// first release runs, so a release that acquires through it stops with
/// `panic("resource: scope is not open")`; a release that needs a resource
/// opens a new scope.
///
/// A `Scope` belongs to the task that made it: a child task does not share
/// it (the specification's resource contract) and opens its own
/// `resource.scoped`. Attaching to a scope
/// from another task stops with `panic("control: boundary is not active")`.
///
/// The `Scope` is closed after `scoped` has ended and stays closed.
scoped = <A, N>(body : () <- (..N, Scope) -> A) <- (..env : N) -> A {
	control.boundary((b) {
		gate = gate_ready(())
		ensure(
			() {
				{ body() } <- (
					Scope(
						attach = (release) {
							// Runs the rest of `body` under `ensure`, so that
							// the releases run innermost (last attached) first.
							control.suspend(b, (k) {
								ensure(() { control.resume(k, ()) }, release)
							})
						},
						gate = gate,
					),
				)
			},
			// Runs before the first attached release, on every exit.
			() { gate_take(gate) },
		)
	})
}
