export(
	Stream,
	Listener,
	NetError,
	Net,
	read,
	write,
	accept,
	listen,
	close_stream,
	close_listener,
)
Stream = Opaque
Listener = Opaque
NetError = (String)
Net = ()
read = (stream : Stream, count : Int) -> BitArray { panic("type-only stub") }
write = (stream : Stream, data : BitArray) -> Nil { panic("type-only stub") }
accept = (listener : Listener) -> Stream { panic("type-only stub") }
listen = (address : String, port : Int) -> Listener { panic("type-only stub") }
close_stream = (stream : Stream) -> Nil { Nil }
close_listener = (listener : Listener) -> Nil { Nil }
