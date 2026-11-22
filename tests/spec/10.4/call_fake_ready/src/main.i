(
	i/hostcall
	i/request
)

(
	Svc(..)
	main
)

@import("app:y/v1")
Svc = (ping = () -> Int)

main = () {
	s = Svc(ping = () { 7 })
	c = hostcall.call(() { s.ping() })
	assert(hostcall.settled(c))
	assert(hostcall.state(c) == request.Ready(7))
	assert(hostcall.take(c) == 7)
}
