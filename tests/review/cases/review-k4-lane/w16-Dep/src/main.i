Dep {
	Dep(value: Int)
}

get = (d ! Dep) { d.value }

S(N) {
	S(f: () -> Int ! N, m: (N) -> Nil)
}

use(N) = (s ! S(N)) {
	S(f, ..) = s
	f()
}

wrap = (svc) {
	! S(_) = svc
	use()
}

@main
main = () {
	! Dep = Dep(7)
	assert(wrap(S(() { get() }, (_: Dep) { Nil })) == 7)
}
