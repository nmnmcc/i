// spec: 9.5
// expect: stop "panic: first"
(
	i/resource
)

// The first release still runs after the second one has ended.
main = () {
	resource.scoped(() {
		a = resource.acquire(() { 1 }, (_) { panic("first") })
		b = resource.acquire(() { 2 }, (_) { () })
		a + b
	})
}
