// §10.2: a test gives `Nil`, and its environment has only host services
// and `Assert`.
(
	(Clock, Storage) = app/main
)

passes = () { assert(1 == 1) }

host = () <- (s: Storage) { assert(s.read("k") == "v") }

gives_int = () { 1 }

needs_clock = () <- (c: Clock) { assert(c.now() == 1) }

never = () { panic("no") }
