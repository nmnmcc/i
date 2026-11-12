// spec: 11.4, 10.1
// expect: pass

// Each `@external` declaration is one value; two declarations of one import
// are two values.
(
	testlib = (test) = std/test
)

@external("test:math/v1", "add")
add: (Int, Int) -> Int

@external("test:math/v1", "add")
plus: (Int, Int) -> Int

check = (c: Bool, what: String) {
	c |> (True) { Nil } | (False) { panic(what) }
}

same = (f: (Int, Int) -> Int, g: (Int, Int) -> Int) { f == g }

test("declarations_are_values", () {
	check(add == add, "add is add")
	check(same(add, add), "add is add as a value")
	check(same(add, plus) == False, "add is not plus")
	check(add != plus, "add is not plus with !=")
	check(add(1, 2) == plus(1, 2), "both call the import")
	f = plus
	check(same(f, plus), "plus is plus")
})
