// spec: 4.2, 6.3, 6.9
// expect: pass

// Decisions 4/5: singleton values are usable where their whole scalar is needed.
(
	testlib = (test) = std/test
)

need_int = (n : Int) { n + 1 }

need_string = (s : String) { s <> "!" }

equal = (a, b) { a == b }

kind = ("a") { 1 } | ("b") { 2 }

total = () -> Int {
	"a" = "a"
	1
}

test("literals", () {
	assert(need_int(-{ { 1 } }) == 0)
	assert(need_string("a") == "a!")
	assert(equal(-1, -{ 1 }))
	assert(equal(-{ -1 }, 1))
	x = 1
	assert(equal(-x, -1))
	assert(equal([1, 2], [1, 2]))
	assert(equal([1, "a"], [1, "a"]))
	assert(kind("a") == 1)
	assert(kind("b") == 2)
	assert(total() == 1)
})
