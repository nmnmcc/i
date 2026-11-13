make = (n) { (x) -> Int { x + n } }

f = () { make(40)(2) }

main = () {
	assert(f() == 42)
	Nil
}
