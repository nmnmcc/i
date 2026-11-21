(
	i/request
)

main = () {
	owner = request.open_owner()
	{
		assert(
			request.await_owned(request.ready(42)) == 42,
		)
		request.wait_none(owner)
		request.wait_none_root(owner.root)
	} <- (owner)
}
