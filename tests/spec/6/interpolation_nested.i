// spec: 6.9
// expect: pass
main = () {
	x = "i"
	identity = (s: String) { s }
	assert("a\(identity("b\(x)c"))d" == "abicd")
	assert("\({
	y = "nested"
	y
})" == "nested")
	Nil
}
