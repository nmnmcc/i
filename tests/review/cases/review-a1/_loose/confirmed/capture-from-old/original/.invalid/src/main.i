// old-syntax-check: E0101,E0101
import (g = app/types)
f = () { case (x) -> Int { case g.Box(x) { g.Box(v) -> v } }(42) { _ -> 1 } * 10 }

@main
main = () { assert(f() == 10) Nil }
