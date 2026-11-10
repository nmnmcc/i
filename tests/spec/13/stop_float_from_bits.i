// spec: 13, 11.3
// expect: stop "float: result is not finite"
// at: spec/stop_float_from_bits:11:14
// at: spec/stop_float_from_bits:15:2

(
	i/core
)

// A library function of the runtime in tail position keeps its position.
value = () { core.float_from_bits(9218868437227405312) }

@main
main = () {
	value()
	Nil
}
