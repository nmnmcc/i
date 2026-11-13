// spec: 1.6
// expect: pass
main = () {
	x = "i"
	assert("raw \\n \\(x)\n  \(x)\n" == "raw \\n \\(x)\n  i\n")
	assert("ab" == "ab")
	Nil
}
