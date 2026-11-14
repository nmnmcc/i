// spec: 4.3, 4.4
// expect: pass
(
	main
	area
	Circle(..)
	Square(..)
	Shape
)

Circle = (radius = Float)
Square = (side = Float)
Shape = Circle | Square

area = (s : Shape) -> Float {
	s |>
		(Circle(..) = c) { 3.0 *. c.radius *. c.radius }
		| (Square(..) = q) { q.side *. q.side }
}

flip = (True) { False } | (False) { True }

main = () {
	b : Bool = area(Square(1.0)) <. 2.0
	assert(b)
	assert(flip(b) == False)
	assert(area(Circle(1.0)) == 3.0)
	r : Result(Int, String) = Ok(1)
	r |> (Ok(n)) { assert(n == 1) } | (Error(_)) { assert(False) }
	Nil
}
