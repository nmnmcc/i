// spec: 1.6
// expect: pass
main = () {
	assert(#"raw \(name) \n "quoted""# == "raw \\(name) \\n \"quoted\"")
	assert(##"a "# b"## == "a \"# b")
	assert(#"""# == "\"")
	assert(##""""## == "\"\"")
	Nil
}
