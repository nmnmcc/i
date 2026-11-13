//// Functions for tuples of two parts, `(a, b)`.
(
	new
	first
	second
	swap
	map_first
	map_second
)

/// Makes a pair.
new = <A, B>(first: A, second: B) -> (A, B) { (first, second) }

/// The first part of a pair.
first = <A, B>(pair: (A, B)) -> A { pair.0 }

/// The second part of a pair.
second = <A, B>(pair: (A, B)) -> B { pair.1 }

/// The pair with its parts the other way round.
///
/// ```i
/// pair.swap((1, "a"))  // ("a", 1)
/// ```
swap = <A, B>(pair: (A, B)) -> (B, A) {
	(x, y) = pair
	(y, x)
}

/// Changes the first part of a pair with a function.
map_first = <A, B, C, N>(of: (A, B), with: (A) <- N -> C) -> (C, B) {
	(x, y) = of
	(with(x), y)
}

/// Changes the second part of a pair with a function.
map_second = <A, B, C, N>(of: (A, B), with: (B) <- N -> C) -> (A, C) {
	(x, y) = of
	(x, with(y))
}
