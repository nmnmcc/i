//// Final functions (§9.5).
(
	ensure
	acquire
)

/// Runs `with` after `body`, whether `body` ends normally, ends with an
/// error, or is unwound.
@external("ilang:resource", "ensure")
ensure : <A, N>(() <- N -> A, () <- N -> Nil) <- N -> A

/// After `open` is done, runs `release` when `use` ends or is unwound.
acquire = <R, E, N, A>(
	open : () <- (Fail(E), ..N) -> R,
	release : (R) <- N -> Nil,
	use : (R) <- (Fail(E), ..N) -> A
) -> A {
	resource = open()
	ensure(() { use(resource) }, () { release(resource) })
}
