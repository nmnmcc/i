// old-syntax-check: E0105
g = (p: (Int, Int)) { case p { t = (a, b) -> (x) -> Int { a + b + x } } }
f = () { g((20, 21))(1) }

@main
main = () { assert(f() == 42) Nil }
