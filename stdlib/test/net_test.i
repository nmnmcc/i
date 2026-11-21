(
	testlib = (test) = i/test
	i/net
	i/request
	i/resource
	i/fail
	i/task
)

// The public creators attach their closes to the current scope. The fake
// answers a close with a completed request the test keeps, so the test can
// see that the release ran: a taken request means the close was called.
fake_net = (
	listen_closed : Request(()),
	connect_closed : Request(()),
	accept_closed : Request(()),
	bind_closed : Request(())
) -> net.Net {
	net.Net(
		bind = (address, port) { request.ready(Ok(1)) },
		send_to = (id, address, port, bytes) { request.ready(Ok(())) },
		receive_from = (id, max) { request.ready(Ok(<<>>)) },
		local_address = (id) { request.ready(Ok("127.0.0.1")) },
		datagram_port = (id) { request.ready(Ok(1000)) },
		close_datagram = (id) { bind_closed },
		listen = (address, port) { request.ready(Ok(2)) },
		local_port = (id) { request.ready(Ok(1001)) },
		accept = (id) { request.ready(Ok(3)) },
		connect = (address, port) { request.ready(Ok(4)) },
		resolve = (name) { request.ready(Ok("")) },
		read = (id, max) { request.ready(Ok(<<>>)) },
		write = (id, bytes) { request.ready(Ok(())) },
		shutdown_write = (id) { request.ready(Ok(())) },
		close_stream = (id) {
			{ id == 3 } |>
				(True()) { accept_closed }
				& (False()) { connect_closed }
		},
		close_listener = (id) { listen_closed },
	)
}

test("net_scope_owns_handles_test", () {
	listen_closed = request.ready(())
	connect_closed = request.ready(())
	accept_closed = request.ready(())
	bind_closed = request.ready(())
	fake = fake_net(listen_closed, connect_closed, accept_closed, bind_closed)
	resource.scoped(() {
		listener = net.listen("127.0.0.1", 0) <- (fake)
		stream = net.connect("127.0.0.1", 1001) <- (fake)
		peer = net.accept(listener) <- (fake)
		socket = net.bind("127.0.0.1", 0) <- (fake)
		assert(net.local_port(listener) == 1001) <- (fake)
		assert(net.stream_number(stream) == 4)
		assert(net.stream_number(peer) == 3)
		()
	})
	assert(request.state(listen_closed) == request.Taken())
	assert(request.state(connect_closed) == request.Taken())
	assert(request.state(accept_closed) == request.Taken())
	assert(request.state(bind_closed) == request.Taken())
	()
})

test("net_real_scope_closes_handles_test", () {
	handles = resource.scoped(() {
		listener = net.listen("127.0.0.1", 0)
		port = net.local_port(listener)
		socket = net.bind("127.0.0.1", 0)
		client = net.connect("127.0.0.1", port)
		peer = net.accept(listener)
		(listener = listener, socket = socket, client = client, peer = peer)
	})
	closed = (operation) {
		Error(
			net.NetError(operation = operation, endpoint = "", code = "closed"),
		)
	}
	assert(
		fail.catch(() { net.local_port(handles.listener) }) ==
			closed("local_port"),
	)
	assert(
		fail.catch(() { net.local_port(handles.socket) }) ==
			closed("local_port"),
	)
	assert(fail.catch(() { net.read(handles.client, 1) }) == closed("read"))
	assert(fail.catch(() { net.read(handles.peer, 1) }) == closed("read"))
	()
})

// Cancelling the resource scope while `accept` waits must cancel the host
// request, so the listener is free for the next accept.
test("net_cancel_during_accept_test", () {
	resource.scoped(() {
		listener = net.listen("127.0.0.1", 0)
		port = net.local_port(listener)
		r = fail.catch(() {
			task.scope((group) {
				task.go(group, () {
					resource.scoped(() { net.accept(listener) })
				})
				task.yield()
				// The child is waiting in accept, so a second accept is busy.
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
				fail.raise("cancel accept")
			})
		})
		assert(r == Error("cancel accept"))
		client = net.connect("127.0.0.1", port)
		peer = net.accept(listener)
		net.write(client, <<"x":utf8>>)
		assert(net.read(peer, 1) == <<"x":utf8>>)
		()
	})
	()
})
