(
	i/net
	i/task
	i/stdio
	i/core
	i/resource
)

echo = (stream) {
	bytes = net.read(stream, 65536)
	core.bit_size(bytes) |> {
		0 -> Nil
		_ -> {
			net.write(stream, bytes)
			echo(stream)
		}
	}
}

serve = (listener, scope) {
	stream = net.accept(listener)
	task.go(scope, () {
		resource.ensure(() { echo(stream) }, () { net.close_stream(stream) })
	})
	serve(listener, scope)
}

// Program entry.
main = () {
	listener = net.listen("127.0.0.1", 0)
	port = net.local_port(listener)
	stdio.print("listening " <> core.inspect(port))
	server = () { task.scope((scope) { serve(listener, scope) }) }
	task.run(server)
}
