// spec: 11.7, 9.6
// expect: stop "assertion failed: assert"
// at: spec/main:8:24
// at: spec/main:11:2

// The example of §11.7: the call of `assert` is in tail position, and it is
// still the first position.
positive = (n : Int) { assert(n > 0) }

main = () {
	positive(-1)
	Nil
}
