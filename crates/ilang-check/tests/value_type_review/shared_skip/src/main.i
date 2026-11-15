Box(A, B) = (A, B)
make = <A>(x : A) { Box(x, x) }
v = make(1)
Box(Item, _) = v
x : Item = 2
