// spec: B.2, 11.7
// expect: stop "host: value is not correct"
// at: spec/host_value_stop:11:7
// at: spec/host_value_stop:17:3

// A value from the host that is not correct stops the program.
@host("test:bad/v1")
Bad {
	Bad(nan: () -> Float)
}

half = (bad ! Bad) {
	x = bad.nan()
	x /. 2.0
}

@main
main = () {
	half()
	Nil
}
