S(E) = (() <- (Fail(E)) -> Nil)
example = () { Nil <- (S(() { Nil }), S(() { Nil })) }
