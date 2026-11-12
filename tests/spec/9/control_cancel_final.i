// spec: 9.4, 9.5, 11.7
// expect: stop "panic: final"
// at: spec/main:23:9
// at: spec/main:16:3
// at: spec/main:19:6
// at: spec/main:15:2
(
	i/control
	i/resource
)

// `cancel` unwinds the paused computation: its final functions run (§9.5),
// on top of the place of `cancel`.
main = () {
	control.boundary((b) {
		resource.ensure(
			() {
				control.suspend(b, (k) {
					control.cancel(k)
					Nil
				})
			},
			() { panic("final") },
		)
		Nil
	})
	Nil
}
