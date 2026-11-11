f = () { { go = alias = (n) -> Int { n + 1 } go(41) } }


main = () { assert(f() == 42) Nil }
