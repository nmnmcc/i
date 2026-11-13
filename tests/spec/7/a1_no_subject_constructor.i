// spec: 6.12, 7.5
// expect: compile-error E0512
(
	main
)
Circle = (r = Int)
Square = (side = Int)
Shape = Circle | Square
bad = (s: Shape) { s |> (Circle(..)) { s.r } }

main = () {
	assert(True)
	Nil
}
