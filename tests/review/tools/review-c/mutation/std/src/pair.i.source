//// Functions for tuples of two parts, `(a, b)`.

export (
	new
	first
	second
	swap
	map_first
	map_second
)

/// Makes a pair.
new = (first: a, second: b) -> (a, b) { (first, second) }

/// The first part of a pair.
first = (pair: (a, b)) -> a { pair.0 }

/// The second part of a pair.
second = (pair: (a, b)) -> b { pair.1 }

/// The pair with its parts the other way round.
///
/// ```i
/// pair.swap((1, "a"))  // ("a", 1)
/// ```
swap = (pair: (a, b)) -> (b, a) {
	(x, y) = pair
	(y, x)
}

/// Changes the first part of a pair with a function.
map_first = (of: (a, b), with: (a) -> c ? e ! n) -> (c, b) ? e ! n {
	(x, y) = of
	(with(x), y)
}

/// Changes the second part of a pair with a function.
map_second = (of: (a, b), with: (b) -> c ? e ! n) -> (a, c) ? e ! n {
	(x, y) = of
	(x, with(y))
}
