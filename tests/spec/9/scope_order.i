// spec: 9.5
// expect: stop "panic: second"
(
	i/resource
)

// Releases run in reverse order: the second release runs first. With
// `scope_order_outer`, this shows that both run, last attached first.
main = () {
	resource.scoped(() {
		a = resource.acquire(() { 1 }, (_) { panic("first") })
		b = resource.acquire(() { 2 }, (_) { panic("second") })
		a + b
	})
}
