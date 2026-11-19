S(N) = (() <- N -> (), (N) -> ())

use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}

run = <N>(body : () <- N -> ()) { body() }

example = () { run(() { use() }) }

main = () { example() }
