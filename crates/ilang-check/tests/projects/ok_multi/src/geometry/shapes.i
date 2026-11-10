(
	Shape(..)
	area
)

Shape {
	Circle(radius: Float)
	Square(side: Float)
}

area = (s: Shape) -> Float {
	s |> {
		Circle(r)    -> 3.14 *. r *. r
		Square(side) -> side *. side
	}
}
