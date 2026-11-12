S(A) = (value = A)
read = () <- (s: S(String)) { s.value <> "!" }
read_int = () <- (s: S(Int)) { s.value + 1 }
run = (f) { f() }
with = <A, R, N>(x: A, f: () <- N -> R) { f() <- (S(value = x)) }

main = () {
	with("ok", read)
	Nil
}
