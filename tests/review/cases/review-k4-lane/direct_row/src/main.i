Dep {
	Dep
}

S(N) {
	S(() -> Nil ! N)
}

T(N) {
	T(() -> Nil ! N)
}

use = (x: T(S(Never) | S(Dep))) { x }

@main
main = () { Nil }
