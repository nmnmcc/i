// spec: 9.11
// expect: pass
// runtime.md 5.1: cancellation ownership is separate from Root affiliation.
// Group observations count only producers; cancel-all keeps groups open.
(
	i/request
)
@host("test:async/v1", "request")
Later = (echo = (String) -> Request(String))
make_group = (root: request.Root) { request.open_owner() <- (root) }
start = (owner: request.Owner, text: String) <- (later: Later) {
	later.echo(text) <- (owner)
}

main = () <- (root: request.Root, owner: request.Owner) {
	separate = request.open_root()
	other = make_group(separate)
	a = start(owner, "a")
	b = start(other, "b")
	assert(request.count_root(root) == 1)
	assert(request.count_root(separate) == 1)
	request.cancel_all(owner)
	assert(request.state(a) == request.Cancelled)
	assert(request.state(b) == request.Waiting)
	assert(request.count_root(root) == 0)
	assert(request.count_root(separate) == 1)
	request.wait_none_root(separate)
	assert(request.state(b) == request.Ready("b"))
	assert(request.take(b) == "b")
	c = start(other, "c")
	request.wait_none(other)
	request.close_owner(other)
	assert(request.take(c) == "c")
	{
		nested = request.open_owner()
		d = start(nested, "d")
		assert(request.count_root(root) == 0)
		assert(request.count_root(separate) == 1)
		request.cancel_all_root(separate)
		assert(request.state(d) == request.Cancelled)
		again = start(nested, "again")
		assert(request.await_owned(again) == "again")
		request.close_root(separate)
		assert(request.settled_root(separate))
	} <- (separate, owner)
}
