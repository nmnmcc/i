// spec: 9.10
// expect: pass
(
	testlib = (test) = std/test
	i/random
	i/fail
	i/core
)

// §9.10: immutable reuse, normalized seeds, and the complete logical identity.
test("identity", () {
	g = random.from_seed(0)
	w = 0x1_0000_0000_0000_0000
	assert(g == random.from_seed(w))
	assert(random.from_seed(-1) == random.from_seed(w - 1))
	assert(random.from_seed(-w * w - 17) == random.from_seed(w - 17))
	assert(random.word(g) == random.word(g))
	assert(random.split(g) == random.split(g))
	assert(random.at_path(g, []) == g)
	(_, next) = random.word(g)
	assert(next != g)
	assert(random.at_path(next, []) == next)
	assert(random.split(next) != random.split(g))
	assert(random.at_path(next, [1]) != random.at_path(g, [1]))
	Nil
})

// §9.10: InvalidPath contains the whole input, including after a valid prefix.
test("invalid", () {
	g = random.from_seed(19)
	assert(fail.catch(() { random.int(g, 0) }) == Error(random.InvalidBound(0)))
	assert(
		fail.catch(() { random.int(g, -123) }) ==
			Error(random.InvalidBound(-123)),
	)
	assert(
		fail.catch(() { random.at_path(g, [0]) }) ==
			Error(random.InvalidPath([0])),
	)
	assert(
		fail.catch(() { random.at_path(g, [3, -1, 2]) }) ==
			Error(random.InvalidPath([3, -1, 2])),
	)
	assert(
		random.word(g) == random.word(random.from_seed(19)),
	)
	Nil
})

// §9.10: successive path components compose; P and S histories stay distinct.
test("paths", () {
	g = random.from_seed(7)
	(a, b) = random.split(g)
	assert(a != b)
	assert(a != random.at_path(g, [1]))
	assert(b != random.at_path(g, [2]))
	assert(
		random.at_path(g, [1, 2, 3]) ==
			random.at_path(random.at_path(g, [1]), [2, 3]),
	)
	large = core.shift_left(1, 200) + 1
	assert(random.at_path(g, [large]) != random.at_path(g, [1]))
	assert(
		random.at_path(g, [large, 2]) ==
			random.at_path(random.at_path(g, [large]), [2]),
	)
	assert(
		random.at_path(a, [1]) != random.split(random.at_path(g, [1])).0,
	)
	// Interleaving reads of other streams does not change a stream's sequence.
	p = random.at_path(g, [1])
	q = random.at_path(g, [2])
	(_, p1) = random.word(p)
	(_, q1) = random.word(q)
	assert(
		random.word(p1) == random.word(random.word(p).1),
	)
	assert(
		random.word(q1) == random.word(random.word(q).1),
	)
	Nil
})

advance = (g: random.Generator, n: Int) -> random.Generator {
	{ n == 0 } |> (True) { g } | (False) { advance(random.word(g).1, n - 1) }
}

// §9.10: k is at least one, even for bound 1; W uses one word, W+1 two.
test("boundaries", () {
	g = random.from_seed(0)
	w = core.shift_left(1, 64)
	assert(random.int(g, 1) == (0, advance(g, 1)))
	assert(random.int(g, w) == random.word(g))
	(u, h) = random.word(g)
	(v, next) = random.word(h)
	assert(random.int(g, w + 1) == ({ u + v * w } % { w + 1 }, next))
	// Powers of W accept every group; the word order is least significant first.
	assert(random.int(g, w * w) == (u + v * w, next))
	Nil
})

check_range = (g: random.Generator, below: Int, count: Int) -> Nil {
	{ count == 0 } |>
		(True) { Nil }
		| (False) {
			(value, next) = random.int(g, below)
			assert(value >= 0 && value < below)
			(f, _) = random.float(g)
			assert(f >=. 0.0 && f <. 1.0)
			check_range(next, below, count - 1)
		}
}

test("ranges", () {
	g = random.from_seed(-19)
	check_range(g, 1, 100)
	check_range(g, 10, 1000)
	check_range(g, core.shift_left(1, 63) + 1, 100)
	check_range(g, core.shift_left(1, 193) + 123, 100)
	Nil
})

// §9.10: exactly 53 high bits and exactly one draw are used for each Float.
check_float = (g: random.Generator, count: Int) -> Nil {
	{ count == 0 } |>
		(True) { Nil }
		| (False) {
			(u, next) = random.word(g)
			(f, after) = random.float(g)
			assert(after == next)
			assert(
				core.truncate(f *. 9007199254740992.0) ==
					core.shift_right(u, 11),
			)
			check_float(next, count - 1)
		}
}

test("floats", () { check_float(random.from_seed(1337), 1000) })

// A deterministic smoke check of small-bound frequencies. This is not a proof
// of independence; the exact rejection and uniform-residue proof are separate.
histogram = (g: random.Generator, count: Int, a: Int, b: Int, c: Int) -> (
	Int,
	Int,
	Int,
) {
	{ count == 0 } |>
		(True) { (a, b, c) }
		| (False) {
			(n, next) = random.int(g, 3)
			n |>
				(0) { histogram(next, count - 1, a + 1, b, c) }
				| (1) { histogram(next, count - 1, a, b + 1, c) }
				| (_) { histogram(next, count - 1, a, b, c + 1) }
		}
}

test("small_bound_frequencies", () {
	(a, b, c) = histogram(random.from_seed(42), 12000, 0, 0, 0)
	assert(a > 3600 && a < 4400)
	assert(b > 3600 && b < 4400)
	assert(c > 3600 && c < 4400)
	Nil
})

walk = (g: random.Generator, count: Int) -> random.Generator {
	{ count == 0 } |>
		(True) { g }
		| (False) { walk(random.split(g).0, count - 1) }
}

test("deep_splits", () {
	g = random.from_seed(0)
	assert(
		walk(walk(g, 64), 64) == walk(g, 128),
	)
	assert(walk(g, 128) != walk(g, 127))
	Nil
})
