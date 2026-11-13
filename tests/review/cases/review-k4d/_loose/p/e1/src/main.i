State(A) = (get = () -> A)
read = <A>() <- (s: State(A)) { s.get() }
outer = (runner) {
	inner = () {
		c = () { read() }
		runner(c)
		c
	}
	a = inner()
	b = inner()
	fa = () { a() + 1 <- (State(get = () { 1 })) }
	fb = () { b() <> "t" <- (State(get = () { "s" })) }
	(fa(), fb())
}

main = () {
	{
		outer((c: () <- (State(Int)) -> Int) { c() })
		Nil
	} <- (State(get = () { 5 }))
}
