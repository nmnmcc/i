S(E, N) = (() <- (Fail(E), ..N) -> Nil)

use = <E, N>() <- (s : S(E, N)) {
	S(f) = s
	f()
}

run = <N>(body : () <- N -> Nil) { body() }

example = <E, N>() { run(() -> Nil { use() }) }

main = () { example() }
