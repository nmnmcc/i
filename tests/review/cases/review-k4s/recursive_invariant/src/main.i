Cell(A) { Cell(() -> A, (A) -> Nil) }
bad = (x) { bad(Cell(() { x }, (_) { Nil })) }
