Cell(A) = (() -> A, (A) -> Nil)
bad = (x) { bad(Cell(() { x }, (_) { Nil })) }
