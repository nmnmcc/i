S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
D = ()
run : <N>(() <- N -> ()) <- N -> () = (body) { body() }
forward = <N>() <- (s : S(N)) -> () {
	local = run
	local(() { use() })
}
