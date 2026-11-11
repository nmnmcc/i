// §10.2 and §10.3: the checks of entry points that need inferred types.
(
	i/fail
)

(
	add
	opaque
	errors
	never
	never_param
	bad_param
	bad_value
	generic
	bad_errors
	twice_error
	with_host
	with_clock
	number
	Handle
	Storage(..)
	Clock(..)
)

@host("app:storage/v1")
Storage {
	Storage(read: (key: String) -> String)
}

Clock {
	Clock(now: () -> Int)
}

Handle {}

// Host exports: parameters, value, and errors that the host can take.
add = (a: Int, b: Int) { a + b }

opaque = (h: Handle, flag: Bool) {
	flag |> {
		True  -> h
		False -> h
	}
}

errors = (x: Int) -> Nil ^ Int | String {
	x |> {
		0 -> fail.raise(1)
		1 -> fail.raise("one")
		_ -> Nil
	}
}

// `Never` is not a type that the host can take (§10.3).
never = (s: String) { panic(s) }

never_param = (x: Never) -> Int { x + 1 }

bad_param = (xs: List(Int)) { 1 }

bad_value = () { (1, 2) }

generic = (x) { x }

bad_errors = () -> Int ^ (Int, Int) { fail.raise((1, 2)) }

twice_error = () -> Int ^ Handle { 1 }

// A host export is a function.
number = 1

// The environment of an entry point: host services and `Assert` only.
with_host = (s ! Storage) {
	assert(s.read("k") == "v")
	1
}

with_clock = (c ! Clock) { c.now() }


main = (s ! Storage, c ! Clock) {
	s.read(c.now() |> core_text)
	Nil
}

core_text = (n: Int) { "x" }
