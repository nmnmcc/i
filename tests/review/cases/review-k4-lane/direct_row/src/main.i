Dep = ()

S(N) = (() <- N -> Nil)

T(N) = (() <- N -> Nil)

use = (x: T(S(Never) | S(Dep))) { x }

main = () { Nil }
