// spec: 6.12, 7.4
// expect: compile-error E0516
(
	main
)
Red = ()
Green = ()
Blue = ()
Color = Red | Green | Blue
bad = (c : Color) { c |> (Red) { 1 } | (Green ? False) { 2 } }

main = () {
	assert(bad(Red) == 1)
	Nil
}
