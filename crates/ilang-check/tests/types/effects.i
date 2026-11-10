(
	i/core
	i/fail
)

// §9.2: environment parameters and environment statements (rule 9).
Clock {
	Clock(now: () -> Int)
}

Log {
	Log(write: (line: String) -> Nil)
}

elapsed = (start: Int, clock ! Clock) { clock.now() - start }

fixed = () {
	! Clock = Clock(now: () { 42 })
	elapsed(40)
}

logged = (log ! Log) {
	log.write("start")
	elapsed(1)
}

// Only Clock is given; Log is still needed.
half = () {
	! Clock = Clock(now: () { 1 })
	logged()
}

// A service may be any value of its type.
given = (c: Clock) {
	! Clock = c
	elapsed(0)
}

// §9.3: errors come together by union.
Http {
	Http(get: (url: String) -> String ? Int)
}

fetch = (url, http ! Http) { http.get(url) }

two = () {
	a = fetch("a")
	b = core.byte_slice(a, start: 0, length: 1) |> fail.from_result
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
