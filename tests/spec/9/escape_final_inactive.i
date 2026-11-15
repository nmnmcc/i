// spec: 9.4, 9.5
// expect: stop "control: boundary is not active"

// EF06: direct escape has the same cleanup behavior as suspend/cancel.
(
	i/control
	i/resource
)

main = () {
	control.boundary((b) {
		resource.ensure(
			() { control.escape(b, 1) },
			() { control.escape(b, 2) },
		)
	})
	Nil
}
