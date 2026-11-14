(
	testlib = (test) = std/test
)

S(A) = (value = A)

one = () <- (s : S(Int)) { s.value + 1 }

run = <N>(f : () <- N -> Int, x : N) { f() }

bad = () {
	_syn3_0 : S(String) = S(value = "bad")
	run(() { one() }, S(value = "hi")) <- (_syn3_0)
}

test("example", () { bad() |> (2) { Nil } | (other) { panic(other) } })
