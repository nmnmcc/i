(i/core)
(f)
(a, b) = (20, 22)
f = () { a + b }

@main
main = () { assert(f() == 42) Nil }
