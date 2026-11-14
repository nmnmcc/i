// spec: 7.1, 7.3, 9.6
// expect: stop "panic: keyed binding failed"
unpack = (data : (value = Int)) {
	(value = 0) = data
	Nil
}
main = () {
	unpack((value = 1)) <- (
		Assert(fail = (_) { panic("keyed binding failed") }),
	)
}
