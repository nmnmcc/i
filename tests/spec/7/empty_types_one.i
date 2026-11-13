// spec: 7.1, 4.8
// expect: pass
// A type test of a custom type with no constructors, when it is the only
// one at its place, or tested together with the others.
(
	testlib = (test) = std/test
)

Window = Opaque

Socket = Opaque

kind = (h: Window | Int) { h |> (_: Window) { "window" } | (_) { "other" } }

both = (h: Window | Socket | Int) {
	h |> (_: Window | Socket) { "host" } | (_) { "other" }
}

test("t", () {
	(kind(1), both(2)) |> (("other", "other")) { Nil } | (_) { panic("no") }
})
