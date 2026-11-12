//// The result of comparing two values, and functions to work with it.
////
//// Compare functions in `std` have the type `(a, a) -> Order`, for example
//// `int.compare` and `string.compare`. `list.sort` takes such a function.

(
	Order(..)
	negate
	reverse
	to_int
	compare
	max
	min
	break_tie
	lazy_break_tie
	is_less
	is_equal
	is_greater
)

/// The result of a comparison: the first value is `Less` than, `Equal` to,
/// or `Greater` than the second.
Order {
	Less
	Equal
	Greater
}

/// Turns `Less` into `Greater` and `Greater` into `Less`. `Equal` stays.
///
/// ```i
/// order.negate(Less)  // Greater
/// ```
negate = (order: Order) -> Order {
	order |> {
		Less    -> Greater
		Equal   -> Equal
		Greater -> Less
	}
}

/// Gives a compare function that orders values the other way round.
///
/// ```i
/// list.sort([1, 3, 2], order.reverse(int.compare))  // [3, 2, 1]
/// ```
reverse(A, E, N) = (
	orderer: (A, A) -> Order ^ E ! N
) -> (A, A) -> Order ^ E ! N { (a, b) { orderer(b, a) } }

/// `-1` for `Less`, `0` for `Equal`, and `1` for `Greater`.
to_int = (order: Order) -> Int {
	order |> {
		Less    -> -1
		Equal   -> 0
		Greater -> 1
	}
}

/// Compares two orders, with `Less < Equal < Greater`.
compare = (a: Order, with: Order) -> Order {
	x = to_int(a)
	y = to_int(with)
	{ x == y } |> {
		True -> Equal
		False -> { x < y } |> {
			True  -> Less
			False -> Greater
		}
	}
}

/// The greater of two orders, with `Less < Equal < Greater`.
max = (a: Order, b: Order) -> Order {
	compare(a, b) |> {
		Less -> b
		_    -> a
	}
}

/// The smaller of two orders, with `Less < Equal < Greater`.
min = (a: Order, b: Order) -> Order {
	compare(a, b) |> {
		Greater -> b
		_       -> a
	}
}

/// Gives `with` when `order` is `Equal`, and `order` when not. It puts
/// comparisons together: compare by the first key, and break ties by the
/// second.
///
/// ```i
/// order.break_tie(int.compare(1, 1), Less)  // Less
/// ```
break_tie = (in: Order, with: Order) -> Order {
	in |> {
		Equal -> with
		_     -> in
	}
}

/// Like `break_tie`, but `with` is only called when it is needed.
lazy_break_tie(E, N) = (in: Order, with: () -> Order ^ E ! N) -> Order ^ E ! N {
	in |> {
		Equal -> with()
		_     -> in
	}
}

/// `True` for `Less`.
is_less = (order: Order) -> Bool { order == Less }

/// `True` for `Equal`.
is_equal = (order: Order) -> Bool { order == Equal }

/// `True` for `Greater`.
is_greater = (order: Order) -> Bool { order == Greater }
