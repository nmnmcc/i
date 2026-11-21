// spec: 10.1
// expect: compile-error E0326
// host-boundary §3.1: the mode of a service is `"now"` or the default
// `"later"`; any other second argument is the wrong form.
(
	Svc(..)
	main
)

@import("app:x/v1", "soon")
Svc = (ping = () -> Int)

main = () { () }
