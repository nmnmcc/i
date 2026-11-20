// spec: 9.4, 9.5
// expect: pass
(
	i/control
)

// A service provided to a block. Each round suspends through its `attach`
// and then calls a nested function that needs the same service again.
// Re-entering the suspended segment must keep the provided environment.
S = (attach = (() -> ()) -> (), peek = () -> Int)

check = (want : Int) <- (s : S) -> Int {
	{ s.peek() == 7 } |>
		(True()) { () }
		& (False()) { panic("peek") }
	want
}

step = (want : Int) <- (s : S) -> Int {
	s.attach(() { () })
	check(want)
}

scoped = (body : () <- (S) -> Int) -> Int {
	control.boundary((b) {
		{ body() } <- (
			S(
				peek = () { 7 },
				attach = (_) {
					control.suspend(b, (k) { control.resume(k, ()) })
				},
			),
		)
	})
}

main = () {
	sum = scoped(() {
		a = step(1)
		b = step(2)
		c = step(3)
		d = step(4)
		a + b + c + d
	})
	{ sum == 10 } |> (True()) { () } & (False()) { panic("sum") }
}
