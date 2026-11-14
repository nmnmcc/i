(
	i/core
	i/fail
)

// §9.2: environment parameters and environment statements (rule 9).
Clock = (now = () -> Int)

Log = (write = (String) -> Nil)

elapsed = (start : Int) <- (clock : Clock) { clock.now() - start }

fixed = () { elapsed(40) <- (Clock(now = () { 42 })) }

logged = () <- (log : Log) {
	log.write("start")
	elapsed(1)
}

// Only Clock is given; Log is still needed.
half = () { logged() <- (Clock(now = () { 1 })) }

// A service may be any value of its type.
given = (c : Clock) { elapsed(0) <- (c) }

// §9.3: errors come together by union.
Http = (get = (String) <- (Fail(Int)) -> String)

fetch = (url) <- (http : Http) { http.get(url) }

two = () {
	a = fetch("a")
	b = core.byte_slice(a, 0, 1) |> fail.from_result
	(a, b)
}

// §6.10: a binding whose pattern may fail needs Assert.
first = (r) {
	Ok(v) = r
	v
}

pair = (t) {
	(a, b) = t
	a
}

// A parameter that may fail puts Assert in the environment (rule 4).
head = ([x, ..]) { x }

unwrap = (Ok(v)) -> Int { v }

// Stage 3 keeps D4 (syntax-3.md §2.12, "as today with the multi-constructor
// `Result`"): `Ok(v)` is fitted to a `Result`, so with a written type that
// has no needs, the parameter that may fail is an error, as before.
closed: (Result(Int, String)) -> Int = (Ok(v)) { v }
