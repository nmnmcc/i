//// Final functions (§9.5).
(
	ensure
	acquire
)

// A final callback excludes Fail while the body may report E (§9.5).
FinalNeed(N) = N - Fail(Never)

/// Runs `with` after `body`, whether `body` ends normally, ends with an
/// error, or is unwound.
@external("ilang:resource", "ensure")
ensure: <A, E, N>(() <- (Fail(E), ..N) -> A, () <- (FinalNeed(N)) -> Nil) <- (
	Fail(E),
	..N,
) -> A

/// After `open` is done, runs `release` when `use` ends or is unwound.
acquire = <R, E, N, A>(
	open: () <- (Fail(E), ..N) -> R,
	release: (R) <- (FinalNeed(N)) -> Nil,
	use: (R) <- (Fail(E), ..N) -> A
) -> A {
	resource = open()
	ensure(() { use(resource) }, () { release(resource) })
}
