(
	i/net
	i/task
	i/stdio
)

main = () {
	listener = net.listen("127.0.0.1", 0)
	port = net.local_port(listener)
	task.run(() {
		task.scope((scope) {
			task.go(scope, () {
				stream = net.accept(listener)
				bytes = net.read(stream, 100)
				net.write(stream, bytes)
				net.close_stream(stream)
			})
			stream = net.connect("localhost", port)
			net.write(stream, <<"hello":utf8>>)
			assert(net.read(stream, 100) == <<"hello":utf8>>)
			net.close_stream(stream)
			Nil
		})
	})
	assert(net.resolve("localhost") != [])
	net.close_listener(listener)
	stdio.print("net ok")
}
