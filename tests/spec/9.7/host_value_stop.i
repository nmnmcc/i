// spec: B.2, 11.7
// expect: stop "host: value is not correct"
// at: spec/main:13:6
// at: spec/main:19:2

// A value from the host that is not correct stops the program.
@host("test:bad/v1")
Bad = (nan = () -> Float)

half = () <- (bad: Bad) {
	x = bad.nan()
	x /. 2.0
}

main = () {
	half()
	Nil
}
