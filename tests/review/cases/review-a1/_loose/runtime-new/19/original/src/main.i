f = () { (20, 22) |> { t = (a, b) -> a + b } }

@main
main = () { assert(f() == 42) Nil }
