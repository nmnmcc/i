(
	i/stdio
	std/int
)

Expr = Num | Add | Mul

Num {
	Num(Int)
}

Add {
	Add(Expr, Expr)
}

Mul {
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

main = () {
	sum = Add(Num(1), Num(5))
	expr = Mul(sum, Num(7))
	expr |> eval |> int.to_string |> stdio.print
}
