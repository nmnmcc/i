(
	testlib = (test) = i/test
	i/tls
	i/net
	i/request
	i/resource
	i/fail
	i/task
	i/hostcall
)

accept_request = (net.Listener(number)) <- (raw : net.Net) {
	hostcall.call(() { raw.accept(number) })
}

fake_net = (connect_closed : Request(())) -> net.Net {
	net.Net(
		bind = (address, port) { Ok(0) },
		send_to = (id, address, port, bytes) { Ok(()) },
		receive_from = (id, max) { Ok(<<>>) },
		local_address = (id) { Ok("127.0.0.1") },
		datagram_port = (id) { Ok(1000) },
		close_datagram = (id) { () },
		listen = (address, port) { Ok(1) },
		local_port = (id) { Ok(1001) },
		accept = (id) { Ok(2) },
		connect = (address, port) { Ok(7) },
		resolve = (name) { Ok("") },
		read = (id, max) { Ok(<<>>) },
		write = (id, bytes) { Ok(()) },
		shutdown_write = (id) { Ok(()) },
		close_stream = (id) { request.take(connect_closed) },
		close_listener = (id) { () },
	)
}

test("tls_scope_owns_streams_test", () {
	connect_closed = request.ready(())
	accept_closed = request.ready(())
	fake = tls.Tls(
		connect = (stream, name, alpn, roots, cert, key) { Ok(1) },
		accept = (stream, cert, key, alpn) { Ok(2) },
		alpn = (id) { Ok("") },
		read = (id, max) { Ok(<<>>) },
		write = (id, bytes) { Ok(()) },
		shutdown_write = (id) { Ok(()) },
		close = (id) {
			{ id == 1 } |>
				(True()) { request.take(connect_closed) }
				& (False()) { request.take(accept_closed) }
		},
	)
	resource.scoped(() {
		client = tls.connect(net.Stream(0), "host", tls.options()) <- (fake)
		server = tls.accept(
			net.Stream(0),
			tls.identity(<<>>, <<>>),
			tls.options(),
		) <- (fake)
		assert(tls.alpn(client) == "") <- (fake)
		assert(tls.alpn(server) == "") <- (fake)
		()
	})
	assert(request.state(connect_closed) == request.Taken())
	assert(request.state(accept_closed) == request.Taken())
	()
})

// `dial` composes a TCP stream and a TLS stream. Both must belong to the same
// scope, and each host close must run exactly once at the scope's end.
test("tls_dial_owns_both_streams_test", () {
	raw_closed = request.ready(())
	tls_closed = request.ready(())
	net_fake = fake_net(raw_closed)
	tls_fake = tls.Tls(
		connect = (stream, name, alpn, roots, cert, key) { Ok(8) },
		accept = (stream, cert, key, alpn) { Ok(9) },
		alpn = (id) { Ok("") },
		read = (id, max) { Ok(<<>>) },
		write = (id, bytes) { Ok(()) },
		shutdown_write = (id) { Ok(()) },
		close = (id) { request.take(tls_closed) },
	)
	resource.scoped(() {
		stream = tls.dial("host", 443, tls.options()) <- (net_fake, tls_fake)
		assert(tls.alpn(stream) == "") <- (tls_fake)
		()
	})
	assert(request.state(tls_closed) == request.Taken())
	assert(request.state(raw_closed) == request.Taken())
	()
})

test("tls_cancel_during_open_test", () {
	resource.scoped(() {
		listener = net.listen("127.0.0.1", 0)
		pending = accept_request(listener)
		closed = request.ready(())
		fake = tls.Tls(
			connect = (stream, name, alpn, roots, cert, key) {
				request.await_owned(pending)
			},
			accept = (stream, cert, key, alpn) { request.await_owned(pending) },
			alpn = (id) { Ok("") },
			read = (id, max) { Ok(<<>>) },
			write = (id, bytes) { Ok(()) },
			shutdown_write = (id) { Ok(()) },
			close = (id) { request.take(closed) },
		)
		r = fail.catch(() {
			task.scope((group) {
				task.go(group, () {
					resource.scoped(() {
						tls.connect(net.Stream(0), "host", tls.options()) <- (
							fake,
						)
					})
				})
				task.yield()
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
				fail.raise("cancel tls")
			})
		})
		assert(r == Error("cancel tls"))
		assert(request.state(pending) == request.Cancelled())
		assert(request.state(closed) == request.Ready(()))
		()
	})
	()
})
