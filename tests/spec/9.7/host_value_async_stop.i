// spec: B.2, B.5, 11.7
// expect: stop "host: value is not correct"
// at: spec/host_value_async_stop:13:6
// at: spec/host_value_async_stop:19:2

// The answer of an async function is checked with the frames of the call.
@host("test:async/v1", "async")
Later {
	Later(nan: () -> Float)
}

half = (later ! Later) {
	x = later.nan()
	x /. 2.0
}


main = () {
	half()
	Nil
}
