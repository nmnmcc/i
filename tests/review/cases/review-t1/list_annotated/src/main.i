f = (xs: List("a" | "b")) { xs |> { [] -> 0  ["a",..] -> 1  ["b",..] -> 2 } }

main = () { assert(f(["b","a"])==2) Nil }
