(
	testlib = (test) = i/test
	i/fetch
	i/net
	i/request
	i/resource
	i/fail
	i/task
)

// The raw accept request, without the scope-owning `net.accept`: a fake
// transport can wait on it, and cancelling the fake's open must cancel it.
accept_request = (net.Listener(number)) <- (raw : net.Net) {
	raw.accept(number)
}

fake_fetch = (start, closed : Request(())) -> fetch.Fetch {
	fetch.Fetch(
		start = start,
		status = (id) { request.ready(Ok(200)) },
		headers = (id) { request.ready(Ok("")) },
		read = (id, max) { request.ready(Ok(<<>>)) },
		close = (id) { closed },
	)
}

test("fetch_scope_owns_response_test", () {
	closed = request.ready(())
	fake = fake_fetch(
		(method, url, headers, body) { request.ready(Ok(1)) },
		closed,
	)
	resource.scoped(() {
		res = fetch.open("GET", "http://example.test/", "", <<>>) <- (fake)
		assert(fetch.status(res) == 200) <- (fake)
		assert(fetch.read(res, 4) == <<>>) <- (fake)
		()
	})
	// The scope's release called the host close and awaited it.
	assert(request.state(closed) == request.Taken())
	()
})

// Cancelling the scope while `open` waits must cancel the host request, not
// abandon it; no response handle exists, so no close is recorded.
test("fetch_cancel_during_open_test", () {
	resource.scoped(() {
		listener = net.listen("127.0.0.1", 0)
		pending = accept_request(listener)
		closed = request.ready(())
		fake = fake_fetch((method, url, headers, body) { pending }, closed)
		r = fail.catch(() {
			task.scope((group) {
				task.go(group, () {
					resource.scoped(() {
						fetch.open("GET", "http://example.test/", "", <<>>) <- (
							fake,
						)
					})
				})
				task.yield()
				// The child is waiting on `pending`: a second raw accept is busy.
				assert(
					fail.catch(() { net.accept(listener) }) ==
						Error(
							net.NetError(
								operation = "accept",
								endpoint = "",
								code = "busy",
							),
						),
				)
				fail.raise("cancel fetch")
			})
		})
		assert(r == Error("cancel fetch"))
		assert(request.state(pending) == request.Cancelled())
		assert(request.state(closed) == request.Ready(()))
		()
	})
	()
})
