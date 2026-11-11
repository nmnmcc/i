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

wrap = (svc) {
	! S(_) = svc
	use()
}


main = () {
	! Dep = Dep(7)
	assert(wrap(S(() { get() })) == 7)
}
