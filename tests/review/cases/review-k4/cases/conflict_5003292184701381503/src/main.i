S(A) = (value = A)
read = () <- (s: S(String)) { s.value <> "!" }
read_int = () <- (s: S(Int)) { s.value + 1 }
run = (f) { f() }
join = <N, M>(f: () <- N -> String, g: () <- M -> Int) {
	run(() {
		f()
		g()
	})
}
bad = () { join(read, read_int) }
