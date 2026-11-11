// spec: 9.6, 11.7
// expect: stop "assertion failed: assert"
// at: spec/assert_main:7:2
// at: spec/assert_main:13:2

positive = (n) {
	assert(n > 0)
	n
}


main = () {
	positive(0)
	Nil
}
