// spec: B.2, B.5, 11.7
// expect: stop "host: value is not correct"
// at: spec/host_value_async_stop:11:7
// at: spec/host_value_async_stop:17:3

// The answer of an async function is checked with the frames of the call.
@host("test:async/v1", "async")
Later {
	Later(nan: () -> Float)
}

half = (later ! Later) {
	x = later.nan()
	x /. 2.0
}

@main
main = () {
	half()
	Nil
}
