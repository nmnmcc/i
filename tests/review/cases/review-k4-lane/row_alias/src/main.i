Dep {
	Dep
}

S(N) {
	S(() -> Nil ! N)
}

T(N) {
	T(() -> Nil ! N)
}

Bad = T(S(Never) | S(Dep))

use = (x: Bad) { x }


main = () { Nil }
