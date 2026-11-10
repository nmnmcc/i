// spec: 6.12
// expect: pass
(
	i/core
)

check = (got: String, want: String) {
	{ got == want } |> {
		True  -> Nil
		False -> panic(got <> " is not " <> want)
	}
}

// The choices are tried in order: the clause is taken with the first choice
// that matches and whose guard is `True`. With `(1, 20)`, the first choice
// matches with `x = 1`, but its guard is `False`; the second gives `x = 20`.
pick = (p) {
	p |> {
		(x, _) | (_, x) ? x > 10 -> "big " <> core.inspect(x)
		_                        -> "small"
	}
}

// Each choice binds `n`; the guard is tried with each choice that matches.
tag = (v) {
	v |> {
		Ok(n) | Error(n) ? n == 1 -> "one"
		Ok(_)                     -> "ok"
		Error(_)                  -> "error"
	}
}

@test
choices = () {
	check(pick((11, 20)), "big 11")
	check(pick((1, 20)), "big 20")
	check(pick((1, 2)), "small")
	check(tag(Ok(1)), "one")
	check(tag(Error(1)), "one")
	check(tag(Ok(2)), "ok")
	check(tag(Error(2)), "error")
}
