// spec: 11.7, 9.4
// expect: stop "panic: inside"
// at: spec/main:21:3
// at: spec/main:15:7
// at: spec/main:26:9
// at: spec/main:25:7
// at: spec/main:37:5
// at: spec/main:42:11
(
	i/control
)

Paused = (Continuation(Nil, Paused, Never, Never))
Done = ()
Paused = Paused | Done

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
