(
	i/net
	i/task
	i/fail
)

(
	ipv4
	ipv6
	errors
	close_pending
	cancel_pending
	all
)

exchange = (address) {
	a = net.bind(address, 0)
	b = net.bind(address, 0)
	assert(net.local_address(a) == address)
	assert(net.local_port(a) > 0)
	net.send_to(a, address, net.local_port(b), <<"first":utf8>>)
	net.send_to(a, address, net.local_port(b), <<"second":utf8>>)
	packet = net.receive_from(b, 3)
	assert(
		packet ==
			(data: <<"fir":utf8>>, address: address, port: net.local_port(a))
	)
	assert(net.receive_from(b, 100).data == <<"second":utf8>>)
	net.send_to(b, packet.address, packet.port, <<>>)
	assert(net.receive_from(a, 1).data == <<>>)
	net.close_datagram(a)
	net.close_datagram(a)
	assert(
		fail.catch(() { net.local_port(a) }) ==
			Error(net.NetError("local_port", "", "closed"))
	)
	assert(
		fail.catch(() { net.local_address(a) }) ==
			Error(net.NetError("local_address", "", "closed"))
	)
	assert(
		fail.catch(() { net.receive_from(a, 1) }) ==
			Error(net.NetError("receive_from", "", "closed"))
	)
	assert(
		fail.catch(() { net.send_to(a, address, 1, <<>>) }) ==
			Error(net.NetError("send_to", address, "closed"))
	)
	net.close_datagram(b)
}

ipv4 = () { exchange("127.0.0.1") }

ipv6 = () { exchange("::1") }

errors = () {
	socket = net.bind("localhost", 0)
	assert(net.local_address(socket) != "")
	assert(net.local_port(socket) > 0)
	assert(
		fail.catch(() { net.bind("", 0) }) ==
			Error(net.NetError("bind", "", "invalid"))
	)
	assert(
		fail.catch(() { net.bind("127.0.0.1", -1) }) ==
			Error(net.NetError("bind", "127.0.0.1", "invalid"))
	)
	assert(
		fail.catch(() { net.bind("127.0.0.1", 65536) }) ==
			Error(net.NetError("bind", "127.0.0.1", "invalid"))
	)
	assert(
		fail.catch(() {
			net.bind(net.local_address(socket), net.local_port(socket))
		}) ==
			Error(
				net.NetError(
					"bind"
					net.local_address(socket)
					"address_in_use"
				)
			)
	)
	assert(
		fail.catch(() { net.receive_from(socket, 0) }) ==
			Error(net.NetError("receive_from", "", "invalid"))
	)
	assert(
		fail.catch(() { net.send_to(socket, "localhost", 0, <<>>) }) ==
			Error(net.NetError("send_to", "localhost", "invalid"))
	)
	assert(
		fail.catch(() { net.send_to(socket, "localhost", 1, <<1:u1>>) }) ==
			Error(net.NetError("send_to", "localhost", "invalid"))
	)
	net.close_datagram(socket)
}

close_pending = () {
	a = net.bind("127.0.0.1", 0)
	b = net.bind("127.0.0.1", 0)
	task.run(() {
		task.scope((scope) {
			pending = task.go(scope, () {
				fail.catch(() { net.receive_from(a, 10) })
			})
			task.yield()
			assert(
				fail.catch(() { net.receive_from(a, 10) }) ==
					Error(net.NetError("receive_from", "", "busy"))
			)
			net.send_to(a, "127.0.0.1", net.local_port(b), <<"ok":utf8>>)
			assert(net.receive_from(b, 10).data == <<"ok":utf8>>)
			net.close_datagram(a)
			assert(
				task.join(pending) ==
					Error(net.NetError("receive_from", "", "closed"))
			)
			Nil
		})
	})
	net.close_datagram(b)
}

cancel_pending = () {
	a = net.bind("127.0.0.1", 0)
	b = net.bind("127.0.0.1", 0)
	task.run(() {
		assert(fail.catch(() {
			task.scope((scope) {
				task.go(scope, () { net.receive_from(a, 10) })
				task.yield()
				fail.raise("cancel UDP receive")
			})
		}) == Error("cancel UDP receive"))
	})
	net.send_to(b, "127.0.0.1", net.local_port(a), <<"ok":utf8>>)
	assert(net.receive_from(a, 10).data == <<"ok":utf8>>)
	net.close_datagram(a)
	net.close_datagram(b)
}

all = () {
	ipv4()
	ipv6()
	errors()
	close_pending()
	cancel_pending()
}
