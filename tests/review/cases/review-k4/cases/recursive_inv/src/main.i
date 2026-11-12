Cell(A) = (A, (A) -> Nil)
bad = (x) {
	y: Cell(_) = x
	Cell(y, (_) { Nil }) |> bad
}
