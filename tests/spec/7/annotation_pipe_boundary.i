// spec: 3, 6.8, 7.1
// expect: pass
(
	testlib = (test) = std/test
)

Box = (value = Int | String)
Clock = (now = () -> Int)

same = (x : Int | x) { x }
union = (x : { Int | String }) { x }
last = (True | _ : Int) { 42 }
unbox = (Box(value = x : { Int | String })) { x }
f : (Int) -> { Int | String } = (n) { n }
remaining = (deadline) <- (clock : Clock) {
	left : Int = deadline - clock.now()
	left
}

test("annotation pipe boundary", () {
	assert(same(7) == 7)
	assert(same("other") == "other")
	assert(union(7) == 7)
	assert(union("other") == "other")
	assert(last(True) == 42)
	assert(last(9) == 42)
	assert(unbox(Box(value = 7)) == 7)
	assert(unbox(Box(value = "other")) == "other")
	v : { Int | String } = f(7)
	assert(v == 7)
	left = remaining(42) <- (Clock(now = () { 40 }))
	assert(left == 2)
})
