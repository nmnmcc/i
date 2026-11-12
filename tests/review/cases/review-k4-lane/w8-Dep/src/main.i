Dep {
	Dep(value: Int)
}

get = (d ! Dep) { d.value }

S(N) {
	S(f: () -> Int ! N)
}

use(N) = (s ! S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = (svc) {
	! S(_) = svc
	use()
}


main = () {
	! Dep = Dep(value: 7)
	assert(wrap(S(f: () { get() })) == 7)
}
