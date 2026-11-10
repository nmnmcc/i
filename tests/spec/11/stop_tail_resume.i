// spec: 11.7, 9.4
// expect: stop "panic: after"
// at: spec/stop_tail_resume:12:3
// at: spec/stop_tail_resume:18:5
// at: spec/stop_tail_resume:17:3
(
	i/control
)

// A call of `control.resume` in tail position counts as a tail call: after
// it, the resumed frames are on top of the place of the boundary.
step = (b) {
	control.suspend(b, (k) { control.resume(k, Nil) })
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
