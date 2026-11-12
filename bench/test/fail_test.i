//// Errors through `Fail`: `failure.catch` with and without a raised error,
//// beside the built-in `fail.catch` (docs/performance.md). Each test runs
//// `rounds` catches.

(
	testlib = (test) = std/test
	i/fail
	i/failure
)

rounds = 3_000_000

check = (ok: Bool, what: String) -> Nil {
	ok |> {
		True  -> Nil
		False -> panic(what)
	}
}

caught = (i: Int, acc: Int) -> Int {
	i |> {
		0 -> acc
		_ -> {
			r = failure.catch(() { failure.raise(i) }) |> {
				Ok(v)    -> v
				Error(e) -> e + 1
			}
			caught(i - 1, acc + r)
		}
	}
}

passed = (i: Int, acc: Int) -> Int {
	i |> {
		0 -> acc
		_ -> {
			r = failure.catch(() { i }) |> {
				Ok(v)    -> v + 1
				Error(e) -> e
			}
			passed(i - 1, acc + r)
		}
	}
}

builtin_caught = (i: Int, acc: Int) -> Int {
	i |> {
		0 -> acc
		_ -> {
			r = fail.catch(() { fail.raise(i) }) |> {
				Ok(v)    -> v
				Error(e) -> e + 1
			}
			builtin_caught(i - 1, acc + r)
		}
	}
}

builtin_passed = (i: Int, acc: Int) -> Int {
	i |> {
		0 -> acc
		_ -> {
			r = fail.catch(() { i }) |> {
				Ok(v)    -> v + 1
				Error(e) -> e
			}
			builtin_passed(i - 1, acc + r)
		}
	}
}

total = rounds * { rounds + 1 } / 2 + rounds

test("fail_caught", () { check(caught(rounds, 0) == total, "fail_caught") })

test("fail_passed", () { check(passed(rounds, 0) == total, "fail_passed") })

test("fail_builtin_caught", () { check(builtin_caught(rounds, 0) == total, "fail_builtin_caught") })

test("fail_builtin_passed", () { check(builtin_passed(rounds, 0) == total, "fail_builtin_passed") })
