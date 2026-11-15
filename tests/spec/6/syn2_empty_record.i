// spec: 3, 6.1
// expect: pass
(
	main
)
identity = (r : ()) { r }
main = () {
	assert(identity(()) == ())
	Nil
}
