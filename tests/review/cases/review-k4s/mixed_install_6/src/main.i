(
	testlib = (test) = std/test
)

S(A) {
	S(value: A)
}

one = (s ! S(Int)) { s.value + 1 }

run(N) = (f: () -> Int ! N, x: N) { f() }

bad = () {
	! S(String) = S(value: "bad")
	run(() { one() }, S(value: "hi"))
}

test("example", () {
	bad() |> {
		2     -> Nil
		other -> panic(other)
	}
})
