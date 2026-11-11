// spec: 9.4, 9.5, 11.7
// expect: stop "panic: final"
// at: spec/control_cancel_final:22:11
// at: spec/control_cancel_final:17:3
// at: spec/control_cancel_final:19:5
// at: spec/control_cancel_final:16:2
(
	i/control
	i/resource
)

// `cancel` unwinds the paused computation: its final functions run (§9.5),
// on top of the place of `cancel`.

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
