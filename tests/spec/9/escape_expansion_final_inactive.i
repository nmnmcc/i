// spec: 9.4, 9.5
// expect: stop "control: boundary is not active"
(
	i/control
	i/resource
)

main = () {
	control.boundary((b) {
		resource.ensure(
			() {
				control.suspend(b, (k) {
					control.cancel(k)
					1
				})
			},
			() { control.escape(b, 2) },
		)
	})
	()
}
