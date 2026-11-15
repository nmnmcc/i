// spec: 7.1, 7.2, 4.8
// expect: pass
Box = (value = Int)

// Label order in a pattern is independent of construction order.
(name = name, age = _) = (age = 36, name = "Ada")

read = ((name = n, age = a)) { (n, a) }
choose = (data : (name = String, age = Int)) {
	data |> ((age = 0, name = n)) { n <> " baby" } | ((name = n, age = _)) { n }
}

// Equal part counts do not make different label sets one shape.
shape = (data : { (a = Int) | (b = String) }) {
	data |>
		((a = n)) { n }
		| ((b = s)) { { s == "yes" } |> (True) { 1 } | (False) { 0 } }
}

main = () {
	assert(name == "Ada")
	assert(shape((a = 3)) == 3)
	assert(shape((b = "yes")) == 1)
	assert(read((age = 36, name = "Ada")) == ("Ada", 36))
	assert(choose((name = "Ada", age = 36)) == "Ada")
	assert(choose((age = 0, name = "Bea")) == "Bea baby")
	(outer = (right = Box(value = v), left = [first, ..rest]), skip = _) = (
		skip = False,
		outer = (left = [2, 3], right = Box(value = 7)),
	)
	assert((v, first, rest) == (7, 2, [3]))
	// Conjunctions bind the field value; the label binds no name.
	(field = whole = (x, y)) = (field = (4, 5))
	assert((whole, x, y) == ((4, 5), 4, 5))
	// Positions precede labels; discards add no hidden binders.
	(a, z = _, b = b) = (..(1), b = 2, z = 3)
	assert((a, b) == (1, 2))
	// Known facts flow through keyed parts to later key reads.
	(item = item) = (item = (answer = 42))
	assert(item.answer == 42)
}
