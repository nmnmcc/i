(
	i/hostcall
)

(
	main
)

main = () {
	r = hostcall.open_root()
	g = hostcall.group_at(r)
	hostcall.close_root(r)
	assert(hostcall.closed(g))
}
