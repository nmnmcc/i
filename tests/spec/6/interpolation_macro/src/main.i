(
	m = spec/macros
)
main = () {
	name = "i"
	assert(m.pass("a\(name)b") == "aib")
	assert(
		"before \(m.pass("inside \(name)")) after" == "before inside i after",
	)
	Nil
}
