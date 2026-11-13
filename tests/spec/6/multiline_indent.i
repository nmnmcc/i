// spec: 1.6
// expect: pass
main = () {
	value = "one\n  two"
	assert(value == "one\n  two")
	assert("" == "")
	Nil
}
