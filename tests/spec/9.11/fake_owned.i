// spec: 9.11, 10.2
// expect: pass
// runtime.md 5.4: borrowed await takes; owning await cancels after take as a no-op.
// runtime.md D5: Await/Root/Owner-only needs can use completed fakes synchronously.
(
	i/request
	i/core
)

main = () <- (root: request.Root, owner: request.Owner) {
	assert(core.portable(root))
	assert(core.portable(owner))
	assert(request.count(owner) == 0)
	assert(request.count_root(root) == 0)
	a = request.ready(1)
	assert(request.await(a) == 1)
	assert(request.state(a) == request.Taken)
	b = request.ready("two")
	assert(request.await_owned(b) == "two")
	assert(request.state(b) == request.Taken)
	request.wait_none(owner)
	request.wait_none_root(root)
	request.cancel_all(owner)
	request.cancel_all_root(root)
	assert(request.settled_owner(owner))
	assert(request.settled_root(root))
	group = request.open_owner()
	request.close_owner(group)
	request.close_owner(group)
	assert(request.count(group) == 0)
	request.wait_none(group)
	independent = request.open_root()
	request.close_root(independent)
	request.close_root(independent)
	assert(request.count_root(independent) == 0)
	request.wait_none_root(independent)
}
