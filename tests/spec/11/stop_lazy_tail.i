// spec: 11.7, 11.6, 9.4
// expect: stop "panic: after"
// at: spec/main:32:25
// at: spec/main:22:6
// at: spec/main:17:6
// at: spec/main:36:3
// at: spec/main:41:8
(
	i/control
)

Paused = (Continuation(Nil, PausedSum, Never, Never))

Done = ()
PausedSum = Paused | Done
w1 = (b) {
	x = w2(b)
	x
}

w2 = (b) {
	x = step(b)
	x
}

// After the resume, the segment of `step` makes a tail call.
step = (b) {
	control.suspend(b, (k) { Paused(k) })
	last(3)
}

last = (n) { n |> (0) { panic("after") } | (_) { last(n - 1) } }

main = () {
	p = control.boundary((b) {
		w1(b)
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
