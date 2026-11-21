// spec: 10.1
// expect: compile-error E0306
// host-boundary §3.1: `ilang:` namespaces belong to `@runtime` in the library.
(
	Core(..)
	main
)

@import("ilang:core")
Core = (ping = () -> Int)

main = () { () }
