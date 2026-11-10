(
	(RemoteSize) = spec/main
)

@test
closed_read = () {
	f = (n: RemoteSize) { n }
	assert(f(3) == 3)
}
