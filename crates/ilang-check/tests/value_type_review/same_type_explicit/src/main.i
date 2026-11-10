Box(A) { Box(A) }
left: Int = 1
right: Int = 2
v = True |> { True -> Box(left)  False -> Box(right) }
Box(Item) = v

