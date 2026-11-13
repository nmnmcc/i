// spec: 4.5, 9.2, 9.3
// expect: compile-error E0501
(
	i/failure
)

// Rule B: in `N - Fail(E)` the row `N` has no entry for `Fail`, so the
// errors of the body all go to the handler: `E` is `Int | String`, and the
// `String` error cannot be left to the caller's `Fail(String)`.
split = () -> Result(Nil, Int) ! Fail(String) {
	failure.catch(() {
		failure.raise(1)
		failure.raise("s")
	})
}
