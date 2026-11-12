(
	testlib = (test) = std/test
)

S(A) = (value = A)

one = () <- (s: S(Int)) { s.value + 1 }

run = <N>(f: () <- N -> Int, x: N) { f() }

bad = () { run(() { one() }, S(value = "hi")) <- (S(value = 1)) }

test("example", () { bad() |> (2) { Nil } | (other) { panic(other) } })
