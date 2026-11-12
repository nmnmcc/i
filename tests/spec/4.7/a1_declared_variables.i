// spec: 2.5, 4.3, 4.7, 5, 6.1
// expect: pass
(
	main
)
Box(A) = (A)
Alias(A) = Box(A)
id: <A>(A) -> A = <A>(x) { x }
outer = <A>(x: A) -> A {
	inherited = (y: A) -> A { y }
	inner = <A>(y: A) -> A { y }
	assert(inner(1) == 1)
	assert(inner("different") == "different")
	inherited(x)
}
unbox = <A>(b: Alias(A)) -> A { b |> (Box(x)) { x } }

main = () {
	assert(id(4) == 4)
	assert(id("four") == "four")
	assert(outer("kept") == "kept")
	assert(unbox(Box(3)) == 3)
	assert(unbox(Box("value")) == "value")
	Nil
}
