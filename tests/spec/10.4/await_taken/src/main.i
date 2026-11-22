(
	i/hostcall
	i/request
)

(
	main
)

main = () {
	c = request.ready(1)
	hostcall.take(c)
	hostcall.await(c)
}
