Box(A) = (A)
choose = (b : Bool) { b |> (True) { Box(1) } | (False) { Box("s") } }
v = choose(True)
Box(Item) = v
x : Item = "s"
