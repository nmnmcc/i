// spec: 9.7, 10.3
// expect: compile-error E0320
(
	testlib = (test) = std/test
)

Handle = Opaque

// An error type that the host can take is a union of `Nil`, `Bool`, `Int`,
// `Float`, `String`, and `BitArray`.
@host("test:kv/v1")
Kv = (get = (String) <- (Fail(Handle)) -> String)

test("t", () { Nil })
