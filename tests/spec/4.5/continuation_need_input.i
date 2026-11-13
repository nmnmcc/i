// spec: 4.4, 4.5, 9.4
// expect: compile-error E0501

// EF02: a wrapper must retain the continuation's provider direction.
Read(A) = (read = () -> A)
Paused(A) = (k = Continuation(Nil, Int, Read(A)))
up = (x: Paused(Int)) -> Paused(Int | String) { x }
main = () { Nil }
