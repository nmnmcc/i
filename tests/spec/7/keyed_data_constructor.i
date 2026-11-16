// spec: 7.1, 7.2, 4.8
// expect: pass
Wrapper = (data = (name = String, age = Int))
Pair = ((name = String, age = Int))
Tuples = (x = (Int, Int))

read = (Wrapper(data = (name = n, age = _))) { n }
choose = (wrapped : Wrapper) {
	wrapped |>
		(Wrapper(data = whole = (age = 0, name = n))) { n <> " baby" <> whole.name }
		& (Wrapper(data = (name = n, age = _))) { n }
}

main = () {
	value = Wrapper(data = (age = 36, name = "Ada"))
	Wrapper(data = (name = n, age = _)) = value
	assert(n == "Ada")
	assert(read(value) == "Ada")
	assert(choose(value) == "Ada")
	assert(choose(Wrapper(data = (name = "Bea", age = 0))) == "Bea babyBea")
	Wrapper(data = whole = (name = _, age = a)) = value
	assert((whole.name, a) == ("Ada", 36))
	Pair((name = p, age = _)) = Pair((age = 42, name = "Cid"))
	assert(p == "Cid")
	Tuples(x = (left, right)) = Tuples(x = (1, 2))
	assert(left + right == 3)
}
