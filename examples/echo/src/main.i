(
	i/net
	i/task
	i/stdio
	i/core
	i/resource
)

echo = (stream) {
	bytes = net.read(stream, 65536)
	core.bit_size(bytes) |>
		(0) { () }
		& (_) {
			net.write(stream, bytes)
			echo(stream)
		}
}

// Accept one connection; keep accepting in a child of the same task scope and
// echo on this connection inside the resource scope that owns its stream.
serve = (listener, scope) {
	resource.scoped(() {
		stream = net.accept(listener)
		task.go(scope, () { serve(listener, scope) })
		echo(stream)
	})
}

// Program entry.
main = () {
	resource.scoped(() {
		listener = net.listen("127.0.0.1", 0)
		port = net.local_port(listener)
		stdio.print("listening " <> core.inspect(port))
		server = () { task.scope((scope) { serve(listener, scope) }) }
		task.run(server)
	})
}
