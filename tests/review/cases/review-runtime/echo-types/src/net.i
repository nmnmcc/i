export (Stream, Listener, NetError, Net, read, write, accept, listen, close_stream, close_listener)
Stream {}
Listener {}
NetError { NetError(String) }
Net { Net }
read = (stream: Stream, count: Int) -> BitArray ? NetError ! Net { panic("type-only stub") }
write = (stream: Stream, data: BitArray) -> Nil ? NetError ! Net { panic("type-only stub") }
accept = (listener: Listener) -> Stream ? NetError ! Net { panic("type-only stub") }
listen = (address: String, port: Int) -> Listener ? NetError ! Net { panic("type-only stub") }
close_stream = (stream: Stream) -> Nil ! Net { Nil }
close_listener = (listener: Listener) -> Nil ! Net { Nil }
