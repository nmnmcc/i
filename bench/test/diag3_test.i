//// Temporary diagnostics (branch ci/perf-baseline only): the components of
//// list_merge and dict_insert. Run with `i test --project bench --opt 2 --time diag3_`.
(
	testlib = (test) = std/test
	i/core
	dict = std/dict
	int = std/int
	list = std/list
	option = (Some) = std/option
)

check = (ok : Bool, what : String) -> () {
	ok |> (True) { () } & (False) { panic(what) }
}

size = 1_000_000

randoms = (n : Int) -> List(Int) {
	go = (i, x, acc) {
		i |>
			(0) { acc }
			& (_) {
				y = x * 75 + 74
				go(i - 1, y % 65537, [x, ..acc])
			}
	}
	go(n, 1, [])
}

// The two input lists of the merge benchmark, with nothing else.
test("diag3_merge_inputs", () {
	evens = list.map(list.range(0, size - 1), (x) { x * 2 })
	odds = list.map(list.range(0, size - 1), (x) { x * 2 + 1 })
	check(list.length(evens) == size, "evens")
	check(list.length(odds) == size, "odds")
})

// Walk the merge order, comparing heads, but build nothing.
count = (xs : List(Int), ys : List(Int), n : Int) -> Int {
	xs |>
		([]) { n + list.length(ys) }
		& ([x, ..xs2]) {
			ys |>
				([]) { n + list.length(xs) }
				& ([y, ..ys2]) {
					{ x > y } |>
						(True) { count(xs, ys2, n + 1) }
						& (False) { count(xs2, ys, n + 1) }
				}
		}
}

test("diag3_merge_count", () {
	evens = list.map(list.range(0, size - 1), (x) { x * 2 })
	odds = list.map(list.range(0, size - 1), (x) { x * 2 + 1 })
	check(count(evens, odds, 0) == 2 * size, "count")
})

// Build the accumulator cells but skip the final reverse and append.
merge_no_finish = (xs : List(Int), ys : List(Int), acc : List(Int), n : Int) -> Int {
	xs |>
		([]) { n + list.length(ys) }
		& ([x, ..xs2]) {
			ys |>
				([]) { n + list.length(xs) }
				& ([y, ..ys2]) {
					{ x > y } |>
						(True) { merge_no_finish(xs, ys2, [y, ..acc], n + 1) }
						& (False) { merge_no_finish(xs2, ys, [x, ..acc], n + 1) }
				}
		}
}

test("diag3_merge_acc", () {
	evens = list.map(list.range(0, size - 1), (x) { x * 2 })
	odds = list.map(list.range(0, size - 1), (x) { x * 2 + 1 })
	check(merge_no_finish(evens, odds, [], 0) == 2 * size, "acc")
})

// The benchmark's merge, for the same tests to subtract.
merge = (xs : List(Int), ys : List(Int), acc : List(Int)) -> List(Int) {
	xs |>
		([]) { list.append(list.reverse(acc), ys) }
		& ([x, ..xs2]) {
			ys |>
				([]) { list.append(list.reverse(acc), xs) }
				& ([y, ..ys2]) {
					{ x > y } |>
						(True) { merge(xs, ys2, [y, ..acc]) }
						& (False) { merge(xs2, ys, [x, ..acc]) }
				}
		}
}

test("diag3_merge_full", () {
	evens = list.map(list.range(0, size - 1), (x) { x * 2 })
	odds = list.map(list.range(0, size - 1), (x) { x * 2 + 1 })
	check(list.length(merge(evens, odds, [])) == 2 * size, "merge")
})

// 200k dict inserts, the benchmark's workload, for subtraction.
test("diag3_dict_build", () {
	d = list.fold(randoms(200_000), dict.new(), (d, x) {
		dict.insert(d, x, x + 1)
	})
	check(dict.size(d) > 0, "size")
})

// The same inserts and then 200k lookups of the same keys.
test("diag3_dict_build_get", () {
	d = list.fold(randoms(200_000), dict.new(), (d, x) {
		dict.insert(d, x, x + 1)
	})
	n = list.fold(randoms(200_000), 0, (n, x) {
		{ dict.get(d, x) } |> (Some(_)) { n + 1 } & (_) { n }
	})
	check(n == 200_000, "found")
})

// Re-inserting the same 200k keys: path rebuild without growing the map.
test("diag3_dict_overwrite", () {
	xs = randoms(200_000)
	d = list.fold(xs, dict.new(), (d, x) { dict.insert(d, x, x + 1) })
	d2 = list.fold(xs, d, (d, x) { dict.insert(d, x, x + 2) })
	check(dict.size(d2) == dict.size(d), "size")
})
