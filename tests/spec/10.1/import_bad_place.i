// spec: 10.1
// expect: compile-error E0327
// host-boundary §3.1: `@import` is on a service type or a declaration
// `name: Type`; a value definition is neither.
(
	main
)

@import("app:x/v1")
value = 1

main = () { () }
