(
	testlib = (test) = std/test
	sugar = (if) = tour/sugar
)

sign = (n) { if(n < 0, "negative", "not negative") }

test("if is a macro", () { assert(sign(-1) == "negative") })
