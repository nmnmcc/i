// spec: 9.1, 9.3
// expect: pass
(
	testlib = (test) = std/test
	i/core
	i/fail
)

check = (got : String, want : String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

show = (x) { core.inspect(x) }

test("raise_and_catch", () {
	check(
		show(fail.catch(() { 1 })),
		"Ok(1)",
	)
	check(
		show(fail.catch(() { fail.raise("e") })),
		"Error(\"e\")",
	)
	check(
		show(fail.catch(() { 1 + fail.raise(2) })),
		"Error(2)",
	)
	// The error goes out through the calls until it is taken.
	inner = (n) { n |> (0) { fail.raise("bottom") } | (_) { n + inner(n - 1) } }
	check(
		show(fail.catch(() { inner(100) })),
		"Error(\"bottom\")",
	)
	// Nested: the inner `catch` takes the error.
	r = fail.catch(() {
		a = fail.catch(() { fail.raise(1) })
		(a, 2)
	})
	check(show(r), "Ok((Error(1), 2))")
	// Raised again from a handler.
	again = fail.catch(() {
		fail.catch(() { fail.raise("x") }) |>
			(Ok(_)) { "no" }
			| (Error(e)) { fail.raise(e <> "y") }
	})
	check(show(again), "Error(\"xy\")")
})

from_result = () {
	check(
		show(fail.catch(() { fail.from_result(Ok(3)) })),
		"Ok(3)",
	)
	check(
		show(fail.catch(() { fail.from_result(Error("no")) })),
		"Error(\"no\")",
	)
}

test("from_result", () { from_result() })

map_error = () {
	m = fail.catch(() { fail.map_error(() { fail.raise(2) }, (e) { e * 10 }) })
	check(show(m), "Error(20)")
	ok = fail.catch(() { fail.map_error(() { 5 }, (e) { e }) })
	check(show(ok), "Ok(5)")
}

test("map_error", () { map_error() })

// §9.1: retrying is a normal function.
retry = <A, N>(times : Int, body : () <- N -> A) -> A {
	fail.catch(body) |>
		(Ok(value)) { value }
		| (Error(error)) {
			{ times > 1 } |>
				(True) { retry(times - 1, body) }
				| (False) { fail.raise(error) }
		}
}

test("retrying", () {
	check(
		show(fail.catch(() { retry(3, () { fail.raise("down") }) })),
		"Error(\"down\")",
	)
	check(
		show(fail.catch(() { retry(3, () { "up" }) })),
		"Ok(\"up\")",
	)
})

// The example of §9.3, with `http_get` and `parse_user` given by a service.
User = (name = String)

HttpError = (status = Int)

ParseError = (at = Int)

Http = (get = (String) <- (Fail(HttpError)) -> String)

default_user = User(name = "default")

http_get = (id : String) <- (http : Http) { http.get(id) }

parse_user = (text : String) {
	text |>
		("user " <> name) { User(name = name) }
		| (_) { fail.raise(ParseError(at = 0)) }
}

load = (id) { parse_user(http_get(id)) }

safe = (id) {
	fail.catch(() { load(id) }) |>
		(Ok(user)) { user }
		| (Error(_ : ParseError)) { default_user }
		| (Error(other)) { fail.raise(other) }
}

fake = Http(
	get = (id) {
		id |>
			("1") { "user ann" }
			| ("2") { "garbage" }
			| (_) { fail.raise(HttpError(status = 404)) }
	},
)

test("safe_example", () {
	{
		check(
			show(fail.catch(() { safe("1") })),
			"Ok(User(\"ann\"))",
		)
		check(
			show(fail.catch(() { safe("2") })),
			"Ok(User(\"default\"))",
		)
		check(
			show(fail.catch(() { safe("3") })),
			"Error(HttpError(404))",
		)
	} <- (fake)
})

// A deep error goes out through many frames, and the frames are gone.
test("deep_unwinding", () {
	down = (n) { n |> (0) { fail.raise(n) } | (_) { 1 + down(n - 1) } }
	each = (k) {
		k |>
			(0) { Nil }
			| (_) {
				check(
					show(fail.catch(() { down(1000) })),
					"Error(0)",
				)
				each(k - 1)
			}
	}
	each(1000)
})
