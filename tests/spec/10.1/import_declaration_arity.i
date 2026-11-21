// spec: 10.1
// expect: compile-error E0326
// host-boundary §3.1: a declaration is `@import("ns", "label", mode)`; the
// mode is written.
(
	main
)

@import("app:x/v1", "ping")
ping : () -> Int

main = () { () }
