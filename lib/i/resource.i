//// Final functions (§9.5).

export (
	ensure
	acquire
)

/// Runs `with` after `body`, whether `body` ends normally, ends with an
/// error, or is unwound.
@external("ilang:resource", "ensure")
ensure: (body: () -> a ? e ! n, with: () -> Nil ! n) -> a ? e ! n

/// After `open` is done, runs `release` when `use` ends or is unwound.
acquire = (
	open: () -> r ? e ! n
	release: (r) -> Nil ! n
	use: (r) -> a ? e ! n
) -> a ? e ! n {
	resource = open()
	ensure(() { use(resource) }, () { release(resource) })
}
