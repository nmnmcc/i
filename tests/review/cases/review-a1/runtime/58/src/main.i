id(A) = (x: A) -> A { x }

f = () { (id(42), id("s")) }


main = () {
	assert(f() == (42, "s"))
	Nil
}
