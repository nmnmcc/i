S(N) = (() <- N -> Nil)

use = <N>() <- (s : S(N)) -> Nil {
	S(f) = s
	f()
}

example = () { use() }

main = () { example() }
