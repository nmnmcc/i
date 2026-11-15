(
	testlib = (test) = std/test
	review/pbt
	i/random
	std/list
	(Some) = std/option
)

walk = (t : pbt.Tree((Int, Int)), depth : Int) -> Nil {
	(n, k) = t.value
	assert(k >= 0 && k <= n)
	{ depth > 0 } |>
		(True) { list.each(t.children(), (c) { walk(c, depth - 1) }) }
		| (False) { Nil }
}

test("dependent", () {
	gen = pbt.flat_map(pbt.int(0, 6), (n) {
		pbt.map(pbt.int(0, n), (k) { (n, k) })
	})
	go = (seed : Int) -> Nil {
		{ seed < 20 } |>
			(True) {
				Some(t) = pbt.sample(gen, 10, random.from_seed(seed))
				walk(t, 4)
				go(seed + 1)
			}
			| (False) { Nil }
	}
	go(0)
})

test("greedy", () {
	(v, path, checked) = pbt.shrink(pbt.int_tree(10, 0), 100, (n) { n >= 6 })
	assert(v == 6)
	assert(path == [2, 2])
	assert(checked == 9)
})

test("zero_budget", () {
	never = () { never() }
	(v, path, checked) = pbt.shrink(pbt.Tree(
		value = 1,
		children = () { never() },
	), 0, (_) { True })
	assert(v == 1)
	assert(path == [])
	assert(checked == 0)
})

test("cyclic_budget", () {
	cyclic = () -> pbt.Tree(Int) {
		pbt.Tree(value = 1, children = () { [cyclic()] })
	}
	(v, path, checked) = pbt.shrink(cyclic(), 10, (_) { True })
	assert(v == 1)
	assert(list.length(path) == 10)
	assert(checked == 10)
})

test("map_has_no_numeric_smaller_law", () {
	original = pbt.make((_, _) { Some(pbt.int_tree(10, 0)) })
	mapped = pbt.map(original, (n) { 100 - n })
	Some(t) = pbt.sample(mapped, 0, random.from_seed(0))
	(v, _, _) = pbt.shrink(t, 100, (_) { True })
	assert(t.value == 90)
	assert(v == 100)
})
