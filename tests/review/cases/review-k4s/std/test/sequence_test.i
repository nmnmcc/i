(
	i/fail
	i/control
	sequence = std/sequence
	list = std/list
	option = (Some, None) = std/option
)

check = (s: sequence.Sequence(Int), xs: List(Int)) -> Nil ! Assert {
	assert(sequence.to_list(s) == xs)
	assert(sequence.length(s) == list.length(xs))
	assert(sequence.is_empty(s) == list.is_empty(xs))
	assert(sequence.first(s) == list.first(xs))
	assert(sequence.last(s) == list.last(xs))
	Nil
}

// std-1 §4.3's example table, Q1 and Q2, including saved source versions.
@test
sequence_examples = () {
	empty = sequence.new()
	assert(sequence.first(empty) == None)
	assert(sequence.last(empty) == None)
	assert(sequence.pop_front(empty) == None)
	assert(sequence.pop_back(empty) == None)
	fifo = sequence.push_back(
		sequence.push_back(sequence.push_back(empty, 1), 2)
		3
	)
	Some((a, tail)) = sequence.pop_front(fifo)
	Some((b, tail)) = sequence.pop_front(tail)
	Some((c, tail)) = sequence.pop_front(tail)
	assert((a, b, c) == (1, 2, 3))
	assert(sequence.pop_front(tail) == None)
	Some((x, rest)) = sequence.pop_back(
		sequence.push_front(sequence.from_list([2, 3]), 1)
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
			sequence.append(sequence.from_list([1, 2]), sequence.wrap(3))
		)
		[3, 2, 1]
	)
	Nil
}

// §4.3 laws compare observable elements, never private representation equality.
check_laws = (xs: List(Int), ys: List(Int), zs: List(Int)) -> Nil ! Assert {
	a = sequence.from_list(xs)
	b = sequence.from_list(ys)
	c = sequence.from_list(zs)
	check(a, xs)
	assert(sequence.equal(sequence.from_list(sequence.to_list(a)), a))
	assert(sequence.equal(sequence.append(a, sequence.new()), a))
	assert(sequence.equal(sequence.append(sequence.new(), a), a))
	assert(
		sequence.equal(
			sequence.append(sequence.append(a, b), c)
			sequence.append(a, sequence.append(b, c))
		)
	)
	assert(
		sequence.length(sequence.append(a, b)) ==
			sequence.length(a) + sequence.length(b)
	)
	assert(sequence.equal(sequence.reverse(sequence.reverse(a)), a))
	list.each(list.range(-1, list.length(xs) + 2), (i) {
		(left, right) = sequence.split(a, i)
		check(left, list.take(xs, i))
		check(right, list.drop(xs, i))
		assert(sequence.equal(sequence.append(left, right), a))
		check(sequence.take(a, i), list.take(xs, i))
		check(sequence.drop(a, i), list.drop(xs, i))
		assert(sequence.at(a, i) == list.at(xs, i))
		sequence.set(a, i, -10) |> {
			None -> assert(i < 0 || i >= list.length(xs))
			Some(changed) -> check(changed, list.index_map(xs, (x, j) {
				{ i == j } |> {
					True  -> -10
					False -> x
				}
			}))
		}
		check(a, xs)
	})
	Nil
}

@test
sequence_small_exhaustive = () {
	list.each(list.range(0, 20), (n) {
		list.each(list.range(0, 20), (m) {
			check_laws(numbers(n), list.map(numbers(m), (x) { 100 + x }), [999])
		})
	})
	Nil
}

// A service observes callback order without putting mutable state in sequences.
Trace {
	Trace(visit: (Int) -> Nil)
}

visit = (x: Int, trace ! Trace) -> Nil ! Trace { trace.visit(x) }

@test
sequence_walks = () {
	source = sequence.from_list([1, 2, 3])
	assert(sequence.to_list(sequence.map(source, (x) { x * 2 })) == [2, 4, 6])
	assert(sequence.fold(source, 0, (a, x) { a + x }) == 6)
	assert(sequence.fold(source, [], (a, x) { [x, ..a] }) == [3, 2, 1])
	assert(sequence.each(source, (x) { x * 2 }) == Nil)
	check(source, [1, 2, 3])
	// Nested boundaries record a trace through a pure supplied service.
	trace = control.boundary((b) {
		! Trace = Trace((x) {
			control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
		})
		sequence.each(source, visit)
		[]
	})
	assert(trace == [1, 2, 3])
	Nil
}

reject_two = (x: Int) -> Int ? String {
	x |> {
		2 -> fail.raise("two")
		3 -> panic("callback after first error")
		_ -> x
	}
}

@test
sequence_callback_errors = () {
	s = sequence.from_list([1, 2, 3])
	assert(fail.catch(() { sequence.map(s, reject_two) }) == Error("two"))
	assert(
		fail.catch(() { sequence.fold(s, 0, (a, x) { a + reject_two(x) }) }) ==
			Error("two")
	)
	assert(fail.catch(() { sequence.each(s, reject_two) }) == Error("two"))
	check(s, [1, 2, 3])
	Nil
}

// A fixed generator branches from retained versions, as required by §4.3.
prng = (seed: Int) -> Int { { seed * 1664525 + 1013904223 } % 4294967296 }

model = (
	seed: Int
	remaining: Int
	versions: List((sequence.Sequence(Int), List(Int)))
) -> Nil ! Assert {
	remaining |> {
		0 -> list.each(versions, (v) {
			(s, xs) = v
			check(s, xs)
		})
		_ -> {
			next = prng(seed)
			Some((s, xs)) = list.at(versions, next % list.length(versions))
			n = list.length(xs)
			i = prng(next) % { n + 3 } - 1
			x = next % 1000
			(t, ys) = next % 10 |> {
				0 -> (sequence.push_front(s, x), [x, ..xs])
				1 -> (sequence.push_back(s, x), list.append(xs, [x]))
				2 -> sequence.pop_front(s) |> {
					None -> (s, xs)
					Some((v, rest)) -> {
						assert(Some(v) == list.first(xs))
						(rest, list.drop(xs, 1))
					}
				}
				3 -> sequence.pop_back(s) |> {
					None -> (s, xs)
					Some((v, rest)) -> {
						assert(Some(v) == list.last(xs))
						(rest, list.take(xs, n - 1))
					}
				}
				4 -> sequence.set(s, i, x) |> {
					None -> {
						assert(i < 0 || i >= n)
						(s, xs)
					}
					Some(rest) -> (rest, list.index_map(xs, (v, j) {
						{ i == j } |> {
							True  -> x
							False -> v
						}
					}))
				}
				5 -> (sequence.reverse(s), list.reverse(xs))
				6 -> (sequence.take(s, i), list.take(xs, i))
				7 -> (sequence.drop(s, i), list.drop(xs, i))
				8 -> {
					Some((other, zs)) = list.at(
						versions
						prng(next) % list.length(versions)
					)
					(sequence.append(s, other), list.append(xs, zs))
				}
				_ -> (sequence.append(s, s), list.append(xs, xs))
			}
			check(t, ys)
			check(s, xs)
			assert(sequence.at(t, i) == list.at(ys, i))
			assert(sequence.equal(sequence.from_list(ys), t))
			model(prng(next), remaining - 1, [(t, ys), ..versions])
		}
	}
}

@test
sequence_persistent_model = () {
	list.each([1, 7, 42, 123456], (seed) {
		model(
			seed
			600
			[
				(sequence.new(), [])
				(sequence.from_list(list.range(0, 100)), list.range(0, 100))
			]
		)
	})
	Nil
}

// Traverse all the way through middle borrowing/collapse, alternating ends.
drain = (s: sequence.Sequence(Int), low: Int, high: Int) -> Nil ! Assert {
	{ low >= high } |> {
		True -> assert(sequence.is_empty(s))
		False -> { { low + high } % 2 == 0 } |> {
			True -> {
				Some((x, rest)) = sequence.pop_front(s)
				assert(x == low)
				drain(rest, low + 1, high)
			}
			False -> {
				Some((x, rest)) = sequence.pop_back(s)
				assert(x == high - 1)
				drain(rest, low, high - 1)
			}
		}
	}
}

@test
sequence_large_boundaries = () {
	sizes = [
		0
		1
		2
		3
		4
		7
		8
		9
		15
		16
		17
		26
		27
		28
		31
		32
		33
		63
		64
		65
		80
		81
		82
		127
		128
		129
		242
		243
		244
		255
		256
		257
		511
		512
		513
		728
		729
		730
		1023
		1024
		1025
		2186
		2187
		2188
		4095
		4096
		4097
		6560
		6561
		6562
		8191
		8192
		8193
		16383
		16384
		16385
		19682
		19683
		19684
		32767
		32768
		32769
		59048
		59049
		59050
		65535
		65536
		65537
		99999
		100000
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
			assert(sequence.equal(sequence.append(left, right), s))
			assert(sequence.at(s, i) == list.at(xs, i))
		})
		check(sequence.append(s, sequence.wrap(-1)), list.append(xs, [-1]))
		check(sequence.append(sequence.wrap(-1), s), [-1, ..xs])
		check(sequence.append(s, s), list.append(xs, xs))
		check(s, xs)
	})
	Nil
}

numbers = (n: Int) -> List(Int) {
	{ n <= 0 } |> {
		True  -> []
		False -> list.range(0, n - 1)
	}
}

Box {
	Box(value: Int)
}

observe = (body: () -> Nil ! Trace | Assert) -> List(Int) ! Assert {
	control.boundary((b) {
		! Trace = Trace((x) {
			control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
		})
		body()
		[]
	})
}

@test
sequence_callback_types_and_order = () {
	xs = numbers(300)
	s = sequence.from_list(items: xs)
	assert(observe(() {
		mapped = sequence.map(over: s, with: (x) {
			visit(x)
			x + 1
		})
		assert(sequence.length(mapped) == 300)
	}) == xs)
	assert(observe(() {
		n = sequence.fold(over: s, from: 0, with: (a, x) {
			visit(x)
			a + 1
		})
		assert(n == 300)
	}) == xs)
	assert(observe(() { sequence.each(over: s, with: visit) }) == xs)
	boxes = sequence.map(s, (x) { Box(x) })
	check(sequence.map(boxes, (b: Box) { b.value }), xs)
	assert(
		sequence.to_list(sequence.map(sequence.from_list(["a", "é"]), (x) {
			x <> "!"
		})) ==
			["a!", "é!"]
	)
	union_error = (x: Int) -> Int ? String | Bool {
		{ x == 1 } |> {
			True  -> fail.raise(True)
			False -> fail.raise("other")
		}
	}
	assert(
		fail.catch(() { sequence.each(sequence.wrap(1), union_error) }) ==
			Error(True)
	)
	assert(
		fail.catch(() { sequence.map(sequence.wrap(2), union_error) }) ==
			Error("other")
	)
	Nil
}

// Q4 and §4.3: unrelated shapes compare by elements. Both repeated insertion
// directions exercise full digits, promoted middle nodes, and retained roots.
@test
sequence_independent_histories = () {
	xs = numbers(512)
	back = list.fold(xs, sequence.new(), (s, x) { sequence.push_back(s, x) })
	front = list.fold(list.reverse(xs), sequence.new(), (s, x) {
		sequence.push_front(s, x)
	})
	joined = sequence.append(
		sequence.from_list(list.take(xs, 211))
		sequence.from_list(list.drop(xs, 211))
	)
	assert(sequence.equal(back, front))
	assert(sequence.equal(front, joined))
	check(back, xs)
	check(front, xs)
	check(joined, xs)
	versions = list.fold(numbers(128), [(back, xs)], (saved, x) {
		[(old, model), .._] = saved
		(next, expected) = { x % 2 == 0 } |> {
			True  -> (sequence.push_front(old, -x), [-x, ..model])
			False -> (sequence.push_back(old, x), list.append(model, [x]))
		}
		check(next, expected)
		[(next, expected), ..saved]
	})
	list.each(versions, (v) {
		(s, model) = v
		check(s, model)
	})
	Nil
}

// Development checks for std-1 §4.3. The Python program below temporarily adds
// private rank/cache audits, then counts helper entries as node work. It restores
// both owned files in finally, even when a check fails. No production exports or
// costs change. Counts are deterministic operation counts, not timings.
// Run from this lane with:
// direnv exec /home/agent/Programming/i python3 -c 'from pathlib import Path; exec("\n".join(x[4:] for x in Path("std/test/sequence_test.i").read_text().splitlines() if x.startswith("//| ")))'
//| import os, re, subprocess
//| from pathlib import Path
//| src = Path('std/src/sequence.i')
//| test = Path('std/test/sequence_test.i')
//| original_src, original_test = src.read_text(), test.read_text()
//| compiler = '/home/agent/Programming/i/.rd/cache/bin/i-1df2fe0'
//| header = re.compile(r'(?m)^([a-z_]\w*(?:\([^\n]*\))? = \([\s\S]*?\) -> [^\n{]+) \{')
//| def effect(h, service):
//|     if ' ! ' in h.rsplit(') -> ', 1)[-1]:
//|         return h + ' | ' + service
//|     return h + ' ! ' + service
//| def exports(s, extra):
//|     return s.replace('\n\tSequence\n', '\n\tSequence\n\t' + extra + '\n', 1)
//| def run(filter):
//|     subprocess.run([compiler, 'test', '--project', 'std', '--opt', '0', filter], check=True)
//| audit = '''
//| audit(A) = (s: Sequence(A)) -> Nil ! Assert {
//|   Sequence(t) = s
//|   audit_tree(t, 0)
//|   Nil
//| }
//| audit_node(A) = (x: Node(A), rank: Int) -> Int ! Assert {
//|   x |> {
//|     Leaf(_) -> { assert(rank == 0) 1 }
//|     Branch2(n, a, b) -> {
//|       assert(rank > 0)
//|       actual = audit_node(a, rank - 1) + audit_node(b, rank - 1)
//|       assert(n == actual)
//|       actual
//|     }
//|     Branch3(n, a, b, c) -> {
//|       assert(rank > 0)
//|       actual = audit_node(a, rank - 1) + audit_node(b, rank - 1) + audit_node(c, rank - 1)
//|       assert(n == actual)
//|       actual
//|     }
//|   }
//| }
//| audit_digit(A) = (xs: List(Node(A)), rank: Int) -> Int ! Assert {
//|   n = list.length(xs)
//|   assert(n >= 1 && n <= 4)
//|   list.fold(xs, 0, (acc, x) { acc + audit_node(x, rank) })
//| }
//| audit_tree(A) = (t: Tree(A), rank: Int) -> Int ! Assert {
//|   t |> {
//|     EmptyTree -> 0
//|     Single(x) -> audit_node(x, rank)
//|     Deep(n, left, middle, right) -> {
//|       actual = audit_digit(left, rank) + audit_tree(middle, rank + 1) + audit_digit(right, rank)
//|       assert(n == actual)
//|       actual
//|     }
//|   }
//| }
//| '''
//| edit_args = {
//|   'new': '', 'wrap': 'item', 'from_list': 'items', 'push_front': 'to, item',
//|   'push_back': 'to, item', 'pop_front': 'from', 'pop_back': 'from',
//|   'set': 'in, index, value', 'append': 'first, second', 'split': 'sequence, at',
//|   'take': 'from, up_to', 'drop': 'from, up_to', 'reverse': 'sequence', 'map': 'over, with'
//| }
//| def audit_edit(match):
//|     h = match[1]
//|     name = h.split('(', 1)[0]
//|     h = effect(h, 'Assert')
//|     if name not in edit_args:
//|         return h + ' {'
//|     checks = 'audit(result)'
//|     if name == 'set':
//|         checks = 'result |> { None -> Nil Some(s) -> audit(s) }'
//|     elif name.startswith('pop_'):
//|         checks = 'result |> { None -> Nil Some((_, s)) -> audit(s) }'
//|     elif name == 'split':
//|         checks = '(a, b) = result\n audit(a)\n audit(b)'
//|     wrappers.append(h + ' {\n result = unchecked_' + name + '(' + edit_args[name] + ')\n ' + checks + '\n result\n}\n')
//|     return h.replace(name, 'unchecked_' + name, 1) + ' {'
//| try:
//|     wrappers = []
//|     variant = header.sub(audit_edit, original_src)
//|     src.write_text(exports(variant, 'audit') + audit + '\n'.join(wrappers))
//|     # All generated/edit results are audited, including every small/model edit.
//|     # Large drains already run in the normal suite; omit their quadratic audits.
//|     test.write_text(original_test.replace('drain(s, 0, n)', '{ n <= 257 } |> { True -> drain(s, 0, n) False -> Nil }'))
//|     print('Private invariant audit: ranks, cached leaf counts, bounded nonempty digits', flush=True)
//|     run('sequence')
//|     def meter(match):
//|         return effect(match[1], 'Meter') + ' {\n touch()'
//|     variant = header.sub(meter, original_src)
//|     # Count the unbounded local loops too, so a flattening regression is visible.
//|     variant = re.sub(r'(go = \([^\n]*\) \{)', r'\1\n touch()', variant)
//|     src.write_text(exports(variant, 'Meter(..)') + '''
//| Meter { Meter(touch: () -> Nil) }
//| touch = (meter ! Meter) -> Nil ! Meter { meter.touch() }
//| ''')
//|     test.write_text('''
//| (i/control, i/core, (Io) = i/io, sequence = std/sequence, list = std/list)
//| count = (body: () -> Nil ! sequence.Meter) -> Int {
//|   control.boundary((b) {
//|     ! sequence.Meter = sequence.Meter(() {
//|       control.suspend(b, (k) { 1 + control.resume(k, Nil) })
//|     })
//|     body()
//|     0
//|   })
//| }
//| height = (n: Int) -> Int { { n <= 1 } |> { True -> 1 False -> 1 + height(n / 2) } }
//| @test
//| sequence_node_counts = (io ! Io) {
//|   ! sequence.Meter = sequence.Meter(() { Nil })
//|   list.each([16, 64, 256, 1024, 4096, 16384, 65536, 100000], (n) {
//|     s = sequence.from_list(list.range(0, n - 1))
//|     small = sequence.from_list([1, 2, 3, 4, 5, 6, 7, 8, 9])
//|     list.each([0, n / 4, n / 2, n - 1], (i) {
//|       a = count(() { sequence.at(s, i) Nil })
//|       b = count(() { sequence.set(s, i, -1) Nil })
//|       c = count(() { sequence.split(s, i) Nil })
//|       // §4.3 requires height work, not these particular implementation constants.
//|       assert(a <= 80 * height(n))
//|       assert(b <= 80 * height(n))
//|       assert(c <= 160 * height(n))
//|       io.print(core.inspect((n, i, a, b, c)))
//|     })
//|     alias = count(() { sequence.append(s, s) Nil })
//|     left = count(() { sequence.append(s, small) Nil })
//|     right = count(() { sequence.append(small, s) Nil })
//|     assert(alias <= 160 * height(n))
//|     assert(left <= 160 * height(n))
//|     assert(right <= 160 * height(n))
//|     io.print(core.inspect((n, alias, left, right)))
//|   })
//|   Nil
//| }
//| ''')
//|     print('Counts: (length, index, at, set, split); (length, append_alias, append_short_right, append_short_left)', flush=True)
//|     run('sequence_node_counts')
//| finally:
//|     src.write_text(original_src)
//|     test.write_text(original_test)
