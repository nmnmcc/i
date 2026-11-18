S(N) = (() <- N -> ())

use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}

example = <N>() {
	local : () <- (S(N), ..N) -> () = () { use() }
	local()
}

main = () { example() }
