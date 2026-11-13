S(N) = (() <- N -> Nil)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = <A, N>(f: () <- N -> A) { f() }

main = <N>() <- (s: S(N)) {
	local: () <- (S(N), ..N) -> Nil = () { use() }
	run(local)
}
