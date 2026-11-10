f = (n: Int) { case n { 0 -> (x) { x + 1 } _ -> (x) { x + 2 } }(40) }

@main
main = () { assert(f(0) == 41 && f(1) == 42) Nil }
