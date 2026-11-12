// spec: 4.5, 9.2
// expect: pass

// Ordinary provider fitting permits 0 <: Int; the environment itself stays Counter(Int).
(
	testlib = (test) = std/test
)

Counter(A) {
	Counter(get: () -> A)
}

read = (c ! Counter(Int)) { c.get() }

test("whole", () {
	! Counter(Int) = Counter(get: () { 0 })
	assert(read() == 0)
})
