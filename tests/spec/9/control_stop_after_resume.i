// spec: 9.4, 11.7
// expect: stop "panic: after"
// at: spec/control_stop_after_resume:13:3
// at: spec/control_stop_after_resume:19:5
// at: spec/control_stop_after_resume:12:28
// at: spec/control_stop_after_resume:18:3
(
	i/control
)

// After `resume`, a stop shows the positions of the resumed frames, on top
// of the place of `resume`.
step = (b) {
	control.suspend(b, (k) {
		control.resume(k, Nil)
		Nil
	})
	panic("after")
}

@main
main = () {
	control.boundary((b) {
		step(b)
		Nil
	})
	Nil
}
