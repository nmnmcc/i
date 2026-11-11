f = (n: Int) { 10 + case n { 0 -> 1 _ -> 2 } }


main = () { assert(f(0) == 11 && f(1) == 12) Nil }
