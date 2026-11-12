(
	i/stdio
	std/int
)

Expr {
	Num(Int)
	Add(Expr, Expr)
	Mul(Expr, Expr)
}

// Inferred: (e: Expr) -> Int
eval = (e) {
	e |> {
		Num(n)    -> n
		Add(a, b) -> eval(a) + eval(b)
		Mul(a, b) -> eval(a) * eval(b)
	}
}

main = () { stdio.print(int.to_string(eval(Mul(Add(Num(1), Num(5)), Num(7))))) }
