(
	assertion = i/assertion
)
f = (0 | 1) { "yes" }

main = () {
	assert(f(0) == "yes")
	assertion.catch(() { f(2) }) |>
		(Error(_)) { Nil }
		| (Ok(_)) { panic("missed Assert") }
}
