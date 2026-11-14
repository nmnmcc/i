import(i / core, i / resource)
@host("probe:wasi/v1")
Wasi = (
	alloc = (Int) -> Int,
	free = (Int, Int) -> Nil,
	store = (Int, BitArray) -> Nil,
	load = (Int, Int) -> BitArray,
	fd_write = (Int, Int, Int, Int) -> Int,
)
more = (fd : Int, address : Int, count : Int, io : Int) <- (w : Wasi) -> Nil {
	case
	count
	(0) { Nil }
		| (_) {
			w.store(io, <<address:u32le, count:u32le>>)
			case
			w.fd_write(fd, io, 1, io + 8)
			(0) {
				<<written:u32le>> = w.load(io + 8, 4)
				case
				written
				(0) { Nil }
					| (_) { more(fd, address + written, count - written, io) }
			}
				| (_) { Nil }
		}
}
write = (fd : Int, bytes : BitArray) <- (w : Wasi) {
	count = core.bit_size(bytes) / 8
	resource.acquire(
		() { w.alloc(count) },
		(address) { w.free(address, count) },
		(address) {
			w.store(address, bytes)
			resource.acquire(
				() { w.alloc(12) },
				(io) { w.free(io, 12) },
				(io) { more(fd, address, count, io) },
			)
		},
	)
}

main = () { write(1, <<"hello\n":utf8>>) }
