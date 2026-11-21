(
	i/hostcall
	i/request
)

(
	main
)

main = () { assert(request.take(request.ready(1)) == 1) }
