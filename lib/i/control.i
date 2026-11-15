//// One operation to pause a computation and go on with it later (§9.4).
(
	boundary
	suspend
	escape
	resume
	cancel
	keep
)

/// Makes a boundary `b` and does `body(b)`.
@external("ilang:control", "boundary")
boundary : <R, N>((Boundary(R, N)) <- N -> R) <- N -> R

/// Pauses the computation back to the boundary `b`, and does `f(k)` in the
/// place of the boundary.
@external("ilang:control", "suspend")
suspend : <R, N, A>(Boundary(R, N), (Continuation(A, R, N)) <- N -> R) -> A

/// Ends the boundary `b` at once with `value`, as `suspend(b, (k) { cancel(k)
/// value })` does, but without keeping the paused computation: no frame
/// records are made, and `resource.ensure` functions inside `b` run as for an
/// error.
@external("ilang:control", "escape")
escape : <R, N>(Boundary(R, N), R) -> Never

/// Goes on with the paused computation, with `value` as the value of `suspend`.
@external("ilang:control", "resume")
resume : <A, R, N>(Continuation(A, R, N), A) <- N -> R

/// Unwinds the paused computation (§9.5).
@external("ilang:control", "cancel")
cancel : <A, R, N>(Continuation(A, R, N)) <- N -> Nil

/// Keeps the rest of the current environment; Fail comes from the later caller.
keep = <S, A, E, N>(f : (S) <- (..N, Fail(E)) -> A) <- (..env : N) -> (S) <- (
	Fail(E),
) -> A {
	(x) {
		// runtime.md §6.3, D16: retain a frame across suspended provides.
		value = f(x) <- env
		value
	}
}
