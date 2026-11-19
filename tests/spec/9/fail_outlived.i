// spec: 9.3, 9.4
// expect: stop "control: boundary is not active"
(
	i/fail
)

// A `Fail` returned out of its `catch` stops the program when an error is
// raised through it later: the boundary of the catch has ended.
take_fail = () <- (f : Fail(String)) { f }

main = () {
	r = fail.catch(() { take_fail() })
	r |> (Ok(f)) { f.raise("late") } & (Error(_)) { () }
}
