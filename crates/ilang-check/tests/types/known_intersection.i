// §4.8: when `P` is an intersection, each of its members is matched with
// `K`. `inc` has the type `(x: A & Int) -> A`, so `inc(5)` has the known
// type `Int`.
inc = (x) {
	x + 1
	x
}

same = () { inc(5) == 1 }

other = () { inc(5) == "s" }

// Matching `a & Int` with `Int | String` gives `a` the whole known type.
wide = (v: Int) { inc(v) == "s" }
