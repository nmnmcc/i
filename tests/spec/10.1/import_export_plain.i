// spec: 10.1
// expect: compile-error E0329
// host-boundary §3.1: an `@import` service type is exported as `Svc(..)`; the
// accessor and a fake need its constructor and fields.
(
	Svc
	main
)

@import("app:x/v1")
Svc = (ping = () -> Int)

main = () { () }
