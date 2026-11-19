Cell(A) = (get = () -> A, put = (A) -> ())
mk = <A>(x : A) { Cell(get = () { x }, put = (_ : A) { () }) }
f = () {
	c : Cell(Int) = mk(0)
	c.put(1)
	c.get()
}

main = () {
	assert(f() == 0)
	()
}
