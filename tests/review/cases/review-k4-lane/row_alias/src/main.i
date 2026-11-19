Dep = ()

S(N) = (() <- N -> ())

T(N) = (() <- N -> ())

Bad = T(S(Never) | S(Dep))

use = (x : Bad) { x }

main = () { () }
