// spec: 9.4, 9.6, 11.7
// expect: stop "control: boundary is not active"
(
	i/assertion
)

get = () <- (a: Assert) { a }

// The `Assert` of `assertion.catch` is used after `catch` has ended.
main = () {
	r = assertion.catch(() { get() })
	r |> (Ok(a)) { assert(False) <- (a) } | (Error(_)) { Nil }
}
