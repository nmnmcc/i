// spec: 11.4
// expect: pass
(
	testlib = (test) = i/test
)

check = (ok : Bool, what : String) {
	ok |> (True) { () } & (False) { panic(what) }
}

test("float_boundaries", () {
	// IEEE: -0.0 equals 0.0, and sums such as 0.1 + 0.2 are not 0.3.
	check(0.0 == -.0.0, "-0.0")
	check(-.0.0 == 0.0, "0.0")
	check({ 0.1 +. 0.2 == 0.3 } == False, "sum")
	check(1.5 == 1.5, "same")
	check({ 1.5 == 1.25 } == False, "different")
})
