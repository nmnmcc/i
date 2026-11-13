// spec: 9.11
// expect: stop "panic: request: cancelled"
// runtime.md 5.1: cancel discards Ready; aliases see Cancelled; take stops.
(
	i/request
)

main = () {
	r = request.ready("answer")
	alias = r
	request.cancel(request.ticket(r))
	request.cancel(request.ticket(r))
	assert(request.state(alias) == request.Cancelled)
	request.take(alias)
}
