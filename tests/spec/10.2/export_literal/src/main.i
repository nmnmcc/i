(
	out
)

// §5.8: a cofinite result is encoded as its scalar head at the host boundary.
out = (n: Int) { n |> (0) { 1 } | (m) { m } }

main = () {
	assert(out(0) == 1)
	assert(out(7) == 7)
}
