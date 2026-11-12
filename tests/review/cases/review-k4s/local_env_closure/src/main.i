S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
Dep = (value = Int)
get = () <- (d: Dep) { d.value }
make = (n: Int) {
	() {
		use() <- (
			Dep(value = n),
			S(
				() {
					get()
					Nil
				},
				(_: Dep) { Nil },
			),
		)
	}
}
example = () {
	f = make(7)
	f()
}
