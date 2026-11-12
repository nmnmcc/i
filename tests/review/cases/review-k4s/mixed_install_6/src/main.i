(
	testlib = (test) = std/test
)

S(A) {
	S(value: A)
}

one = (s ! S(Int)) { s.value + 1 }

run(N) = (f: () -> Int ! N, x: N) { f() }

bad = () {
	! S(String) = S("bad")
	run(() { one() }, S("hi"))
}

test("example", () {
	bad() |> {
		2     -> Nil
		other -> panic(other)
	}
})
