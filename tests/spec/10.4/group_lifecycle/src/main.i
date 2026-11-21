(
	i/hostcall
	i/core
)

(
	main
)

main = () {
	g = hostcall.group()
	assert(core.not(hostcall.closed(g)))
	hostcall.close(g)
	assert(hostcall.closed(g))
	hostcall.close(g)
	assert(hostcall.closed(g))
}
