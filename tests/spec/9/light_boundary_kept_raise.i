// spec: 9.2, 9.3, 9.4, 11.7
// expect: stop "control: boundary is not active"
// at: spec/main:14:32
(
	i/fail
)

// A closure that explicitly captures Fail still points at its ended catch.
// E3 keep-env forwarding leaves Fail at the call (§2.10.8).
main = () {
	r : Result((Int) -> Nil, Int) = failure.catch(() {
		control.keep((x : Int) { failure.raise(x) })
	})
	r |> (Ok(k)) { k(1) } | (Error(_)) { Nil }
}
