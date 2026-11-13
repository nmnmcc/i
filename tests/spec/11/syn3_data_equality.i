// spec: 11.4, 11.8
// expect: pass
// Syntax three stage 6: records are closed labelled data (syntax-3 §2.3).
(
	testlib = (test) = std/test
	i/core
)

pair = (x: (Int, Int) | (a = Int, b = Int)) { x }

test("labelled data equality", () {
	assert((a = 1) == (a = 1))
	assert((a = 1, b = 2) == (b = 2, a = 1))
	assert((a = 1) != (a = 2))
	assert(pair((1, 2)) != pair((a = 1, b = 2)))
	assert(pair((a = 1, b = 2)) == pair((b = 2, a = 1)))
	Nil
})

test("labelled data text", () {
	assert(core.inspect((a = 1)) == "(a = 1)")
	assert(core.inspect((b = "x", a = (1, 2))) == "(a = (1, 2), b = \"x\")")
	assert(core.inspect(()) == "()")
	Nil
})

test("spread and access", () {
	ada: (name = String, age = Int) = (name = "ada", age = 36)
	older = (..ada, age = 37)
	assert(older.age == 37)
	assert(older.name == "ada")
	assert(older == (age = 37, name = "ada"))
	Nil
})
