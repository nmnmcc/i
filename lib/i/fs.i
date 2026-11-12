//// File system operations.
(
	i/request
	i/resource
	i/fail
	i/core
)

(
	Fs
	File
	FsError(..)
	OpenMode(..)
	open
	read
	write
	flush
	close
	remove
	rename
	mkdir
	list
	metadata
	temp_dir
	read_file
	write_file
	read_lines
	Metadata(..)
)

File {
	File(Int)
}

FsError {
	FsError(operation: String, path: String, code: String)
}

OpenMode {
	ReadOnly
	WriteTruncate
	Append
	ReadWrite
}

Metadata {
	Metadata(size: Int, kind: String)
}

@host("i:fs/v1", "request")
Fs {
	Fs(
		open:     (String, String) -> Request(Result(Int, String))
		read:     (Int, Int) -> Request(Result(BitArray, String))
		write:    (Int, BitArray) -> Request(Result(Nil, String))
		flush:    (Int) -> Request(Result(Nil, String))
		close:    (Int) -> Request(Nil)
		remove:   (String) -> Request(Result(Nil, String))
		rename:   (String, String) -> Request(Result(Nil, String))
		mkdir:    (String) -> Request(Result(Nil, String))
		list:     (String) -> Request(Result(String, String))
		size:     (String) -> Request(Result(Int, String))
		kind:     (String) -> Request(Result(String, String))
		temp_dir: () -> Request(Result(String, String))
	)
}

mode_string = {
	ReadOnly      -> "read"
	WriteTruncate -> "truncate"
	Append        -> "append"
	ReadWrite     -> "read_write"
}

open = (path, mode, raw ! Fs) {
	r = raw.open(path, mode_string(mode))
	number = resource.ensure(() {
		request.await(r) |> {
			Ok(value)   -> value
			Error(code) -> fail.raise(FsError(operation: "open", path: path, code: code))
		}
	}, () {
		request.state(r) |> {
			request.Ready(Ok(value)) -> request.await_owned(raw.close(value))
			_                        -> Nil
		}
		request.cancel(request.ticket(r))
	})
	File(number)
}

read = (File(number), max_bytes, raw ! Fs) {
	request.await_owned(raw.read(number, max_bytes)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError(operation: "read", path: "", code: code))
	}
}

write = (File(number), bytes, raw ! Fs) {
	request.await_owned(raw.write(number, bytes)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError(operation: "write", path: "", code: code))
	}
}

flush = (File(number), raw ! Fs) {
	request.await_owned(raw.flush(number)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError(operation: "flush", path: "", code: code))
	}
}

close = (File(number), raw ! Fs) { request.await_owned(raw.close(number)) }

remove = (path, raw ! Fs) {
	request.await_owned(raw.remove(path)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError(operation: "remove", path: path, code: code))
	}
}

rename = (from, to, raw ! Fs) {
	request.await_owned(raw.rename(from, to)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError(operation: "rename", path: from, code: code))
	}
}

mkdir = (path, raw ! Fs) {
	request.await_owned(raw.mkdir(path)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError(operation: "mkdir", path: path, code: code))
	}
}

slice = (text, start, length) {
	core.byte_slice(text, start, length) |> {
		Ok(value) -> value
		Error(_)  -> panic("fs: invalid string slice")
	}
}

list_parts = (text: String, start: Int) -> List(String) {
	core.index_of(text, "\u{0}", start) |> {
		Ok(end) -> [
			slice(text, start, end - start)
			..list_parts(text, end + 1)
		]
		Error(_) -> []
	}
}

list = (path, raw ! Fs) {
	request.await_owned(raw.list(path)) |> {
		Ok(value)   -> list_parts(value, 0)
		Error(code) -> fail.raise(FsError(operation: "list", path: path, code: code))
	}
}

metadata = (path, raw ! Fs) {
	size = request.await_owned(raw.size(path)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError(operation: "metadata", path: path, code: code))
	}
	kind = request.await_owned(raw.kind(path)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError(operation: "metadata", path: path, code: code))
	}
	Metadata(size: size, kind: kind)
}

temp_dir = (raw ! Fs) {
	request.await_owned(raw.temp_dir()) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError(operation: "temp_dir", path: "", code: code))
	}
}

read_chunks = (file) {
	bytes = read(file, 65536)
	core.bit_size(bytes) |> {
		0 -> <<>>
		_ -> core.bit_array_concat([bytes, read_chunks(file)])
	}
}

read_file = (path) {
	resource.acquire(() { open(path, ReadOnly) }, (f) { close(f) }, (f) {
		read_chunks(f)
	})
}

write_file = (path, bytes) {
	resource.acquire(() { open(path, WriteTruncate) }, (f) { close(f) }, (f) {
		write(f, bytes)
	})
}

line_parts = (text: String, start: Int) -> List(String) {
	core.index_of(text, "\n", start) |> {
		Ok(end) -> [
			slice(text, start, end - start)
			..line_parts(text, end + 1)
		]
		Error(_) -> {
			done = start == core.byte_size(text)
			done |> {
				True  -> []
				False -> [slice(text, start, core.byte_size(text) - start)]
			}
		}
	}
}

read_lines = (path) {
	core.bit_array_to_string(read_file(path)) |> {
		Ok(text) -> line_parts(text, 0)
		Error(_) -> fail.raise(FsError(operation: "read_lines", path: path, code: "invalid"))
	}
}
