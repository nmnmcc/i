// spec: 10.1
// expect: compile-error E0311
// host-boundary §3.1, §8 case 4: two services may not repeat a namespace and
// label.
(
	A(..)
	B(..)
	main
)

@import("app:x/v1")
A = (ping = () -> Int)

@import("app:x/v1")
B = (ping = () -> Int)

main = () { () }
