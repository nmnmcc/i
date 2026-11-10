// §4.8 rule 12: a catch-all gets `B - H`, where `H` has the type
// constructors that the clauses before it take there.
(
	i/fail
)

// §9.3, with `http_get` and `parse_user` declared with written types.
Http {
	Http(get: (url: String) -> String ? HttpError)
}

HttpError {
	HttpError(code: Int)
}

ParseError {
	ParseError(message: String)
}

User {
	User(name: String)
}

http_get = (url: String, http ! Http) -> String ? HttpError ! Http {
	http.get(url)
}

parse_user = (text: String) -> User ? ParseError {
	text |> {
		""   -> fail.raise(ParseError("empty"))
		name -> User(name)
	}
}

default_user = User("nobody")

load = (id) { parse_user(http_get(id)) }

// inferred: (id: String) -> User ? HttpError | ParseError ! Http

safe = (id) {
	fail.catch(() { load(id) }) |> {
		Ok(user)             -> user
		Error(_: ParseError) -> default_user
		Error(other)         -> fail.raise(other) // other: HttpError
	}
}

// inferred: (id: String) -> User ? HttpError ! Http

// At the top place.
rest = (v: Int | String | Nil) {
	v |> {
		_: Int -> "int"
		Nil    -> "nil"
		other  -> other
	}
}

// Every constructor of `Result` before: `Result` is taken.
whole = (v: Result(Int, Int) | String) {
	v |> {
		Ok(_)    -> "ok"
		Error(_) -> "error"
		s        -> s
	}
}

// At a place below a constructor.
nested = (r: Result(Int | String, Nil)) {
	r |> {
		Ok(_: Int) -> ""
		Ok(s)      -> s
		Error(_)   -> "error"
	}
}

// At a place below a tuple, with a part that must match too.
pairs = (p: (Int | String, Bool)) {
	p |> {
		(_: Int, _) -> ""
		(s, True)   -> s
		(_, False)  -> ""
	}
}

// Two levels down: every `Result` below `Ok` is taken, but rule 12 takes
// out whole type constructors only, so after `Ok(Ok(_))` alone `inner`
// would still be a `Result`.
deep = (r: Result(Result(Int, String) | Nil, Nil)) {
	r |> {
		Ok(Ok(_))    -> "ok"
		Ok(Error(s)) -> s
		Ok(inner)    -> inner
		Error(_)     -> "error"
	}
}

// Not taken: a guard, a literal, or a part that does not match every
// value.
guarded = (v: Int | String) {
	v |> {
		_: Int ? True -> "int"
		x             -> x
	}
}

literal = (v: Int | String) {
	v |> {
		1 -> "one"
		x -> x
	}
}

partial = (v: Result(Int, String) | Nil) {
	v |> {
		Ok(1)    -> Nil
		Error(_) -> Nil
		other    -> other
	}
}

// Rule 12 looks at what the patterns match, not at the types: the first
// part of `(True, _: Int)` and `(False, _: Int)` is not a catch-all, so
// `Int` is not taken at the second part.
by_patterns = (p: (Bool, Int | String)) {
	p |> {
		(True, _: Int)  -> "t"
		(False, _: Int) -> "f"
		(_, s)          -> s
	}
}

// With no annotation, the type of the subject comes from the patterns.
open = (v) {
	v |> {
		_: Int -> 0
		other  -> other
	}
}
