// spec: 9.7, 10.3
// expect: compile-error E0320

Handle {}

// An error type that the host can take is a union of `Nil`, `Bool`, `Int`,
// `Float`, `String`, and `BitArray`.
@host("test:kv/v1")
Kv {
	Kv(get: (key: String) -> String ^ Handle)
}

@test
t = () { Nil }
