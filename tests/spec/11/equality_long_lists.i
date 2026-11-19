// spec: 11.4
// expect: pass
(
	testlib = (test) = std/test
	i/core
	list = std/list
)

// Long lists, so the comparison's continuation for each cell is exercised
// many times; the runtime reuses that continuation frame rather than
// allocating one per cell.
test("long_lists_of_tuples", () {
	xs = list.map(list.range(1, 20_000), (x) { (x, "a") })
	ys = list.map(list.range(1, 20_000), (x) { (x, "a") })
	assert(xs == ys)
	assert({ xs != ys } == False)

	// A difference in the last element is still found.
	zs = list.map(list.range(1, 20_000), (x) { (x, { x == 20_000 } |> (True) { "b" } & (False) { "a" }) })
	assert(xs != zs)
})

test("nested_lists", () {
	inner = list.range(1, 100)
	xs = list.map(list.range(1, 200), (_) { inner })
	ys = list.map(list.range(1, 200), (_) { list.range(1, 100) })
	assert(xs == ys)
})
