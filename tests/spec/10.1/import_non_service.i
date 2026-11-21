// spec: 10.1
// expect: compile-error E0327
// host-boundary §3.1: an `@import` type is one constructor whose fields have
// labels and function types; an alias of `Int` is not a service.
(
	Alias
	main
)

@import("app:x/v1")
Alias = Int

main = () { () }
