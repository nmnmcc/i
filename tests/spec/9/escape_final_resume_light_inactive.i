// spec: 9.4, 9.5
// expect: stop "control: boundary is not active"
(
	i/control
	i/resource
)

main = () {
	control.boundary((outer) {
		control.boundary((b) {
			resource.ensure(
				() { control.escape(b, 1) },
				() {
					control.suspend(outer, (k) { control.resume(k, ()) })
					control.escape(b, 2)
				},
			)
		})
	})
	()
}
