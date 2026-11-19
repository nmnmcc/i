Dep = ()

S(N) = (() <- N -> ())

Bad = S(Never) | S(Dep)

use = (body : () <- (Bad) -> ()) { body() }

main = () { () }
