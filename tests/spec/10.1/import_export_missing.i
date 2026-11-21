// spec: 10.1
// expect: compile-error E0329
// host-boundary §3.1: an `@import` service type that the export group leaves
// out is not exported with `(..)` either.
(
	main
)

@import("app:x/v1")
Svc = (ping = () -> Int)

main = () { () }
