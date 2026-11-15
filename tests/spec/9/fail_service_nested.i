// spec: 4.8, 9.2, 9.3
// expect: pass

// EF13: forwarding and service patterns preserve the selected handler.
(
	i/control
	i/fail
)

raise = (s : String) <- (h : Fail(String)) { h.raise(s) }

main = () {
	answer = control.boundary((b) {
		outer = Fail(raise = (s : String) { control.escape(b, "outer " <> s) })
		inner = Fail(raise = (s : String) { control.escape(b, "inner " <> s) })
		// The inner provide wins for the same constructor.
		raise("x") <- (inner) <- (outer)
	})
	assert(answer == "inner x")
	answer2 = control.boundary((b) {
		fail.raise("y") <- (
			Fail(raise = (s : String) { control.escape(b, "direct " <> s) }),
		)
	})
	assert(answer2 == "direct y")
}
