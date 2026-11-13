State(A) = (get = () -> A)
read = <A>() <- (s : State(A)) { s.get() }
outer = (runner) {
	inner = () {
		c = () { read() }
		runner(c)
		c
	}
	a = inner()
	b = inner()
	fa = () {
		_syn3_0 : State(Int) = State(get = () { 1 })
		a() + 1 <- (_syn3_0)
	}
	fb = () {
		_syn3_1 : State(String) = State(get = () { "s" })
		b() <> "t" <- (_syn3_1)
	}
	(fa(), fb())
}

main = () {
	_syn3_2 : State(Int) = State(get = () { 5 })
	{
		outer((c : () <- (State(Int)) -> Int) { c() })
		Nil
	} <- (_syn3_2)
}
