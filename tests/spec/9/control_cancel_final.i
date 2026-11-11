// spec: 9.4, 9.5, 11.7
// expect: stop "panic: final"
// at: spec/main:22:11
// at: spec/main:17:3
// at: spec/main:19:5
// at: spec/main:16:2
(
	i/control
	i/resource
)

// `cancel` unwinds the paused computation: its final functions run (§9.5),
// on top of the place of `cancel`.
@main
main = () {
	control.boundary((b) {
		resource.ensure(() {
			control.suspend(b, (k) {
				control.cancel(k)
				Nil
			})
		}, () { panic("final") })
		Nil
	})
	Nil
}
