// spec: 4.7, 9.2
// expect: compile-error E0501
// Decision 2: excluding Fail also excludes an unused Fail(Never) key.
NoError(N) = N - Fail(Never)
run = <N>(body : () <- (NoError(N)) -> Nil) { body() }
unused = <E>() <- (handler : Fail(E)) { Nil }
main = () { run(unused) }
