State(A) = (get = () -> A)
read = <A>() <- (s: State(A)) { s.get() }
outer = () {
	inner = () { read() }
	fa = () { inner() + 1 <- (State(get = () { 1 })) }
	fb = () { inner() <> "y" <- (State(get = () { "x" })) }
	(fa(), fb())
}

main = () {
	outer()
	Nil
}
