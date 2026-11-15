(
	i/fail
	(Clock, now) = time/clock
)

(
	remaining
	Late(..)
)

Late = ()
remaining = (due) {
	due - now() |> (n ? n < 0) { fail.raise(Late) } | (n) { n }
}
