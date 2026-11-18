S(N) = (() <- N -> ())

use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}

main = <N>() {
	local = () { use() }
	()
}
