// spec: 11.7, 9.6
// expect: stop "assertion failed: assert"
// at: spec/main:8:23
// at: spec/main:12:2

// The example of §11.7: the call of `assert` is in tail position, and it is
// still the first position.
positive = (n: Int) { assert(n > 0) }

@main
main = () {
	positive(-1)
	Nil
}
