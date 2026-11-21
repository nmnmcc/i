(
	i/hostcall
	i/request
)

(
	main
)

main = () {
	c = request.ready(1)
	hostcall.cancel(c)
	hostcall.cancel(c)
	assert(hostcall.settled(c))
}
