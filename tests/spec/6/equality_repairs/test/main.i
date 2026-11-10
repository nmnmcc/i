// spec: 4.2, 6.9
// expect: pass
(list = std/list)

// Literal-types §8: pass constants as parameters to compare at runtime.
equal = (a, b) { a == b }
status = () -> String { "ok" }

@main
main = () {
	x = 1
	assert(equal(x, 2) == False)
	assert({status() == "error"} == False)
	assert(list.map(["a"], (x: String) { x == "b" }) == [False])
}
