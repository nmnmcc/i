(
	testlib = (test) = i/test
	i/fail
	i/control
	sequence = i/sequence
	list = i/list
	option = (Some, None) = i/option
)

check = (s : sequence.Sequence(Int), xs : List(Int)) -> Nil {
	assert(sequence.to_list(s) == xs)
	assert(sequence.length(s) == list.length(xs))
	assert(sequence.is_empty(s) == list.is_empty(xs))
	assert(sequence.first(s) == list.first(xs))
	assert(sequence.last(s) == list.last(xs))
	Nil
}

// The example table of the sequence documentation, including saved source versions.
test("sequence_examples", () {
	empty = sequence.new()
	assert(sequence.first(empty) == None)
	assert(sequence.last(empty) == None)
	assert(sequence.pop_front(empty) == None)
	assert(sequence.pop_back(empty) == None)
	fifo = sequence.push_back(
		sequence.push_back(sequence.push_back(empty, 1), 2),
		3,
	)
	Some((a, tail)) = sequence.pop_front(fifo)
	Some((b, tail)) = sequence.pop_front(tail)
	Some((c, tail)) = sequence.pop_front(tail)
	assert((a, b, c) == (1, 2, 3))
	assert(sequence.pop_front(tail) == None)
	Some((x, rest)) = sequence.pop_back(
		sequence.push_front(sequence.from_list([2, 3]), 1),
	)
	assert(x == 3)
	check(rest, [1, 2])
	original = sequence.from_list([10, 20])
	assert(sequence.at(original, -1) == None)
	assert(sequence.at(original, 0) == Some(10))
	assert(sequence.at(original, 2) == None)
	Some(changed) = sequence.set(original, 1, 7)
	check(changed, [10, 7])
	check(original, [10, 20])
	assert(sequence.set(original, -1, 7) == None)
	assert(sequence.set(original, 2, 7) == None)
	check(sequence.wrap(5), [5])
	check(
		sequence.reverse(
			sequence.append(sequence.from_list([1, 2]), sequence.wrap(3)),
		),
		[3, 2, 1],
	)
	Nil
})

// Sequence laws compare observable elements, never private representation equality.
check_laws = (xs : List(Int), ys : List(Int), zs : List(Int)) -> Nil {
	a = sequence.from_list(xs)
	b = sequence.from_list(ys)
	c = sequence.from_list(zs)
	check(a, xs)
	assert(
		sequence.equal(
			sequence.from_list(sequence.to_list(a)),
			a,
		),
	)
	assert(
		sequence.equal(
			sequence.append(a, sequence.new()),
			a,
		),
	)
	assert(
		sequence.equal(
			sequence.append(sequence.new(), a),
			a,
		),
	)
	assert(
		sequence.equal(
			sequence.append(sequence.append(a, b), c),
			sequence.append(a, sequence.append(b, c)),
		),
	)
	assert(
		sequence.length(sequence.append(a, b)) ==
			sequence.length(a) + sequence.length(b),
	)
	assert(
		sequence.equal(
			sequence.reverse(sequence.reverse(a)),
			a,
		),
	)
	list.each(
		list.range(-1, list.length(xs) + 2),
		(i) {
			(left, right) = sequence.split(a, i)
			check(left, list.take(xs, i))
			check(right, list.drop(xs, i))
			assert(
				sequence.equal(sequence.append(left, right), a),
			)
			check(sequence.take(a, i), list.take(xs, i))
			check(sequence.drop(a, i), list.drop(xs, i))
			assert(sequence.at(a, i) == list.at(xs, i))
			sequence.set(a, i, -10) |>
				(None) { assert(i < 0 || i >= list.length(xs)) }
				& (Some(changed)) {
					check(
						changed,
						list.index_map(xs, (x, j) {
							{ i == j } |> (True) { -10 } & (False) { x }
						}),
					)
				}
			check(a, xs)
		},
	)
	Nil
}

test("sequence_small_exhaustive", () {
	list.each(list.range(0, 20), (n) {
		list.each(list.range(0, 20), (m) {
			check_laws(
				numbers(n),
				list.map(numbers(m), (x) { 100 + x }),
				[999],
			)
		})
	})
	Nil
})

// A service observes callback order without putting mutable state in sequences.
Trace = (visit = (Int) -> Nil)

visit = (x : Int) <- (trace : Trace) -> Nil { trace.visit(x) }

test("sequence_walks", () {
	source = sequence.from_list([1, 2, 3])
	assert(
		sequence.to_list(sequence.map(source, (x) { x * 2 })) == [2, 4, 6],
	)
	assert(sequence.fold(source, 0, (a, x) { a + x }) == 6)
	assert(sequence.fold(source, [], (a, x) { [x, ..a] }) == [3, 2, 1])
	assert(sequence.each(source, (x) { x * 2 }) == Nil)
	check(source, [1, 2, 3])
	// Nested boundaries record a trace through a pure supplied service.
	trace = control.boundary((b) {
		{
			sequence.each(source, visit)
			[]
		} <- (
			Trace(
				visit = (x) {
					control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
				},
			),
		)
	})
	assert(trace == [1, 2, 3])
	Nil
})

reject_two = (x : Int) -> Int {
	x |>
		(2) { fail.raise("two") }
		& (3) { panic("callback after first error") }
		& (_) { x }
}

test("sequence_callback_errors", () {
	s = sequence.from_list([1, 2, 3])
	assert(fail.catch(() { sequence.map(s, reject_two) }) == Error("two"))
	assert(
		fail.catch(() { sequence.fold(s, 0, (a, x) { a + reject_two(x) }) }) ==
			Error("two"),
	)
	assert(fail.catch(() { sequence.each(s, reject_two) }) == Error("two"))
	check(s, [1, 2, 3])
	Nil
})

// A fixed generator branches from retained versions, which must stay unchanged.
prng = (seed : Int) -> Int { { seed * 1664525 + 1013904223 } % 4294967296 }

model = (
	seed : Int,
	remaining : Int,
	versions : List((sequence.Sequence(Int), List(Int)))
) -> Nil {
	remaining |>
		(0) {
			list.each(versions, (v) {
				(s, xs) = v
				check(s, xs)
			})
		}
		& (_) {
			next = prng(seed)
			Some((s, xs)) = list.at(versions, next % list.length(versions))
			n = list.length(xs)
			i = prng(next) % { n + 3 } - 1
			x = next % 1000
			(t, ys) = next % 10 |>
				(0) { (sequence.push_front(s, x), [x, ..xs]) }
				& (1) { (sequence.push_back(s, x), list.append(xs, [x])) }
				& (2) {
					sequence.pop_front(s) |>
						(None) { (s, xs) }
						& (Some((v, rest))) {
							assert(Some(v) == list.first(xs))
							(rest, list.drop(xs, 1))
						}
				}
				& (3) {
					sequence.pop_back(s) |>
						(None) { (s, xs) }
						& (Some((v, rest))) {
							assert(Some(v) == list.last(xs))
							(rest, list.take(xs, n - 1))
						}
				}
				& (4) {
					sequence.set(s, i, x) |>
						(None) {
							assert(i < 0 || i >= n)
							(s, xs)
						}
						& (Some(rest)) {
							(
								rest,
								list.index_map(xs, (v, j) {
									{ i == j } |> (True) { x } & (False) { v }
								}),
							)
						}
				}
				& (5) { (sequence.reverse(s), list.reverse(xs)) }
				& (6) { (sequence.take(s, i), list.take(xs, i)) }
				& (7) { (sequence.drop(s, i), list.drop(xs, i)) }
				& (8) {
					Some((other, zs)) = list.at(
						versions,
						prng(next) % list.length(versions),
					)
					(sequence.append(s, other), list.append(xs, zs))
				}
				& (_) { (sequence.append(s, s), list.append(xs, xs)) }
			check(t, ys)
			check(s, xs)
			assert(sequence.at(t, i) == list.at(ys, i))
			assert(
				sequence.equal(sequence.from_list(ys), t),
			)
			model(prng(next), remaining - 1, [(t, ys), ..versions])
		}
}

test("sequence_persistent_model", () {
	list.each([1, 7, 42, 123456], (seed) {
		model(
			seed,
			600,
			[
				(sequence.new(), []),
				(sequence.from_list(list.range(0, 100)), list.range(0, 100)),
			],
		)
	})
	Nil
})

// Traverse all the way through middle borrowing/collapse, alternating ends.
drain = (s : sequence.Sequence(Int), low : Int, high : Int) -> Nil {
	{ low >= high } |>
		(True) { assert(sequence.is_empty(s)) }
		& (False) {
			{ { low + high } % 2 == 0 } |>
				(True) {
					Some((x, rest)) = sequence.pop_front(s)
					assert(x == low)
					drain(rest, low + 1, high)
				}
				& (False) {
					Some((x, rest)) = sequence.pop_back(s)
					assert(x == high - 1)
					drain(rest, low, high - 1)
				}
		}
}

test("sequence_large_boundaries", () {
	sizes = [
		0,
		1,
		2,
		3,
		4,
		7,
		8,
		9,
		15,
		16,
		17,
		26,
		27,
		28,
		31,
		32,
		33,
		63,
		64,
		65,
		80,
		81,
		82,
		127,
		128,
		129,
		242,
		243,
		244,
		255,
		256,
		257,
		511,
		512,
		513,
		728,
		729,
		730,
		1023,
		1024,
		1025,
		2186,
		2187,
		2188,
		4095,
		4096,
		4097,
		6560,
		6561,
		6562,
		8191,
		8192,
		8193,
		16383,
		16384,
		16385,
		19682,
		19683,
		19684,
		32767,
		32768,
		32769,
		59048,
		59049,
		59050,
		65535,
		65536,
		65537,
		99999,
		100000,
	]
	list.each(sizes, (n) {
		xs = numbers(n)
		s = sequence.from_list(xs)
		check(s, xs)
		check(sequence.reverse(s), list.reverse(xs))
		drain(s, 0, n)
		list.each([-1, 0, 1, n / 3, n / 2, n - 1, n, n + 1], (i) {
			(left, right) = sequence.split(s, i)
			check(left, list.take(xs, i))
			check(right, list.drop(xs, i))
			assert(
				sequence.equal(sequence.append(left, right), s),
			)
			assert(sequence.at(s, i) == list.at(xs, i))
		})
		check(
			sequence.append(s, sequence.wrap(-1)),
			list.append(xs, [-1]),
		)
		check(
			sequence.append(sequence.wrap(-1), s),
			[-1, ..xs],
		)
		check(sequence.append(s, s), list.append(xs, xs))
		check(s, xs)
	})
	Nil
})

numbers = (n : Int) -> List(Int) {
	{ n <= 0 } |> (True) { [] } & (False) { list.range(0, n - 1) }
}

Box = (value = Int)

observe = (body : () <- (Trace, Assert) -> Nil) -> List(Int) {
	control.boundary((b) {
		{
			body()
			[]
		} <- (
			Trace(
				visit = (x) {
					control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
				},
			),
		)
	})
}

test("sequence_callback_types_and_order", () {
	xs = numbers(300)
	s = sequence.from_list(xs)
	assert(
		observe(() {
			mapped = sequence.map(s, (x) {
				visit(x)
				x + 1
			})
			assert(sequence.length(mapped) == 300)
		}) ==
			xs,
	)
	assert(
		observe(() {
			n = sequence.fold(s, 0, (a, x) {
				visit(x)
				a + 1
			})
			assert(n == 300)
		}) ==
			xs,
	)
	assert(observe(() { sequence.each(s, visit) }) == xs)
	boxes = sequence.map(s, (x) { Box(value = x) })
	check(sequence.map(boxes, (b : Box) { b.value }), xs)
	assert(
		sequence.to_list(
			sequence.map(sequence.from_list(["a", "é"]), (x) { x <> "!" }),
		) ==
			["a!", "é!"],
	)
	union_error = (x : Int) -> Int {
		{ x == 1 } |>
			(True) { fail.raise(True) }
			& (False) { fail.raise("other") }
	}
	assert(
		fail.catch(() { sequence.each(sequence.wrap(1), union_error) }) ==
			Error(True),
	)
	assert(
		fail.catch(() { sequence.map(sequence.wrap(2), union_error) }) ==
			Error("other"),
	)
	Nil
})

// Unrelated shapes compare by elements. Both repeated insertion
// directions exercise full digits, promoted middle nodes, and retained roots.
test("sequence_independent_histories", () {
	xs = numbers(512)
	back = list.fold(xs, sequence.new(), (s, x) { sequence.push_back(s, x) })
	front = list.fold(list.reverse(xs), sequence.new(), (s, x) {
		sequence.push_front(s, x)
	})
	joined = sequence.append(
		sequence.from_list(list.take(xs, 211)),
		sequence.from_list(list.drop(xs, 211)),
	)
	assert(sequence.equal(back, front))
	assert(sequence.equal(front, joined))
	check(back, xs)
	check(front, xs)
	check(joined, xs)
	versions = list.fold(numbers(128), [(back, xs)], (saved, x) {
		[(old, model), .._] = saved
		(next, expected) = { x % 2 == 0 } |>
			(True) { (sequence.push_front(old, -x), [-x, ..model]) }
			& (False) { (sequence.push_back(old, x), list.append(model, [x])) }
		check(next, expected)
		[(next, expected), ..saved]
	})
	list.each(versions, (v) {
		(s, model) = v
		check(s, model)
	})
	Nil
})
// Cached sizes, ranks and bounded digits are the private sequence invariants.
