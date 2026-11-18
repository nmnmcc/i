Cell(A) = (A, (A) -> ())
bad = (x) {
	y : Cell(_) = x
	Cell(y, (_) { () }) |> bad
}
