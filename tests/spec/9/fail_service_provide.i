// spec: 4.8, 9.2, 9.3
// expect: stop "panic: custom x"

// EF13: an explicitly provided Fail overrides the platform service.
(
	i/fail
)

main = () {
	fail.raise("x") <- (Fail(raise = (s: String) { panic("custom " <> s) }))
}
