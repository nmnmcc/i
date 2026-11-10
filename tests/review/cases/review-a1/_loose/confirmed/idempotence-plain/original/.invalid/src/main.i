// old-syntax-check: E0101
f = () { case (x) -> Int { x }(42) { _ -> 42 } * 10 }

@main
main = () { assert(f() == 420) Nil }
