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
		open:     (path: String, mode: String) -> Request(Result(Int, String))
		read:     (file: Int, max_bytes: Int) -> Request(Result(BitArray, String))
		write:    (file: Int, bytes: BitArray) -> Request(Result(Nil, String))
		flush:    (file: Int) -> Request(Result(Nil, String))
		close:    (file: Int) -> Request(Nil)
		remove:   (path: String) -> Request(Result(Nil, String))
		rename:   (from: String, to: String) -> Request(Result(Nil, String))
		mkdir:    (path: String) -> Request(Result(Nil, String))
		list:     (path: String) -> Request(Result(String, String))
		size:     (path: String) -> Request(Result(Int, String))
		kind:     (path: String) -> Request(Result(String, String))
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
			Error(code) -> fail.raise(FsError("open", path, code))
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
		Error(code) -> fail.raise(FsError("read", "", code))
	}
}

write = (File(number), bytes, raw ! Fs) {
	request.await_owned(raw.write(number, bytes)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError("write", "", code))
	}
}

flush = (File(number), raw ! Fs) {
	request.await_owned(raw.flush(number)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError("flush", "", code))
	}
}

close = (File(number), raw ! Fs) { request.await_owned(raw.close(number)) }

remove = (path, raw ! Fs) {
	request.await_owned(raw.remove(path)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError("remove", path, code))
	}
}

rename = (from, to, raw ! Fs) {
	request.await_owned(raw.rename(from, to)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError("rename", from, code))
	}
}

mkdir = (path, raw ! Fs) {
	request.await_owned(raw.mkdir(path)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError("mkdir", path, code))
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
		Error(code) -> fail.raise(FsError("list", path, code))
	}
}

metadata = (path, raw ! Fs) {
	size = request.await_owned(raw.size(path)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError("metadata", path, code))
	}
	kind = request.await_owned(raw.kind(path)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError("metadata", path, code))
	}
	Metadata(size, kind)
}

temp_dir = (raw ! Fs) {
	request.await_owned(raw.temp_dir()) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(FsError("temp_dir", "", code))
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
		Error(_) -> fail.raise(FsError("read_lines", path, "invalid"))
	}
}
