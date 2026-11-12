(
	testlib = (test) = std/test
	i/net
	i/task
	i/fail
)

test("network", () {
	assert(
		fail.catch(() { net.listen("localhost", 0) }) ==
			Error(
				net.NetError(
					operation: "listen"
					endpoint:  "localhost"
					code:      "invalid"
				)
			)
	)
	assert(
		fail.catch(() { net.connect("localhost", 0) }) ==
			Error(
				net.NetError(
					operation: "connect"
					endpoint:  "localhost"
					code:      "invalid"
				)
			)
	)
	listener = net.listen("127.0.0.1", 0)
	port = net.local_port(listener)
	task.run(() {
		task.scope((scope) {
			task.go(scope, () {
				peer = net.accept(listener)
				assert(net.read(peer, 4) == <<"ping":utf8>>)
				assert(net.read(peer, 4) == <<>>)
				net.write(peer, <<"pong":utf8>>)
				net.close_stream(peer)
			})
			stream = net.connect("localhost", port)
			assert(
				fail.catch(() { net.read(stream, 0) }) ==
					Error(
						net.NetError(
							operation: "read"
							endpoint:  ""
							code:      "invalid"
						)
					)
			)
			net.write(stream, <<"ping":utf8>>)
			net.shutdown_write(stream)
			net.shutdown_write(stream)
			assert(
				fail.catch(() { net.write(stream, <<"x":utf8>>) }) ==
					Error(
						net.NetError(
							operation: "write"
							endpoint:  ""
							code:      "closed"
						)
					)
			)
			assert(net.read(stream, 4) == <<"pong":utf8>>)
			net.close_stream(stream)
			net.close_stream(stream)
			assert(
				fail.catch(() { net.read(stream, 1) }) ==
					Error(
						net.NetError(
							operation: "read"
							endpoint:  ""
							code:      "closed"
						)
					)
			)
			Nil
		})
	})
	assert(net.resolve("localhost") != [])
	net.close_listener(listener)
	net.close_listener(listener)
	assert(
		fail.catch(() { net.local_port(listener) }) ==
			Error(
				net.NetError(
					operation: "local_port"
					endpoint:  ""
					code:      "closed"
				)
			)
	)
	Nil
})

test("pending_accept_is_exclusive_and_closed", () {
	listener = net.listen("127.0.0.1", 0)
	task.run(() {
		task.scope((scope) {
			pending = task.go(scope, () {
				fail.catch(() { net.accept(listener) })
			})
			task.yield()
			assert(
				fail.catch(() { net.accept(listener) }) ==
					Error(
						net.NetError(
							operation: "accept"
							endpoint:  ""
							code:      "busy"
						)
					)
			)
			net.close_listener(listener)
			assert(
				task.join(pending) ==
					Error(
						net.NetError(
							operation: "accept"
							endpoint:  ""
							code:      "closed"
						)
					)
			)
			Nil
		})
	})
	Nil
})

test("pending_read_allows_write_and_close_wakes_it", () {
	listener = net.listen("127.0.0.1", 0)
	client = net.connect("localhost", net.local_port(listener))
	peer = net.accept(listener)
	task.run(() {
		task.scope((scope) {
			pending = task.go(scope, () {
				fail.catch(() { net.read(client, 10) })
			})
			task.yield()
			assert(
				fail.catch(() { net.read(client, 10) }) ==
					Error(
						net.NetError(
							operation: "read"
							endpoint:  ""
							code:      "busy"
						)
					)
			)
			net.write(client, <<"ok":utf8>>)
			assert(net.read(peer, 10) == <<"ok":utf8>>)
			net.close_stream(client)
			assert(
				task.join(pending) ==
					Error(
						net.NetError(
							operation: "read"
							endpoint:  ""
							code:      "closed"
						)
					)
			)
			Nil
		})
	})
	net.close_stream(peer)
	net.close_listener(listener)
	Nil
})
