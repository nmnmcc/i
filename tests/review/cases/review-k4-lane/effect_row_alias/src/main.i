Dep {
	Dep
}

S(N) {
	S(() -> Nil ! N)
}

Bad = S(Never) | S(Dep)

use = (body: () -> Nil ! Bad) { body() }


main = () { Nil }
