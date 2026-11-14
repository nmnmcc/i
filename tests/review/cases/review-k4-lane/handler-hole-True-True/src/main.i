Svc(A) = (f = () -> A)

use = () <- (s : Svc(Int)) { s.f() }

wrap = <N>(body : () <- (Svc(Int), ..N) -> Int, svc : Svc(Int)) {
	_syn3_0 : Svc(_) = svc
	body() <- (_syn3_0)
}

main = () { assert(wrap(use, Svc(f = () { 6 })) == 6) }
