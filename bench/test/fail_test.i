//// Errors through `Fail`: `fail.catch` with and without a raised error
//// (docs/performance.md). Each test runs `rounds` catches.
(
	testlib = (test) = i/test
	i/fail
)

rounds = 3_000_000

check = (ok : Bool, what : String) -> () {
	ok |> (True()) { () } & (False()) { panic(what) }
}

caught = (i : Int, acc : Int) -> Int {
	i |>
		(0) { acc }
		& (_) {
			r = fail.catch(() { fail.raise(i) }) |>
				(Ok(v)) { v }
				& (Error(e)) { e + 1 }
			caught(i - 1, acc + r)
		}
}

passed = (i : Int, acc : Int) -> Int {
	i |>
		(0) { acc }
		& (_) {
			r = fail.catch(() { i }) |> (Ok(v)) { v + 1 } & (Error(e)) { e }
			passed(i - 1, acc + r)
		}
}

total = rounds * { rounds + 1 } / 2 + rounds

test("fail_caught", () { check(caught(rounds, 0) == total, "fail_caught") })

test("fail_passed", () { check(passed(rounds, 0) == total, "fail_passed") })

// The former built-in cases now exercise the library implementation too.
builtin_caught = (i : Int, acc : Int) -> Int {
	i |>
		(0) { acc }
		& (_) {
			r = fail.catch(() { fail.raise(i) }) |>
				(Ok(v)) { v }
				& (Error(e)) { e + 1 }
			builtin_caught(i - 1, acc + r)
		}
}

builtin_passed = (i : Int, acc : Int) -> Int {
	i |>
		(0) { acc }
		& (_) {
			r = fail.catch(() { i }) |> (Ok(v)) { v + 1 } & (Error(e)) { e }
			builtin_passed(i - 1, acc + r)
		}
}

test("fail_builtin_caught", () {
	check(builtin_caught(rounds, 0) == total, "fail_builtin_caught")
})

test("fail_builtin_passed", () {
	check(builtin_passed(rounds, 0) == total, "fail_builtin_passed")
})
