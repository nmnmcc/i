// spec: 3, 4.8, 6.7
// expect: pass
(
	main
)

name_of = (r: (name = String)) -> String { r.name }
keep = (r: Record) -> Record { r }

main = () {
	x: (name = String, age = Int) = (age = 36, name = "Ada")
	assert(name_of(x) == "Ada")
	assert(keep(x) == x)
	assert(keep(()) == ())
	y: (other_key = Int) = (other_key = 5)
	assert(keep(y) == y)
	Nil
}
