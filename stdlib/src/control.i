//// One operation to pause a computation and go on with it later.
(
	boundary
	suspend
	escape
	resume
	cancel
	keep
)

/// Makes a boundary `b` and does `body(b)`.
@runtime("ilang:control", "boundary")
boundary : <R, N>((Boundary(R, N)) <- N -> R) <- N -> R

/// Pauses the computation back to the boundary `b`, and does `f(k)` in the
/// place of the boundary.
@runtime("ilang:control", "suspend")
suspend : <R, N, A>(Boundary(R, N), (Continuation(A, R, N)) <- N -> R) -> A

/// Ends the boundary `b` at once with `value`, as `suspend(b, (k) { cancel(k)
/// value })` does, but without keeping the paused computation: no frame
/// records are made, and `resource.ensure` functions inside `b` run as for an
/// error.
@runtime("ilang:control", "escape")
escape : <R, N>(Boundary(R, N), R) -> Never

/// Goes on with the paused computation, with `value` as the value of `suspend`.
@runtime("ilang:control", "resume")
resume : <A, R, N>(Continuation(A, R, N), A) <- N -> R

/// Unwinds the paused computation.
@runtime("ilang:control", "cancel")
cancel : <A, R, N>(Continuation(A, R, N)) <- N -> ()

/// Keeps the rest of the current environment; Fail comes from the later caller.
keep = <S, A, E, N>(f : (S) <- (..N, Fail(E)) -> A) <- (..env : N) -> (S) <- (
	Fail(E),
) -> A {
	(x) {
		// retain a frame across suspended provides.
		value = f(x) <- env
		value
	}
}
