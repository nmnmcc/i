// spec: B.2, 11.7
// expect: stop "host: value is not correct"
// at: spec/host_value_bits:12:3
// at: spec/host_value_bits:17:3

// A bit array from the host whose bits after the end are not 0 stops. The
// call is in tail position, and it is the first position (§11.7).
@host("test:bad/v1")
Bad {
	Bad(bits: () -> BitArray)
}

take = (bad ! Bad) { bad.bits() }

@main
main = () {
	take()
	Nil
}
