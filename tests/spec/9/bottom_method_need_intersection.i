// spec: 4.4, 9.2, 11.7
// expect: stop "bottom"
// Decision 1: Read(Never) is inhabited by a stopping function.
Read(A) = (read = () -> A)
f = () <- (r : Read(Int)) { r.read() }
g = () <- (r : Read(String)) { r.read() }
both = () { f() g() }
main = () { both() <- (Read(read = () { panic("bottom") })) }
