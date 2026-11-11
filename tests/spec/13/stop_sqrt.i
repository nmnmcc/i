// spec: 13, 11.3
// expect: stop "float: result is not finite"
// at: spec/main:11:14
// at: spec/main:15:2

(
	i/core
)

// A library function of the runtime in tail position keeps its position.
value = () { core.sqrt(-.1.0) }


main = () {
	value()
	Nil
}
