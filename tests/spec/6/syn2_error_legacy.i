// spec: 3, 6.8
// expect: pass
(
	main
)
value = () -> Int { 1 }
main = () {
	assert(value() == 1)
	Nil
}
