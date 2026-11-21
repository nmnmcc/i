// spec: 10.1
// expect: compile-error E0327
// host-boundary §3.1: `@import("ns", "label", mode)` imports one function and
// belongs on a declaration; a service type takes `@import("ns")`.
(
	Svc(..)
	main
)

@import("app:x/v1", "ping", "now")
Svc = (ping = () -> Int)

main = () { () }
