// spec: 9.4, 9.5
// expect: stop "control: boundary is not active"

// EF06: the full boundary is already inactive when its final tries suspend.
(
	i/control
	i/resource
)

main = () {
	control.boundary((b) {
		resource.ensure(
			() { control.escape(b, 1) },
			() {
				control.suspend(b, (_) { 2 })
				()
			},
		)
	})
	()
}
