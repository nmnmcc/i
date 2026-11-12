Svc(A) = (f = () -> A)

use = () <- (s: Svc(Int)) { s.f() }

wrap: <N>(() <- (Svc(Int), ..N) -> Int, Svc(Int)) <- N -> Int = (body, svc) {
	body() <- (svc)
}

main = () { assert(wrap(use, Svc(f = () { 6 })) == 6) }
