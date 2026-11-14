S(N) = (() <- N -> Nil)
a = <N>(b : Bool) <- (s : S(N)) {
	b |>
		(True) {
			S(f) = s
			f()
		}
		| (False) { z(True) }
}
z = (b : Bool) { a(b) }
