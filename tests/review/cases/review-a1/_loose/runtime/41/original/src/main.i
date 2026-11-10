f = (n: Int) { case n { 0 -> 1 _ -> 2 } * 10 }

@main
main = () { assert(f(0) == 10 && f(1) == 20) Nil }
