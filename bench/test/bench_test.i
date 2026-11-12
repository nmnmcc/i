//// Benchmarks. `scripts/bench` runs every test here with `i test --time`
//// at `--opt 2` and prints how long each one takes. Each one checks its
//// result, so that no work can be left out.

(
	testlib = (test) = std/test
	i/core
	dict = std/dict
	int = std/int
	list = std/list
	option = (Some) = std/option
)

size = 1_000_000

// Numbers that look random: x * 75 + 74 modulo 65537. Every step fits in
// a small Int.
randoms = (n: Int) -> List(Int) {
	go = (i, x, acc) {
		i |> {
			0 -> acc
			_ -> {
				y = x * 75 + 74
				go(i - 1, y % 65537, [x, ..acc])
			}
		}
	}
	go(n, 1, [])
}

check = (ok: Bool, what: String) -> Nil {
	ok |> {
		True  -> Nil
		False -> panic(what)
	}
}

test("list_build", () {
	xs = list.range(1, size)
	check(list.length(xs) == size, "length")
})

test("list_map", () {
	xs = list.map(list.range(1, size), with: (x) { x + 1 })
	check(list.first(xs) == Some(2), "first")
})

test("list_reverse", () {
	xs = list.range(1, size)
	ys = list.reverse(
		list.reverse(list.reverse(list.reverse(list.reverse(xs))))
	)
	check(list.first(ys) == Some(size), "first")
})

test("list_sort", () {
	xs = list.sort(randoms(size), by: int.compare)
	check(list.length(xs) == size, "length")
})

// A merge of two sorted lists, written out.
merge = (xs: List(Int), ys: List(Int), acc: List(Int)) -> List(Int) {
	xs |> {
		[] -> list.append(list.reverse(acc), ys)
		[x, ..xs2] -> ys |> {
			[] -> list.append(list.reverse(acc), xs)
			[y, ..ys2] -> { x > y } |> {
				True  -> merge(xs, ys2, [y, ..acc])
				False -> merge(xs2, ys, [x, ..acc])
			}
		}
	}
}

test("list_merge", () {
	evens = list.map(list.range(0, size - 1), with: (x) { x * 2 })
	odds = list.map(list.range(0, size - 1), with: (x) { x * 2 + 1 })
	xs = merge(evens, odds, [])
	check(list.length(xs) == 2 * size, "length")
})

test("closures", () {
	adders = list.map(list.range(1, 100), with: (k) { (x) { x + k } })
	total = list.fold(list.range(1, 10_000), from: 0, with: (acc, x) {
		list.fold(adders, from: acc, with: (a, f) {
			y = f(x) + a
			y % 1_000_000
		})
	})
	check(total >= 0, "total")
})

inspect = () {
	text = core.inspect(list.map(list.range(1, 200_000), with: (x) {
		(x, [x, x + 1])
	}))
	check(core.byte_size(text) > 0, "text")
}

test("inspect", () { inspect() })

test("dict_insert", () {
	d = list.fold(randoms(200_000), from: dict.new(), with: (d, x) {
		dict.insert(d, x, x + 1)
	})
	check(dict.size(d) > 0, "size")
})

test("equality", () {
	xs = list.map(list.range(1, 100_000), with: (x) { (x, "a") })
	ys = list.map(list.range(1, 100_000), with: (x) { (x, "a") })
	n = list.fold(list.range(1, 20), from: 0, with: (acc, _) {
		{ xs == ys } |> {
			True  -> acc + 1
			False -> acc
		}
	})
	check(n == 20, "equal")
})
