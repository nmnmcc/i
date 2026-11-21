// spec: 10.1
// expect: compile-error E0326
// host-boundary §3.1, App. D: `@import` takes string literals by position.
(
	main
)

@import()
ping : () -> Int

main = () { () }
