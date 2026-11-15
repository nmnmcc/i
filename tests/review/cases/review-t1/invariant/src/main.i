Cell(A) = (run = (A) -> A)
make = () { Cell(run = (x) { x }) }
ci : Cell(Int) = make()
cs : Cell(String) = make()
id = <A>(x : A) -> A { x }

main = () {
	assert(ci.run(3) == 3)
	assert(cs.run("b") == "b")
	assert(id(1) == 1)
	Nil
}
