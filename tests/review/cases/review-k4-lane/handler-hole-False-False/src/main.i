Svc(A) = (f = () -> A)

use = () <- (s: Svc(Int)) { s.f() }

wrap: <N>(() <- (Svc(Int), ..N) -> Int, Svc(Int)) <- N -> Int = (body, svc) {
	_syn3_0: Svc(Int) = svc
	body() <- (_syn3_0)
}

main = () { assert(wrap(use, Svc(f = () { 6 })) == 6) }
