// spec: 8.3, 11.7
// expect: stop "bit array: value is too short"
// at: spec/main:11:6
// at: spec/main:16:2

// A length that no bit array can have still only makes the value too
// short: the stop is not "out of memory".
id = (n : Int) { n }

make = (b : BitArray) {
	x = <<b:bits(id(34359738369))>>
	x
}

main = () {
	make(<<1>>)
	Nil
}
