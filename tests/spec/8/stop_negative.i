// spec: 8.3, 11.7
// expect: stop "bit array: length is negative"
// at: spec/main:11:6
// at: spec/main:17:2

make = (n: Int) {
	b = <<1:u1>>
	c = <<0:u7>>
	b2 = <<b:bits, c:bits>>
	// The position of `<<`.
	x = <<1:u(n)>>
	x
}


main = () {
	make(-3)
	Nil
}
