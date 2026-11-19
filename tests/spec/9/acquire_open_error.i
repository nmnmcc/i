// spec: 9.5
// expect: pass
(
	i/fail
	i/resource
)

// When `open` raises, no release is attached: the error comes out as it was.
main = () {
	r = fail.catch(() {
		resource.scoped(() {
			x = resource.acquire(
				() { fail.raise("open failed") },
				(_) { panic("released") },
			)
			x + 1
		})
	})
	assert(r == Error("open failed"))
}
