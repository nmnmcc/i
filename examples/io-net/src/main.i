(
	i/net
	i/task
	i/stdio
	i/resource
)

// Program entry.
main = () {
	resource.scoped(() {
		listener = net.listen("127.0.0.1", 0)
		port = net.local_port(listener)
		task.run(() {
			task.scope((scope) {
				task.go(scope, () {
					resource.scoped(() {
						stream = net.accept(listener)
						bytes = net.read(stream, 100)
						net.write(stream, bytes)
					})
				})
				resource.scoped(() {
					stream = net.connect("localhost", port)
					net.write(stream, <<"hello":utf8>>)
					assert(net.read(stream, 100) == <<"hello":utf8>>)
					()
				})
			})
		})
		assert(net.resolve("localhost") != [])
	})
	stdio.print("net ok")
}
