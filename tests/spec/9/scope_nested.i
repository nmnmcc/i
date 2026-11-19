// spec: 9.5
// expect: stop "panic: inner"
(
	i/resource
)

// An inner scope releases before the outer one, even when the outer release
// was attached first.
main = () {
	resource.scoped(() {
		a = resource.acquire(() { 1 }, (_) { panic("outer") })
		b = resource.scoped(() {
			resource.acquire(() { 2 }, (_) { panic("inner") })
		})
		a + b
	})
}
