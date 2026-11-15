Dep = ()

S(N) = (() <- N -> Nil)

T(N) = (() <- N -> Nil)

Bad = T(S(Never) | S(Dep))

use = (x : Bad) { x }

main = () { Nil }
