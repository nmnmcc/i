S(A) = (value = A)
read = () <- (s: S(String)) { s.value <> "!" }
read_int = () <- (s: S(Int)) { s.value + 1 }
run = (f) { f() }
with = <A>(x: A, f) { f() <- (S(value = x)) }

main = () {
	with(1, read)
	Nil
}
