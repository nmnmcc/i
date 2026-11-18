// spec: 9.2
// expect: compile-error E0523
// Decision 2 (EF03): a greatest form within Sink still needs a Sink entry.
Sink(A) = (put = (A) -> (), get = () -> Int)
read = () <- (s : Sink(Never)) { s.get() }
main = () { assert(read() == 42) }
