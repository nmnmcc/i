// old-syntax-check: E0115
import (i/core)
(f)
f = () { 2 |> { x -> core.inspect(x) } }

@main
main = () { assert(f() == "2") Nil }
