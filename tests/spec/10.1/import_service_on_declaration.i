// spec: 10.1
// expect: compile-error E0327
// host-boundary §3.1: `@import("ns")` alone is the service form; a
// declaration names its label and mode.
(
	main
)

@import("app:x/v1")
ping : () -> Int

main = () { () }
