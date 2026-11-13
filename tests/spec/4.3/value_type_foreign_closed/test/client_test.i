(
	testlib = (test) = std/test
	(Alias) = spec/privacy
	(RemoteSize) = spec/main
)

test("closed_read", () {
	f = (n: RemoteSize) { n }
	assert(f(3) == 3)
})

test("opaque_alias_export", () {
	same = (value: Alias) { value }
	assert(
		same(privacy.make()) == privacy.make(),
	)
})
