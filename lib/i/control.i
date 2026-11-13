//// One operation to pause a computation and go on with it later (§9.4).

(
	boundary
	suspend
	escape
	resume
	cancel
	keep_env
)

/// Makes a boundary `b` and does `body(b)`.
@external("ilang:control", "boundary")
boundary(R, E, N): ((Boundary(R, E, N)) -> R ^ E ! N) -> R ^ E ! N

/// Pauses the computation back to the boundary `b`, and does `f(k)` in the
/// place of the boundary.
@external("ilang:control", "suspend")
suspend(R, E, N, A): (
	Boundary(R, E, N)
	(Continuation(A, R, E, N)) -> R ^ E ! N
) -> A

/// Ends the boundary `b` at once with `value`, as `suspend(b, (k) { cancel(k)
/// value })` does, but without keeping the paused computation: no frame
/// records are made, and `resource.ensure` functions inside `b` run as for an
/// error.
@external("ilang:control", "escape")
escape(R, E, N): (b: Boundary(R, E, N), value: R) -> Never

/// Goes on with the paused computation, with `value` as the value of `suspend`.
@external("ilang:control", "resume")
resume(A, R, E, N): (Continuation(A, R, E, N), A) -> R ^ E ! N

/// Unwinds the paused computation (§9.5).
@external("ilang:control", "cancel")
cancel(A, R, E, N): (Continuation(A, R, E, N)) -> Nil ! N

/// Keeps every service bound at this call, including services unused by f.
/// Resume and cancel inside the returned function keep these services too.
@external("ilang:control", "keep_env")
keep_env(S, A, E, N): ((S) -> A ^ E ! N) -> { (S) -> A ^ E } ! N
