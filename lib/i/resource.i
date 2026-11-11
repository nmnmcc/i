//// Final functions (§9.5).

(
	ensure
	acquire
)

/// Runs `with` after `body`, whether `body` ends normally, ends with an
/// error, or is unwound.
@external("ilang:resource", "ensure")
ensure(A, E, N): (body: () -> A ^ E ! N, with: () -> Nil ! N) -> A ^ E ! N

/// After `open` is done, runs `release` when `use` ends or is unwound.
acquire(R, E, N, A) = (
	open: () -> R ^ E ! N
	release: (R) -> Nil ! N
	use: (R) -> A ^ E ! N
) -> A ^ E ! N {
	resource = open()
	ensure(() { use(resource) }, () { release(resource) })
}
