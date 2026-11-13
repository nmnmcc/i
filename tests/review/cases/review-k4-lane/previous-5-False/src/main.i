S(N) = (() <- N -> Nil)

use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}

example = <N>() {
	local: () <- (S(N), ..N) -> Nil = () { use() }
	local()
}

main = () { example() }
