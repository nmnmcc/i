(
	i/request
)

main = () <- (root : request.Root, owner : request.Owner) {
	assert(
		request.await_owned(request.ready(42)) == 42,
	)
	request.wait_none(owner)
	request.wait_none_root(root)
}
