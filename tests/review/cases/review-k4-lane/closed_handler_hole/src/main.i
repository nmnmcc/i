Svc(A) = (f = () -> A)

use = () <- (s: Svc(Int)) { s.f() }

wrap = <N>(body: () <- (Svc(Int), ..N) -> Int, svc: Svc(Int)) -> Int {
	body() <- (svc)
}

main = () { assert(wrap(use, Svc(f = () { 6 })) == 6) }
