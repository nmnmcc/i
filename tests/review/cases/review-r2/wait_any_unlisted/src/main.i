(
	i/request
)
@host("test:async/v1", "request")
Later = (echo = (String) -> Request(String))

main = () <- (later: Later, owner: request.Owner) {
	first = later.echo("first")
	second = later.echo("second")
	request.wait_any([request.ticket(second), request.ticket(second)])
	request.wait_none(owner)
	request.close_owner(owner)
	assert(request.take(first) == "first")
	assert(request.take(second) == "second")
}
