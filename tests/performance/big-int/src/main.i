(
	add
	compare
	multiply
	divide
	empty
)

// Keep the last result live across the loop; inputs arrive from the host.
add = (n: Int, a: Int, b: Int, last: Int) -> Int {
	{ n == 0 } |> (True) { last } | (False) { add(n - 1, a, b, a + b) }
}

compare = (n: Int, a: Int, b: Int, last: Bool) -> Bool {
	{ n == 0 } |> (True) { last } | (False) { compare(n - 1, a, b, a < b) }
}

multiply = (n: Int, a: Int, b: Int, last: Int) -> Int {
	{ n == 0 } |> (True) { last } | (False) { multiply(n - 1, a, b, a * b) }
}

divide = (n: Int, a: Int, b: Int, last: Int) -> Int {
	{ n == 0 } |> (True) { last } | (False) { divide(n - 1, a, b, a / b) }
}

empty = (n: Int, a: Int, b: Int, last: Int) -> Int {
	{ n == 0 } |> (True) { last } | (False) { empty(n - 1, a, b, a) }
}
