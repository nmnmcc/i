(
	Shape(..)
	area
)

Circle = (radius = Float)
Square = (side = Float)
Shape = Circle | Square

area = (s: Shape) -> Float {
	s |> (Circle(r)) { 3.14 *. r *. r } | (Square(side = side)) { side *. side }
}
