// spec: 8.3, 11.7
// expect: stop "bit array: value is too short"
// at: spec/main:11:6
// at: spec/main:17:2

make = (n: Int) {
	b = <<1:u1>>
	c = <<0:u7>>
	b2 = <<b:bits, c:bits>>
	// The position of `<<`.
	x = <<b2:bits(9)>>
	x
}

@main
main = () {
	make(-3)
	Nil
}
