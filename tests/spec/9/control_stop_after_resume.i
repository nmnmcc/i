// spec: 9.4, 11.7
// expect: stop "panic: after"
// at: spec/main:18:2
// at: spec/main:24:3
// at: spec/main:15:3
// at: spec/main:23:2
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
