// spec: 6.10, 9.6, 11.7
// expect: stop "assertion failed: pattern Ok(v) did not match Error(7)"
// at: spec/binding_fails:7:2
// at: spec/binding_fails:13:2

first = (r: Result(Int, Int)) {
	Ok(v) = r
	v
}


main = () {
	first(Error(7))
	Nil
}
