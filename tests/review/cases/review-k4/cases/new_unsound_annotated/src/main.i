S(A, N) = (() <- N -> A, (N) -> Nil)
use = <A, N>() <- (s : S(A, N)) {
	S(f, _) = s
	f()
}
run = (f) { f() }
read = () { run(() { use() }) <> "!" }
with = <X, R, N>(x : X, f : () <- (S(X, Never), ..N) -> R) {
	_syn3_0 : S(X, Never) = S(() { x }, (_ : Never) { Nil })
	f() <- (_syn3_0)
}

main = () {
	with(1, read)
	Nil
}
