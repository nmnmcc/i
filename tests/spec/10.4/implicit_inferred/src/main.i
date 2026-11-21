(
	i/hostcall
	i/request
)

(
	main
)

// Needs Root through `group`, no row written.
open = () { hostcall.group() }

// A value parameter of an implicit service is written by the caller.
open_at = (root : hostcall.Root) -> hostcall.CallGroup { hostcall.group_at(root) }

// Needs Await through `await`, no row written.
get = (c : hostcall.Call(Int)) -> Int { hostcall.await(c) }

main = () {
	a = open()
	b = open_at(hostcall.root())
	assert(get(request.ready(5)) == 5)
	hostcall.close(a)
	hostcall.close(b)
}
