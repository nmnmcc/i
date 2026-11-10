//// One operation to pause a computation and go on with it later (§9.4).

(
	boundary
	suspend
	resume
	cancel
)

/// Makes a boundary `b` and does `body(b)`.
@external("ilang:control", "boundary")
boundary(R, E, N): (body: (b: Boundary(R, E, N)) -> R ? E ! N) -> R ? E ! N

/// Pauses the computation back to the boundary `b`, and does `f(k)` in the
/// place of the boundary.
@external("ilang:control", "suspend")
suspend(R, E, N, A): (
	b: Boundary(R, E, N)
	f: (k: Continuation(A, R, E, N)) -> R ? E ! N
) -> A

/// Goes on with the paused computation, with `value` as the value of `suspend`.
@external("ilang:control", "resume")
resume(A, R, E, N): (k: Continuation(A, R, E, N), value: A) -> R ? E ! N

/// Unwinds the paused computation (§9.5).
@external("ilang:control", "cancel")
cancel(A, R, E, N): (k: Continuation(A, R, E, N)) -> Nil ! N
