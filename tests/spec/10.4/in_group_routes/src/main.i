(
	i/hostcall
	i/request
)

(
	Later(..)
	main
)

@host("test:async/v1", "request")
Later = (echo = (String) -> Request(String))

// A wrapper that starts without naming a group.
start = (text : String) <- (later : Later) { later.echo(text) }

main = () {
	g = hostcall.group()
	inside = hostcall.in_group(g, () { start("in") })
	outside = start("out")
	hostcall.close(g)
	assert(request.state(inside) == request.Cancelled())
	assert(request.await(outside) == "out")
}
