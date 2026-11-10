// old-syntax-check: E0101
f = () { { (x) -> Int { x + 1 }(41) } }

@main
main = () { assert(f() == 42) Nil }
