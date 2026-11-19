// spec: 9.3, 9.5, 11.7
// expect: stop "panic: boom"
(
	i/fail
)

// A stop is not an error: `fail.catch` does not take it, so it does not
// give `Error(…)` and the program stops.
main = () {
	r : Result(Int, String) = fail.catch(() { panic("boom") })
	r |> (Ok(_)) { () } & (Error(_)) { () }
}
