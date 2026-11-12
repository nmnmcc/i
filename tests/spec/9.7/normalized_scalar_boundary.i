// spec: 4.2, 4.4, 9.7, 10.3
// expect: pass
(
	fail = i/fail
)

// Error members are combined before checking incoming scalar sets (§10.3).
Whole = String - "error" | "error"
@host("test:kv/v1")
Kv = (
	get = (String & { String | Int }) <- (Fail(String | "error")) -> Whole,
	check = ({ Int | String } & Int) <- (
		Fail(Nil | Int - 0 | 0 | String | "error"),
	) -> Int - 0 | 0,
)

main = () <- (kv: Kv) {
	assert(kv.get("name") == "i")
	assert(kv.check(21) == 42)
	fail.catch(() { kv.get("x") }) |>
		(Error(e)) { assert(e == "no x") }
		| (Ok(_)) { panic("missing error") }
	fail.catch(() { kv.check(200) }) |>
		(Error(e: Int)) { assert(e == 200) }
		| (_) { panic("missing Int error") }
}
