(
	i/hostcall
)

(
	Svc(..)
	main
)

// A "now" field: `call` needs no wait for it.
@import("app:x/v1", "now")
Svc = (ping = () -> Int)

main = () {
	s = Svc(ping = () { 7 })
	g = hostcall.group()
	hostcall.close(g)
	hostcall.call_in(g, () { s.ping() })
}
