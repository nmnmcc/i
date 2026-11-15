(
	i/fail
	clock = (Clock) = time/clock
)

(
	remaining
	Late(..)
)
Late = ()
remaining = (deadline) {
	left = deadline - clock.now()
	left |> (n ? n < 0) { fail.raise(Late) } | (n) { n }
}
