f = (p: (Int, Int)) { p |> { (0, y) -> y
(x, _) -> x } }

@main
main = () { assert(f((0, 42)) == 42 && f((42, 0)) == 42) Nil }
