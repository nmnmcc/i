(
	i/hostcall
	i/core
)

(
	Async(..)
	main
)

@import("test:async/v1")
Async = (echo = (String) -> String)

main = () {
	a = make_async()
	c = hostcall.call(() { a.echo("hi") })
	assert(core.not(hostcall.settled(c)))
	assert(hostcall.await(c) == "hi")
}
