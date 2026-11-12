// spec: 9.6, 11.7
// expect: stop "assertion failed: assert"
// at: spec/main:6:2
// at: spec/main:11:2
positive = (n) {
	assert(n > 0)
	n
}

main = () {
	positive(0)
	Nil
}
