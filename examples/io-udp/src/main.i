(
	i/net
	i/stdio
)

main = () {
	a = net.bind("127.0.0.1", 0)
	b = net.bind("127.0.0.1", 0)
	net.send_to(a, "localhost", net.local_port(b), <<"ping":utf8>>)
	packet = net.receive_from(b, 100)
	assert(packet.data == <<"ping":utf8>>)
	assert(packet.address == net.local_address(a))
	assert(packet.port == net.local_port(a))
	net.send_to(b, packet.address, packet.port, <<"pong":utf8>>)
	assert(net.receive_from(a, 100).data == <<"pong":utf8>>)
	net.close_datagram(a)
	net.close_datagram(b)
	stdio.print("udp ok")
}
