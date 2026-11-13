// spec: 9.3, 9.4, 11.7
// expect: stop "control: boundary is not active"
// at: spec/main:14:17
(
	i/failure
)

// A `Fail` that outlives its `catch` stops the program when it is used
// (syntax-3 §2.10.6): the boundary of the catch is not active.
the_fail = () <- (f: Fail(Int)) { f }

main = () {
	r = failure.catch(() { the_fail() })
	r |> (Ok(f)) { f.raise(1) } | (Error(_)) { Nil }
}
