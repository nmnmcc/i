Dep = ()

S(N) = (() <- N -> ())

T(N) = (() <- N -> ())

use = (x : T(S(Never) | S(Dep))) { x }

main = () { () }
