// §10.2: a test gives `Nil`, and its environment has only host services
// and `Assert`.
import ((Clock, Storage) = app/main)

@test
passes = () { assert(1 == 1) }

@test
host = (s ! Storage) { assert(s.read("k") == "v") }

@test
gives_int = () { 1 }

@test
needs_clock = (c ! Clock) { assert(c.now() == 1) }

@test
never = () { panic("no") }
