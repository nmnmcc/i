// spec: 9.5
// expect: stop "panic: resource: scope is not open"
(
	i/fail
	i/resource
)

the_scope = () <- (s : resource.Scope) { s }

// A closed Scope stops an acquisition before `open`: the panic names the
// scope, not the poisoned `open`.
main = () {
	s = resource.scoped(() { the_scope() })
	resource.acquire(() { panic("opened") }, (_) { () }) <- (s)
}
