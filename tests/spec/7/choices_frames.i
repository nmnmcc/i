// spec: 4.8, 6.12, 7.3
// expect: stop "panic: inner choices"
f = (s: String) -> Int {
	(a, b) = (1, 2)
	s |> ("a" | "c") { a } <- (Assert(fail = (_) { panic("inner choices") }))
}

main = () { f("b") }
