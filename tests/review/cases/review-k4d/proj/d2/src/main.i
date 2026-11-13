State(A) = (get = () -> A)
read = <A>() <- (s: State(A)) { s.get() }
outer = () {
	inner = () { read() }
	fa = () {
		_syn3_0: State(Int) = State(get = () { 1 })
		inner() + 1 <- (_syn3_0)
	}
	fb = () {
		_syn3_1: State(String) = State(get = () { "x" })
		inner() <> "y" <- (_syn3_1)
	}
	(fa(), fb())
}

main = () {
	outer()
	Nil
}
