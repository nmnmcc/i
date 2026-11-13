// spec: 9.4, 9.5
// expect: pass

// EF06: cleanup may still open and use a fresh boundary of its own.
(
	i/control
	i/resource
)

main = () {
	answer = control.boundary((b) {
		resource.ensure(
			() { control.escape(b, 1) },
			() {
				inner = control.boundary((c) { control.escape(c, 2) })
				assert(inner == 2)
			},
		)
	})
	assert(answer == 1)
}
