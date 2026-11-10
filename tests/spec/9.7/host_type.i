// spec: 9.7, 10.3
// expect: compile-error E0320

// The parameters of a function of a host service are types that the host
// can take.
@host("test:kv/v1")
Kv {
	Kv(get: (keys: List(String)) -> String)
}

@test
t = () { Nil }
