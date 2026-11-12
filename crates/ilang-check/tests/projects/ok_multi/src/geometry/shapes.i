(
	Shape
	Circle(..)
	Square(..)
	area
)

Shape = Circle | Square

Circle {
	Circle(radius: Float)
}

Square {
	Square(side: Float)
}

area = (s: Shape) -> Float {
	s |> {
		Circle(r)    -> 3.14 *. r *. r
		Square(side) -> side *. side
	}
}
