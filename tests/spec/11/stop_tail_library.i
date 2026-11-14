// spec: 11.7
// expect: stop "float: result is not finite"
// at: spec/main:11:21
// at: spec/main:14:2
(
	i/core
)

// When the stop is in the library function itself, the tail call is the
// first position.
root = (x : Float) { core.sqrt(x) }

main = () {
	root(-.1.0)
	Nil
}
