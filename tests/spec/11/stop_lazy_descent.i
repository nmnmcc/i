// spec: 11.7, 9.4
// expect: stop "panic: inside"
// at: spec/main:25:2
// at: spec/main:19:6
// at: spec/main:30:7
// at: spec/main:29:6
// at: spec/main:40:3
// at: spec/main:45:8
(
	i/control
)

Paused = (Continuation(Nil, PausedSum, Never, Never))

Done = ()
PausedSum = Paused | Done
// A frame that waits inside `inner`, and one that waits outside it.
in1 = (outer, inner) {
	x = leaf(outer, inner)
	x
}

leaf = (outer, inner) {
	control.suspend(outer, (k) { Paused(k) })
	panic("inside")
}

out1 = (outer) {
	x = control.boundary((inner) {
		y = in1(outer, inner)
		y
	})
	x
}

// The resume goes into `inner` again: the stop inside it shows the frames
// of both lists.
main = () {
	p = control.boundary((outer) {
		out1(outer)
		Done
	})
	p |>
		(Paused(k)) {
			x = control.resume(k, Nil)
			x
		}
		| (Done) { panic("not paused") }
	Nil
}
