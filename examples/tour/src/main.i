(
	i/stdio
	std/int
)

Num = (Int)

Add = (Expr, Expr)
Mul = (Expr, Expr)
Expr = Num | Add | Mul
// Inferred: (e: Expr) -> Int
eval = (e) {
	e |>
		(Num(n)) { n }
		| (Add(a, b)) { eval(a) + eval(b) }
		| (Mul(a, b)) { eval(a) * eval(b) }
}

main = () {
	sum = Add(Num(1), Num(5))
	expr = Mul(sum, Num(7))
	expr |> eval |> int.to_string |> stdio.print
}
