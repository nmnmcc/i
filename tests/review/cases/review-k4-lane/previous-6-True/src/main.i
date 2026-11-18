S(E, N) = (() <- (Fail(E), ..N) -> ())

use = <E, N>() <- (s : S(E, N)) {
	S(f) = s
	f()
}

run = <N>(body : () <- N -> ()) { body() }

example = <E, N>() { run(() { use() }) }

main = () { example() }
