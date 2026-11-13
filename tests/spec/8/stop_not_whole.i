// spec: 8.3, 11.7
// expect: stop "bit array: bytes are not whole"
// at: spec/main:10:6
// at: spec/main:15:2
make = (n: Int) {
	b = <<1:u1>>
	c = <<0:u7>>
	b2 = <<b:bits, c:bits>>
	// The position of `<<`.
	x = <<b2:bits, b:bytes>>
	x
}

main = () {
	make(-3)
	Nil
}
