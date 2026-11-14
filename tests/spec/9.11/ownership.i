// spec: 9.4, 9.11
// expect: pass
// runtime.md 5.4: cancelling a borrowed waiter leaves the producer Waiting;
// cancelling await_owned cancels the producer before the unwind returns.
(
	i/request
	i/control
)
@host("test:async/v1", "request")
Later = (echo = (String) -> Request(String))

main = () <- (later : Later) {
	borrowed = later.echo("borrowed")
	control.boundary((b) {
		{
			request.await(borrowed)
			panic("unreachable")
		} <- (
			request.Await(
				wait = (_) { control.suspend(b, (k) { control.cancel(k) }) },
			),
		)
	})
	assert(request.state(borrowed) == request.Waiting)
	request.cancel(request.ticket(borrowed))
	owned = later.echo("owned")
	control.boundary((b) {
		{
			request.await_owned(owned)
			panic("unreachable")
		} <- (
			request.Await(
				wait = (_) { control.suspend(b, (k) { control.cancel(k) }) },
			),
		)
	})
	assert(request.state(owned) == request.Cancelled)
}
