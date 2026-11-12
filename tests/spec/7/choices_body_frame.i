// spec: 4.8, 6.12, 7.3
// expect: stop "panic: outer choices"
f = (s: String) {
	s |> ("a" | "c") {
		{
			"x" = s
			1
		} <- (Assert(fail = (_) { panic("body choices") }))
	}
}

main = () { f("b") <- (Assert(fail = (_) { panic("outer choices") })) }
