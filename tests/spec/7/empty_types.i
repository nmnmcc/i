// spec: 7.1, 4.8
// expect: compile-error E0501
// Custom types with no constructors all count as one type constructor for
// a type test, so a test of `Window` would also take a `Socket`.
(
	testlib = (test) = std/test
)

Window = Opaque

Socket = Opaque

kind = (h: Window | Socket) {
	h |> (_: Window) { "window" } | (_: Socket) { "socket" }
}

test("t", () { Nil })
