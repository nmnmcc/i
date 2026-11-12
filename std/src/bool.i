//// Functions for the built-in type `Bool { True False }`.
////
//// `&&` and `||` are operators and may not be used as values; `and` and
//// `or` are the same as functions, for example for `list.fold`. They do
//// both sides, because the arguments of a call are always worked out.
(
	i/core
	order = (Order, Less, Equal, Greater) = std/order
)

(
	not
	and
	or
	xor
	nand
	nor
	exclusive_nor
	to_string
	to_int
	compare
	guard
	lazy_guard
)

/// `False` for `True`, and `True` for `False`.
not = (a: Bool) -> Bool { core.not(a) }

/// `True` when both are `True`.
and = (a: Bool, b: Bool) -> Bool { a && b }

/// `True` when one or both are `True`.
or = (a: Bool, b: Bool) -> Bool { a || b }

/// `True` when exactly one is `True`.
xor = (a: Bool, b: Bool) -> Bool { a != b }

/// `False` when both are `True`.
nand = (a: Bool, b: Bool) -> Bool { core.not(a && b) }

/// `True` when both are `False`.
nor = (a: Bool, b: Bool) -> Bool { core.not(a || b) }

/// `True` when both are the same.
exclusive_nor = (a: Bool, b: Bool) -> Bool { a == b }

/// `"True"` or `"False"`.
to_string = (bool: Bool) -> String {
	bool |> (True) { "True" } | (False) { "False" }
}

/// `1` for `True` and `0` for `False`.
to_int = (bool: Bool) -> Int { bool |> (True) { 1 } | (False) { 0 } }

/// Compares two bools, with `False < True`.
compare = (a: Bool, with: Bool) -> Order {
	(a, with) |>
		((True, False)) { Greater }
		| ((False, True)) { Less }
		| (_) { Equal }
}

/// Gives `return` when `when` is `True`, and the value of `otherwise()`
/// when not. It is used to end early:
///
/// ```i
/// bool.guard(n < 0, "negative", () { "positive" })
/// ```
guard = <A, N>(when: Bool, return: A, otherwise: () <- N -> A) -> A {
	when |> (True) { return } | (False) { otherwise() }
}

/// Like `guard`, but the value for `True` is made by a function too.
lazy_guard = <A, N>(
	when: Bool,
	return: () <- N -> A,
	otherwise: () <- N -> A
) -> A { when |> (True) { return() } | (False) { otherwise() } }
