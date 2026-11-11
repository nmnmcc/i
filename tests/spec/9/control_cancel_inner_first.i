// spec: 9.4, 9.5
// expect: stop "panic: inner"
(
	i/control
	i/resource
)

// Final functions run from the innermost to the outermost when a
// continuation is cancelled.

main = () {
	control.boundary((b) {
		resource.ensure(() {
			resource.ensure(() {
				control.suspend(b, (k) {
					control.cancel(k)
					1
				})
			}, () { panic("inner") })
		}, () { panic("outer") })
	})
	Nil
}
