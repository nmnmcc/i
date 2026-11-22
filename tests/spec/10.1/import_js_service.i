// spec: 10.1
// expect: compile-error E0328
// host-boundary §3.1, §4.3: the namespace `js` is kept for JS values; it is
// not a service namespace.
(
	Js(..)
	main
)

@import("js")
Js = (ping = () -> Int)

main = () { () }
