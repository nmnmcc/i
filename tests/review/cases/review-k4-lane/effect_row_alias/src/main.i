Dep = ()

S(N) = (() <- N -> Nil)

Bad = S(Never) | S(Dep)

use = (body : () <- (Bad) -> Nil) { body() }

main = () { Nil }
