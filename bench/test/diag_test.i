//// Temporary diagnostics for the performance lane (branch ci/perf-baseline
//// only, never merged). They isolate costs that bench_test.i mixes: the
//// dispatch cost of a comparator held in a function value, a direct known
//// call, and a capturing closure. Each loop does 10M comparisons and checks
//// its sum, so nothing can be removed.
(
	testlib = (test) = std/test
	i/core
	int = std/int
	list = std/list
	order = std/order
)

check = (ok : Bool, what : String) -> () {
	ok |> (True) { () } & (False) { panic(what) }
}

rounds = 10_000_000

test("diag_loop", () {
	go = (i, acc) {
		i |>
			(0) { acc }
			& (_) { go(i - 1, acc) }
	}
	check(go(rounds, 0) == 0, "loop")
})

test("diag_call_indirect", () {
	// The function value comes out of a list, so the emitter cannot know it.
	fns = list.map(list.range(1, 2), (_) { int.compare })
	f = list.fold(fns, int.compare, (last, next) { next })
	go = (i, acc) {
		i |>
			(0) { acc }
			& (_) {
				r = f(i, i + 1)
				go(i - 1, acc + order.to_int(r))
			}
	}
	check(go(rounds, 0) == rounds, "sum")
})

test("diag_call_direct", () {
	go = (i, acc) {
		i |>
			(0) { acc }
			& (_) {
				r = int.compare(i, i + 1)
				go(i - 1, acc + order.to_int(r))
			}
	}
	check(go(rounds, 0) == rounds, "sum")
})

test("diag_call_closure", () {
	// A capturing closure: the comparison goes through the closure's captured
	// values, not only through the code reference.
	make = (k : Int) {
		(a : Int, b : Int) { int.compare(a + k, b + k) }
	}
	f = make(0)
	go = (i, acc) {
		i |>
			(0) { acc }
			& (_) {
				r = f(i, i + 1)
				go(i - 1, acc + order.to_int(r))
			}
	}
	check(go(rounds, 0) == rounds, "sum")
})
