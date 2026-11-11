f = () { (20, 22) |> { t = (a, b) -> a + b } }


main = () { assert(f() == 42) Nil }
