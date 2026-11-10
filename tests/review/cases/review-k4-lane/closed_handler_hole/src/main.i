Svc(A) {
	Svc(f: () -> A)
}

use = (s ! Svc(Int)) { s.f() }

wrap(N) = (body: () -> Int ! N | Svc(Int), svc: Svc(Int)) -> Int ! N {
	! Svc(_) = svc
	body()
}

@main
main = () { assert(wrap(use, Svc(() { 6 })) == 6) }
