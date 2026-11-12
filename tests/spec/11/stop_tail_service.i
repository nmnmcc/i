// spec: 11.7, 9.6
// expect: stop "assertion failed: mine"
// at: spec/main:12:12
// at: spec/main:16:19
// at: spec/main:20:2

// A tail call of a function of a service keeps its position as the first
// one; the tail call of `check` is gone.
check = (c: Bool) <- (a: Assert) {
	c |> (True) { Nil } | (False) { a.fail(AssertionFailure(message = "mine")) }
}

apply = (f, x) { [f(x)] }

main = () {
	apply((v) { check(v == 0) }, 1)
	Nil
}
