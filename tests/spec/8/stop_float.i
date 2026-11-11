// spec: 8.3, 11.7
// expect: stop "bit array: float is not finite"
// at: spec/stop_float:11:6
// at: spec/stop_float:17:2

make = (n: Int) {
	b = <<1:u1>>
	c = <<0:u7>>
	b2 = <<b:bits, c:bits>>
	// The position of `<<`.
	x = <<1.0e10:f16be>>
	x
}


main = () {
	make(-3)
	Nil
}
