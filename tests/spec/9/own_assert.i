// spec: 9.6
// expect: stop "panic: custom: assert"
// at: spec/main:8:26

// Any code may give its own `Assert` with an environment statement.

main = () {
	! Assert = Assert(fail: (f) { panic("custom: " <> f.message) })
	assert(False)
}
