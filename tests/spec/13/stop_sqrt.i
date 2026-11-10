// spec: 13, 11.3
// expect: stop "float: result is not finite"
// at: spec/stop_sqrt:10:3
// at: spec/stop_sqrt:15:3

(
	i/core
)

// A library function of the runtime in tail position keeps its position.
value = () { core.sqrt(-.1.0) }

@main
main = () {
	value()
	Nil
}
