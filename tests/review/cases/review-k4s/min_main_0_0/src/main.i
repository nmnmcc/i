S(N) = (() <- N -> ())
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
run = <A, N>(f : () <- N -> A) { f() }

main = <N>() <- (s : S(N)) {
	local : () <- (S(N), ..N) -> () = () { use() }
	run(local)
}
