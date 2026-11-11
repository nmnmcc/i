Dep {
	Dep(value: Int)
}

get = (d ! Dep) { d.value }

S(N) {
	S(f: () -> Int ! N)
}

use(N) = (s ! S(N)) {
	S(f, ..) = s
	f()
}

wrap(N) = (svc: S(N)) -> Int ! N {
	! S(N) = svc
	use()
}


main = () {
	! Dep = Dep(7)
	assert(wrap(S(() { get() })) == 7)
}
