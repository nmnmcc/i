// spec: 11.7
// expect: stop "float: result is not finite"
// at: spec/stop_tail_library:11:21
// at: spec/stop_tail_library:15:2
(
	i/core
)

// When the stop is in the library function itself, the tail call is the
// first position.
root = (x: Float) { core.sqrt(x) }


main = () {
	root(-.1.0)
	Nil
}
