(
	testlib = (test) = std/test
	i/core
	i/random
	i/fail
	i/assertion
	std/property
	std/list
	std/string
	std/sequence
	(Some, None) = std/option
)

snapshot(A) = (t: property.Tree(A), depth: Int) -> String {
	core.inspect(t.value) <> {
		{ depth == 0 } |> {
			True -> ""
			False -> core.inspect(list.map(t.children(), (c) {
				snapshot(c, depth - 1)
			}))
		}
	}
}

root(A) = (
	gen: property.Gen(A)
	size: Int
	seed: Int
) -> property.Tree(A) ! Assert {
	Some(t) = property.sample(gen, size, random.from_seed(seed))
	t
}

// Find a root with a source-specified shrink sequence, without fixing a seed
// from this implementation as an independent expected random oracle.
find_ten = (gen: property.Gen(Int), seed: Int) -> property.Tree(Int) ! Assert {
	assert(seed < 1000)
	t = root(gen, 0, seed)
	{ t.value == 10 } |> {
		True  -> t
		False -> find_ten(gen, seed + 1)
	}
}

test("sampling_and_map_laws", () {
	leaf = root(property.constant(7), -5, 42)
	assert(leaf.value == 7)
	assert(leaf.children() == [])
	sizes = property.make((s, _) { Some(property.tree(s, () { [] })) })
	assert(root(sizes, -1, 0).value == 0)
	Ok(gen) = property.int(-100, 100)
	g = random.from_seed(17)
	Some(a) = property.sample(gen, 9, g)
	Some(b) = property.sample(gen, 9, g)
	assert(snapshot(a, 3) == snapshot(b, 3))
	assert(
		snapshot(root(property.map(gen, (x) { x }), 9, 17), 3) == snapshot(a, 3)
	)
	composed = property.map(property.map(gen, (x) { x + 2 }), (x) { x * x })
	direct = property.map(gen, (x) { { x + 2 } * { x + 2 } })
	assert(
		snapshot(root(composed, 9, 17), 3) == snapshot(root(direct, 9, 17), 3)
	)
	duplicates = property.make((_, _) {
		Some(property.tree(1, () {
			[property.tree(-1, () { [] }), property.tree(1, () { [] })]
		}))
	})
	mapped = root(property.map(duplicates, (x) { x * x }), 0, 0)
	assert(list.map(mapped.children(), (c) { c.value }) == [1, 1])
})

test("integer_rules", () {
	assert(property.int(3, 2) == Error(property.InvalidRange(3, 2)))
	assert(property.element([]) == Error(property.EmptyChoice))
	assert(
		property.list(property.constant(0), -1, 2) ==
			Error(property.InvalidLength(-1, 2))
	)
	assert(
		property.list(property.constant(0), 3, 2) ==
			Error(property.InvalidLength(3, 2))
	)
	assert(property.string(-2) == Error(property.InvalidLength(0, -2)))
	Ok(gen) = property.int(0, 10)
	t = find_ten(gen, 0)
	assert(list.map(t.children(), (c) { c.value }) == [0, 5, 8, 9])
	negative = root(
		property.map(property.make((_, _) { Some(t) }), (x) { -x })
		0
		0
	)
	assert(list.map(negative.children(), (c) { c.value }) == [0, -5, -8, -9])
	verify = (
		node: property.Tree(Int)
		target: Int
		depth: Int
	) -> Nil ! Assert {
		assert(node.value >= 0 && node.value <= 10)
		{ depth == 0 } |> {
			True -> Nil
			False -> list.each(node.children(), (c) {
				assert(c.value < node.value)
				verify(c, target, depth - 1)
			})
		}
	}
	verify(t, 0, 8)
	Ok(positive) = property.int(3, 10)
	p = root(positive, 0, 0)
	{ p.value == 3 } |> {
		True -> assert(p.children() == [])
		False -> assert(list.first(p.children()) |> {
			Some(c) -> c.value == 3
			None    -> False
		})
	}
})

valid_dependent = (t: property.Tree((Int, Int)), depth: Int) -> Nil ! Assert {
	assert(t.value.1 >= 0 && t.value.1 <= t.value.0)
	{ depth == 0 } |> {
		True  -> Nil
		False -> list.each(t.children(), (c) { valid_dependent(c, depth - 1) })
	}
}

test("dependent_and_size", () {
	Ok(outer) = property.int(0, 5)
	dependent = property.flat_map(outer, (n) {
		property.int(0, n) |> {
			Ok(gen)  -> property.map(gen, (k) { (n, k) })
			Error(_) -> property.constant((n, -1))
		}
	})
	list.each(list.range(0, 19), (seed) {
		valid_dependent(root(dependent, 4, seed), 4)
	})
	sizes = property.sized((s) { property.constant(s) })
	assert(root(property.resize(sizes, -3), 9, 0).value == 0)
	assert(root(sizes, 9, 0).value == 9)
	resized = property.resize(
		property.flat_map(property.constant(1), (_) { sizes })
		3
	)
	assert(root(resized, 9, 0).value == 3)
	pair = property.pair(outer, sizes)
	explicit = property.flat_map(outer, (x) {
		property.map(sizes, (y) { (x, y) })
	})
	assert(snapshot(root(pair, 4, 19), 4) == snapshot(root(explicit, 4, 19), 4))
})

test("filter_prunes_whole_subtrees", () {
	gen = property.make((_, _) {
		Some(property.tree(4, () {
			[
				property.tree(1, () { [property.tree(2, () { [] })] })
				property.tree(0, () { [] })
			]
		}))
	})
	t = root(property.filter(gen, (x) { x % 2 == 0 }), 0, 0)
	assert(list.map(t.children(), (c) { c.value }) == [0])
	assert(
		property.check(
			property.filter(property.constant(0), (x) { x > 0 })
			property.Config(
				seed:         0
				cases:        1
				max_size:     0
				max_discards: 0
				max_shrinks:  0
			)
			(_) { property.Pass }
		) ==
			property.GaveUp(successes: 0, discards: 1)
	)
})

test("greedy_path_budget_and_replay", () {
	Ok(ints) = property.int(0, 10)
	ten = find_ten(ints, 0)
	gen = property.make((_, _) { Some(ten) })
	predicate = (x: Int) -> property.Verdict {
		{ x >= 6 } |> {
			True  -> property.Fail(core.inspect(x))
			False -> property.Pass
		}
	}
	property.Falsified(f) = property.check(
		gen
		property.Config(
			seed:         -1
			cases:        1
			max_size:     0
			max_discards: 0
			max_shrinks:  100
		)
		predicate
	)
	assert(f.original == 10 && f.smallest == 6)
	assert(f.path == [2, 2])
	assert(f.checked == 9 && core.not(f.limited))
	assert(f.reason == "6" && f.seed == 0xffffffffffffffff)
	assert(property.replay(gen, f.seed, f.attempt, f.size, f.path) == Some(6))
	property.Falsified(short) = property.check(
		gen
		property.Config(
			seed:         0
			cases:        1
			max_size:     0
			max_discards: 0
			max_shrinks:  2
		)
		predicate
	)
	assert(
		short.smallest == 10 &&
			short.checked == 2 &&
			short.path == [] &&
			short.limited
	)
	never = () { never() }
	lazy = property.make((_, _) { Some(property.tree(10, () { never() })) })
	property.Falsified(zero) = property.check(
		lazy
		property.Config(
			seed:         0
			cases:        1
			max_size:     0
			max_discards: 0
			max_shrinks:  0
		)
		predicate
	)
	assert(
		zero.smallest == 10 &&
			zero.checked == 0 &&
			zero.path == [] &&
			zero.limited
	)
	cyclic = () -> property.Tree(Int) { property.tree(10, () { [cyclic()] }) }
	cycle = property.make((_, _) { Some(cyclic()) })
	property.Falsified(c) = property.check(
		cycle
		property.Config(
			seed:         0
			cases:        1
			max_size:     0
			max_discards: 0
			max_shrinks:  7
		)
		predicate
	)
	assert(c.checked == 7 && c.limited && list.length(c.path) == 7)
	assert(property.replay(gen, 0, -1, 0, []) == None)
	assert(property.replay(gen, 0, 0, -1, []) == None)
	assert(property.replay(gen, 0, 0, 0, [-1]) == None)
	assert(property.replay(gen, 0, 0, 0, [99]) == None)
	assert(property.replay(property.make((_, _) { None }), 0, 0, 0, []) == None)
})

test("structural_shrinking", () {
	Ok(ints) = property.int(0, 10)
	ten = find_ten(ints, 0)
	gen = property.map(property.make((_, _) { Some(ten) }), (x) { 100 - x })
	property.Falsified(f) = property.check(
		gen
		property.Config(
			seed:         0
			cases:        1
			max_size:     0
			max_discards: 0
			max_shrinks:  10
		)
		(_) { property.Fail("all") }
	)
	assert(f.original == 90 && f.smallest == 100)
})

test("config_and_protocol", () {
	invalid = property.make((_, _) { panic("invalid config called generator") })
	predicate = (_) { panic("invalid config called predicate") }
	assert(
		property.defaults() ==
			property.Config(
				seed:         0
				cases:        100
				max_size:     100
				max_discards: 1000
				max_shrinks:  1000
			)
	)
	assert(
		property.check(
			invalid
			property.Config(
				seed:         0
				cases:        0
				max_size:     -1
				max_discards: -1
				max_shrinks:  -1
			)
			predicate
		) ==
			property.InvalidConfig("cases must be positive")
	)
	assert(
		property.check(
			invalid
			property.Config(
				seed:         0
				cases:        1
				max_size:     -1
				max_discards: -1
				max_shrinks:  -1
			)
			predicate
		) ==
			property.InvalidConfig("max_size must be nonnegative")
	)
	assert(
		property.check(
			invalid
			property.Config(
				seed:         0
				cases:        1
				max_size:     0
				max_discards: -1
				max_shrinks:  -1
			)
			predicate
		) ==
			property.InvalidConfig("max_discards must be nonnegative")
	)
	assert(
		property.check(
			invalid
			property.Config(
				seed:         0
				cases:        1
				max_size:     0
				max_discards: 0
				max_shrinks:  -1
			)
			predicate
		) ==
			property.InvalidConfig("max_shrinks must be nonnegative")
	)
	no_show = (_) { panic("nonfailure called show") }
	assert(
		property.format(property.Passed(successes: 3, discards: 2), no_show) ==
			"std/property v1: passed; successes: 3; discards: 2"
	)
	assert(
		property.format(property.GaveUp(successes: 0, discards: 1), no_show) ==
			"std/property v1: gave up; successes: 0; discards: 1"
	)
	assert(
		property.format(property.InvalidConfig("bad"), no_show) ==
			"std/property v1: invalid config; bad"
	)
	report = property.Falsified(
		property.Failure(
			seed:     0
			attempt:  1
			size:     2
			original: 10
			smallest: 6
			reason:   "why\nnext"
			path:     [2, 1]
			checked:  8
			limited:  False
		)
	)
	assert(
		property.format(report, core.inspect) ==
			"std/property v1: falsified\nseed: 0\nattempt: 1\nsize: 2\npath: [2, 1]\nchecked: 8\nlimited: False\noriginal: 10\nsmallest: 6\nreason: why\nnext"
	)
})

test("attempt_streams_seed_and_prefixes", () {
	gen = property.make((s, g) {
		Some(property.tree((s, random.word(g).0), () { [] }))
	})
	list.each(list.range(0, 8), (j) {
		Ok(g) = fail.catch(() { random.at_path(random.from_seed(0), [j + 1]) })
		Some(value) = property.replay(gen, 0, j, j % 3, [])
		assert(value == (j % 3, random.word(g).0))
		assert(
			property.replay(gen, 0x10000000000000000, j, j % 3, []) ==
				Some(value)
		)
	})
	Ok(ints) = property.int(-100, 100)
	predicate = (x: Int) -> property.Verdict { property.Fail(core.inspect(x)) }
	a = property.check(
		ints
		property.Config(
			seed:         0
			cases:        1
			max_size:     2
			max_discards: 0
			max_shrinks:  20
		)
		predicate
	)
	b = property.check(
		ints
		property.Config(
			seed:         0x10000000000000000
			cases:        5
			max_size:     2
			max_discards: 100
			max_shrinks:  20
		)
		predicate
	)
	assert(a == b)
	sizes = property.sized((s) { property.constant(s) })
	assert(
		property.check(
			sizes
			property.Config(
				seed:         0
				cases:        3
				max_size:     2
				max_discards: 6
				max_shrinks:  0
			)
			(s) {
				{ s == 2 } |> {
					True  -> property.Pass
					False -> property.Discard
				}
			}
		) ==
			property.Passed(successes: 3, discards: 6)
	)
})

test("list_order_and_generated_bounds", () {
	Ok(gen) = property.list(property.constant(7), 0, 4)
	find_four = (seed: Int) -> property.Tree(List(Int)) ! Assert {
		assert(seed < 1000)
		t = root(gen, 4, seed)
		{ list.length(t.value) == 4 } |> {
			True  -> t
			False -> find_four(seed + 1)
		}
	}
	t = find_four(0)
	assert(
		list.map(t.children(), (c) { list.length(c.value) }) ==
			[0, 2, 2, 3, 3, 3, 3]
	)
	Ok(items) = property.int(-3, 3)
	Ok(lists) = property.list(items, 2, 5)
	verify = (t: property.Tree(List(Int)), depth: Int) -> Nil ! Assert {
		assert(list.length(t.value) >= 2 && list.length(t.value) <= 5)
		assert(list.all(t.value, (x) { x >= -3 && x <= 3 }))
		{ depth == 0 } |> {
			True  -> Nil
			False -> list.each(t.children(), (c) { verify(c, depth - 1) })
		}
	}
	list.each(list.range(0, 19), (seed) { verify(root(lists, 5, seed), 3) })
	assert(list.length(root(lists, 0, 0).value) == 2)
	assert(
		property.sample(property.list(property.make((_, _) { None }), 1, 1) |> {
			Ok(g)    -> g
			Error(_) -> property.constant([])
		}, 1, random.from_seed(0)) == None
	)
})

test("finite_and_scalar_generators", () {
	floats = property.float()
	scalars = property.utf_codepoint()
	Ok(strings) = property.string(5)
	list.each(list.range(0, 99), (seed) {
		t = root(floats, 3, seed)
		bits = core.float_to_bits(t.value)
		assert(core.bit_and(core.shift_right(bits, 52), 2047) < 2047)
		(left, right) = random.split(random.from_seed(seed))
		Ok((sign, _)) = fail.catch(() { random.int(left, 2) })
		Ok((magnitude, _)) = fail.catch(() {
			random.int(right, 0x7ff0000000000000)
		})
		assert(bits == sign * 0x8000000000000000 + magnitude)
		c = core.codepoint_to_int(root(scalars, 3, seed).value)
		assert(c >= 0 && c <= 0x10ffff && { c < 0xd800 || c > 0xdfff })
		assert(string.length(root(strings, 3, seed).value) <= 3)
	})
})

test("assertion_adapter_and_clients", () {
	report = assertion.catch(() {
		property.test(
			property.constant(1)
			property.Config(
				seed:         42
				cases:        1
				max_size:     0
				max_discards: 0
				max_shrinks:  0
			)
			(x) { assert(x == 0) }
			core.inspect
		)
	})
	Error(failure) = report
	assert(string.contains(failure.message, "seed: 42"))
	assert(string.contains(failure.message, "original: 1\nsmallest: 1"))
	property.test(
		property.constant(1)
		property.defaults()
		(x) { assert(x == 1) }
		core.inspect
	)
	Ok(items) = property.int(-10, 10)
	Ok(lists) = property.list(items, 0, 10)
	property.test(
		lists
		property.Config(
			seed:         0
			cases:        30
			max_size:     10
			max_discards: 0
			max_shrinks:  100
		)
		(xs) {
			assert(list.reverse(list.reverse(xs)) == xs)
			assert(sequence.to_list(sequence.from_list(xs)) == xs)
		}
		core.inspect
	)
	Ok(strings) = property.string(20)
	property.test(
		strings
		property.Config(
			seed:         0
			cases:        30
			max_size:     20
			max_discards: 0
			max_shrinks:  100
		)
		(s) { assert(core.from_codepoints(core.codepoints(s)) == s) }
		core.inspect
	)
})

test("negative_integer_boolean_element_and_list_element_order", () {
	Ok(negative) = property.int(-10, 0)
	find = (seed: Int) -> property.Tree(Int) ! Assert {
		assert(seed < 1000)
		t = root(negative, 0, seed)
		{ t.value == -10 } |> {
			True  -> t
			False -> find(seed + 1)
		}
	}
	assert(list.map(find(0).children(), (c) { c.value }) == [0, -5, -8, -9])
	samples = list.map(list.range(0, 49), (seed) {
		root(property.bool(), 0, seed)
	})
	assert(list.any(samples, (t) { t.value }))
	assert(list.any(samples, (t) { core.not(t.value) }))
	list.each(samples, (t) {
		assert(list.map(t.children(), (c) { c.value }) == {
			t.value |> {
				True  -> [False]
				False -> []
			}
		})
	})
	Ok(choices) = property.element(["first", "second", "first"])
	list.each(list.range(0, 49), (seed) {
		t = root(choices, 0, seed)
		assert(t.value == "first" || t.value == "second")
		Ok((index, _)) = fail.catch(() {
			random.int(random.from_seed(seed), 3)
		})
		expected = index |> {
			0 -> []
			1 -> ["first"]
			_ -> ["first", "second"]
		}
		assert(list.map(t.children(), (c) { c.value }) == expected)
	})
	// With fixed length there are no deletions. Retaining per-element trees
	// gives the first element's children, then the second's, without redrawing.
	hand = property.make((_, _) {
		Some(property.tree(1, () { [property.tree(0, () { [] })] }))
	})
	Ok(fixed) = property.list(hand, 2, 2)
	t = root(fixed, 0, 0)
	assert(list.map(t.children(), (c) { c.value }) == [[0, 1], [1, 0]])
})

test("budget_prefixes_and_discarded_children", () {
	Ok(ints) = property.int(0, 10)
	ten = find_ten(ints, 0)
	gen = property.make((_, _) { Some(ten) })
	// P7 also counts Discard; it never descends into that child.
	predicate = (x: Int) -> property.Verdict {
		{ x >= 6 } |> {
			True  -> property.Fail(core.inspect(x))
			False -> property.Discard
		}
	}
	expected = [
		([], 10)
		([], 10)
		([2], 8)
		([2], 8)
		([2], 8)
		([2, 2], 6)
		([2, 2], 6)
		([2, 2], 6)
		([2, 2], 6)
	]
	list.index_map(expected, (pair, index) {
		property.Falsified(f) = property.check(
			gen
			property.Config(
				seed:         0
				cases:        1
				max_size:     0
				max_discards: 0
				max_shrinks:  index + 1
			)
			predicate
		)
		assert(f.checked == index + 1 && f.limited)
		assert((f.path, f.smallest) == pair)
	})
	Nil
})

test("dependent_branch_pruning_and_inner_alternatives", () {
	outer = property.make((_, _) {
		Some(property.tree(2, () {
			[
				property.tree(1, () { [property.tree(0, () { [] })] })
				property.tree(0, () { [] })
			]
		}))
	})
	gen = property.flat_map(outer, (n) {
		property.make((s, _) {
			n |> {
				1 -> None
				0 -> Some(property.tree(0, () { [] }))
				_ -> Some(property.tree(20 + s, () {
					[property.tree(21 + s, () { [] })]
				}))
			}
		})
	})
	t = root(gen, 3, 17)
	assert(t.value == 23)
	children = t.children()
	assert(list.map(children, (c) { c.value }) == [0, 24])
	Some(inner) = list.at(children, 1)
	assert(list.map(inner.children(), (c) { c.value }) == [0])
	// The failed n=1 branch's n=0 grandchild is not promoted; only the second
	// outer child appears. The inner shrink keeps this current outer alternative.
	assert(list.length(children) == 2)
})
