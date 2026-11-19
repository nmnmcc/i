// spec: 9.5
// expect: stop "panic: released 5"
(
	i/fail
	i/resource
)

// The release gets the resource when the scoped body ends with an error.
main = () {
	fail.catch(() {
		resource.scoped(() {
			r = resource.acquire(
				() { 5 },
				(r) { r |> (5) { panic("released 5") } & (_) { () } },
			)
			fail.raise(r)
		})
	})
}
