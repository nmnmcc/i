// spec: 6.10, 9.6, 11.7
// expect: stop "assertion failed: pattern Ok(v) did not match Error(7)"
// at: spec/main:6:2
// at: spec/main:11:2
first = (r : Result(Int, Int)) {
	Ok(v) = r
	v
}

main = () {
	first(Error(7))
	Nil
}
