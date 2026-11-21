// spec: 10.4
// expect: compile-error E0323
// host-boundary §5.1, §5.4 row 1: `call` is reached only through
// `i/hostcall`, which needs the key.
(
	i/hostcall
)

(
	Svc(..)
	main
)

@import("app:x/v1")
Svc = (ping = () -> Int)

main = () {
	s = Svc(ping = () { 1 })
	hostcall.take(hostcall.call(() { s.ping() }))
}
