// spec: 9.4, 9.2
// expect: pass
(
	testlib = (test) = std/test
	i/control
	i/core
)

check = (got : String, want : String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

show = (x) { core.inspect(x) }

Name = (text = String)

name = () <- (n : Name) { n.text }

Paused = (Continuation(Nil, Paused, Never, Name))
Done = (String)
Paused = Paused | Done

leaf = (b) {
	control.suspend(b, (k) { Paused(k) })
	name()
}

// Two environment statements: the later one is seen.
twice = (b) {
	{
		x = leaf(b)
		x <> "/" <> name()
	} <- (Name(text = "a"), Name(text = "twice"))
}

one = (b) {
	{
		x = twice(b)
		x <> "/" <> name()
	} <- (Name(text = "one"))
}

two = (b) {
	{
		x = one(b)
		x <> "/" <> name()
	} <- (Name(text = "two"))
}

plain = (b) {
	x = two(b)
	x <> "/" <> name()
}

// §9.4 rule 3: the frames that wait keep their environment statements, on
// top of the environment of the place of `resume`.
test("environment_of_frames_that_wait", () {
	{
		p = control.boundary((b) { Done(plain(b)) })
		k = p |> (Paused(k)) { k } | (Done(_)) { panic("not paused") }
		{ control.resume(k, Nil) } |>
			(Done(s)) { check(s, "twice/twice/one/two/resumer") }
			| (Paused(_)) { panic("paused again") } <- (Name(text = "resumer"))
	} <- (Name(text = "start"))
})

// Each resume of a computation gives it the environment of its own place.
test("two_places", () {
	{
		p = control.boundary((b) {
			x = leaf(b)
			y = leaf(b)
			Done(x <> "," <> y)
		})
		k = p |> (Paused(k)) { k } | (Done(_)) { panic("not paused") }
		{
			k2 = { control.resume(k, Nil) } |>
				(Paused(k2)) { k2 }
				| (Done(_)) { panic("not paused again") }
			{ control.resume(k2, Nil) } |>
				(Done(s)) { check(s, "first,second") }
				| (Paused(_)) { panic("paused again") } <- (
				Name(text = "second"),
			)
		} <- (Name(text = "first"))
	} <- (Name(text = "start"))
})
