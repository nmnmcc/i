// spec: 8.3, 11.7
// expect: stop "bit array: value is too short"
// at: spec/stop_short_huge_bytes:11:6
// at: spec/stop_short_huge_bytes:17:2

// A length that no bit array can have still only makes the value too
// short: the stop is not "out of memory".
id = (n: Int) { n }

make = (b: BitArray) {
	x = <<b:bytes(id(1152921504606846976))>>
	x
}


main = () {
	make(<<1>>)
	Nil
}
