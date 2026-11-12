Box(A) = (A)
v: Box(Int | String) = True |> (True) { Box(1) } | (False) { Box("s") }
Box(Item) = v
