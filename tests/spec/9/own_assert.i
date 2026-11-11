// spec: 9.6
// expect: stop "panic: custom: assert"
// at: spec/own_assert:8:26

// Any code may give its own `Assert` with an environment statement.

main = () {
	! Assert = Assert((f) { panic("custom: " <> f.message) })
	assert(False)
}
