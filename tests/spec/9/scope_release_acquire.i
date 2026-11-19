// spec: 9.5
// expect: stop "panic: resource: scope is not open"
(
	i/fail
	i/resource
)

impossible = Fail(raise = (_ : Never) { panic("impossible") })

// The scope is closing when its release runs; an acquisition through the
// captured scope stops before `open`, with the scope message.
main = () {
	fail.catch(() {
		resource.scoped(() <- (s : resource.Scope) {
			resource.acquire(
				() { 1 },
				(_) {
					resource.acquire(() { 2 }, (_) { () }) <- (s, impossible)
					()
				},
			) <- (s, impossible)
			()
		})
	})
}
