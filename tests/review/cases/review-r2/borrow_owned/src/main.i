(
	i/request
	i/control
)
@host("test:async/v1", "request")
Later = (echo = (String) -> Request(String))

main = () <- (later: Later) {
	a = later.echo("borrow")
	b = later.echo("own")
	control.boundary((exit) {
		request.await(a) <- (
			request.Await(
				wait = (_) { control.suspend(exit, (k) { control.cancel(k) }) },
			),
		)
	})
	assert(request.state(a) == request.Waiting)
	control.boundary((exit) {
		request.await_owned(b) <- (
			request.Await(
				wait = (_) { control.suspend(exit, (k) { control.cancel(k) }) },
			),
		)
	})
	assert(request.state(b) == request.Cancelled)
	assert(request.await(a) == "borrow")
}
