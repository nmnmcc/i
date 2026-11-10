S(E) { S(() -> Nil ? E) }
example = () { ! S(String) = S(() { Nil }) ! S(Int) = S(() { Nil }) Nil }
