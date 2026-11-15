S(A) = (value = A)
read = () <- (s : S(String)) { s.value <> "!" }
read_int = () <- (s : S(Int)) { s.value + 1 }
run = (f) { f() }
with = <A, R, N>(x : A, f : () <- N -> R) {
	() {
		_syn3_0 : S(A) = S(value = x)
		f() <- (_syn3_0)
	}
}

main = () {
	f = with("ok", read)
	f()
	Nil
}
