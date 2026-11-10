// spec: 4.2, 4.4, 9.7, 10.3
// expect: pass
(fail = i/fail)

// Error members are combined before checking incoming scalar sets (§10.3).
Whole = {String - "error"} | "error"
@host("test:kv/v1")
Kv {
	Kv(
		get: (key: String & {String | Int}) -> Whole ? String | "error"
		check: (n: {Int | String} & Int) -> {Int - 0} | 0 ? Nil | {Int - 0} | 0 | String | "error"
	)
}

@main
main = (kv ! Kv) {
	assert(kv.get("name") == "i")
	assert(kv.check(21) == 42)
	fail.catch(() { kv.get("x") }) |> {
		Error(e) -> assert(e == "no x")
		Ok(_) -> panic("missing error")
	}
	fail.catch(() { kv.check(200) }) |> {
		Error(e: Int) -> assert(e == 200)
		_ -> panic("missing Int error")
	}
}
