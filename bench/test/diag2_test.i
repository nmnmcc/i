//// Temporary diagnostics (branch ci/perf-baseline only): where the remaining
//// list_merge, dict_insert and equality time goes. Each test isolates one
//// cost; run with `i test --project bench --opt 2 --time diag_`.
(
	testlib = (test) = std/test
	i/core
	dict = std/dict
	int = std/int
	list = std/list
)

check = (ok : Bool, what : String) -> () {
	ok |> (True) { () } & (False) { panic(what) }
}

rounds = 10_000_000

// Hashing one small Int: the per-insert cost of dict that is not allocation.
test("diag_hash_int", () {
	go = (i, acc) {
		i |>
			(0) { acc }
			& (_) { go(i - 1, acc + core.hash(i)) }
	}
	check(go(rounds, 0) != 0, "hash")
})

// A 2M-element list, to price one cons cell without any comparison.
test("diag_build_2m", () {
	xs = list.range(1, 2_000_000)
	check(list.length(xs) == 2_000_000, "length")
})

// Merging two sorted 1M lists repeatedly: per-merge cost without building
// the inputs. `merge` is the benchmark's own merge, and does not destroy its
// inputs.
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

test("diag_merge_3x", () {
	size = 1_000_000
	evens = list.map(list.range(0, size - 1), (x) { x * 2 })
	odds = list.map(list.range(0, size - 1), (x) { x * 2 + 1 })
	a = merge(evens, odds, [])
	b = merge(evens, odds, [])
	c = merge(evens, odds, [])
	check(list.length(a) == 2 * size, "a")
	check(list.length(b) == 2 * size, "b")
	check(list.length(c) == 2 * size, "c")
})

// One comparison of the two 100k tuple lists, repeated 20 times (the
// benchmark's loop), and separately the tuple contents only.
test("diag_equal_20x", () {
	xs = list.map(list.range(1, 100_000), (x) { (x, "a") })
	ys = list.map(list.range(1, 100_000), (x) { (x, "a") })
	n = list.fold(list.range(1, 20), 0, (acc, _) {
		{ xs == ys } |> (True) { acc + 1 } & (False) { acc }
	})
	check(n == 20, "equal")
})

// 200k `dict.insert`s, the benchmark's dict test.
test("diag_dict_200k", () {
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
	d = list.fold(randoms(200_000), dict.new(), (d, x) {
		dict.insert(d, x, x + 1)
	})
	check(dict.size(d) > 0, "size")
})
