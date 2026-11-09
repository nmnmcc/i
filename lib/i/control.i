//// One operation to pause a computation and go on with it later (§9.4).

export (
	boundary
	suspend
	resume
	cancel
)

/// Makes a boundary `b` and does `body(b)`.
@external("ilang:control", "boundary")
boundary: (body: (b: Boundary(r, e, n)) -> r ? e ! n) -> r ? e ! n

/// Pauses the computation back to the boundary `b`, and does `f(k)` in the
/// place of the boundary.
@external("ilang:control", "suspend")
suspend: (
	b: Boundary(r, e, n)
	f: (k: Continuation(a, r, e, n)) -> r ? e ! n
) -> a

/// Goes on with the paused computation, with `value` as the value of `suspend`.
@external("ilang:control", "resume")
resume: (k: Continuation(a, r, e, n), value: a) -> r ? e ! n

/// Unwinds the paused computation (§9.5).
@external("ilang:control", "cancel")
cancel: (k: Continuation(a, r, e, n)) -> Nil ! n
