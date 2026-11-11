// spec: 11.7, 9.4
// expect: stop "panic: after"
// at: spec/stop_tail_resume:14:2
// at: spec/stop_tail_resume:20:3
// at: spec/stop_tail_resume:19:2
(
	i/control
)

// A call of `control.resume` in tail position counts as a tail call: after
// it, the resumed frames are on top of the place of the boundary.
step = (b) {
	control.suspend(b, (k) { control.resume(k, Nil) })
	panic("after")
}


main = () {
	control.boundary((b) {
		step(b)
		Nil
	})
	Nil
}
