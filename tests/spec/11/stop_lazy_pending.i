// spec: 11.7, 9.4
// expect: stop "panic: after"
// at: spec/main:35:2
// at: spec/main:29:6
// at: spec/main:24:6
// at: spec/main:19:6
// at: spec/main:42:3
// at: spec/main:45:21
(
	i/control
)

Paused = (Continuation(Nil, PausedSum, Never, Never))

Done = ()
PausedSum = Paused | Done
// Three frames that wait.
f1 = (b) {
	x = f2(b)
	x
}

f2 = (b) {
	x = f3(b)
	x
}

f3 = (b) {
	x = leaf(b)
	x
}

leaf = (b) {
	control.suspend(b, (k) { Paused(k) })
	panic("after")
}

// After the resume, the stop shows the innermost frame, the three frames
// that wait, then the place of `resume`.
main = () {
	p = control.boundary((b) {
		f1(b)
		Done
	})
	p |> (Paused(k)) { control.resume(k, Nil) } | (Done) { panic("not paused") }
	Nil
}
