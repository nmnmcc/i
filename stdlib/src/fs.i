//// File system operations over the host's `i:fs/v1` service.
////
//// The platform gives `Fs` on `native`, `nodejs` and `wasi` (the Node
//// binding and the WASI adapter both implement the service). On any other
//// target an entry that needs `Fs` is the compile error "`Fs` is not
//// provided by the target" (E0523).
////
//// Every operation that can fail raises an `FsError` through `Fail(FsError)`.
//// Its `reason` is an `FsProblem` made from the host's error code once, in
//// one place; `code` keeps the host's string for diagnostics. Two codes come
//// from this module rather than the host: `"depth"` for a path below
//// `walk`'s limit and `"same"` for `copy` onto the same path.
////
//// A `File` is owned by a `resource.Scope`: `open` attaches the file's
//// `close` to the current scope before it gives the `File`, so the handle is
//// closed when the scope ends, also on an error or a cancel. The `File`
//// constructor is private; only `open` makes one. Using a `File` after its
//// scope has ended raises an `FsError` whose `reason` is `Closed`.
////
//// The scope belongs to the task that made it: a child task opens its own
//// `resource.scoped` before it opens a file (the specification's resource
//// contract); attaching to a
//// scope from another task stops.
(
	i/request
	i/resource
	i/fail
	i/core
	lists = i/list
	string = i/string
)

(
	Fs
	File
	FsError(..)
	FsProblem(..)
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
	copy
	walk
	Metadata(..)
)

/// An open file: the host's handle number and the path it was opened with.
/// The constructor is private; `open` makes a `File`.
File = (number = Int, path = String)

NotFound = ()
Denied = ()
Exists = ()
Closed = ()
Busy = ()
Invalid = ()
Unsupported = ()
Other = ()

/// Why a file system operation failed.
///
/// - `NotFound`: the path does not exist.
/// - `Denied`: the operating system did not allow it.
/// - `Exists`: the path exists already.
/// - `Closed`: the `File` is closed (its scope has ended).
/// - `Busy`: another operation is using the `File`.
/// - `Invalid`: a bad argument or data, an interrupted operation, or a walk
///   deeper than its cap.
/// - `Unsupported`: the platform cannot do it.
/// - `Other`: any other error; see `FsError.code`.
FsProblem = NotFound
	| Denied
	| Exists
	| Closed
	| Busy
	| Invalid
	| Unsupported
	| Other

/// The error of a file system operation: the operation's name, the path it
/// was working on (`""` for `temp_dir`), the reason, and the host's raw error
/// code for diagnostics.
///
/// ```i
/// fail.catch(() { fs.metadata("/no/such/file") })
/// // Error(FsError(operation = "metadata", path = "/no/such/file",
/// //               reason = NotFound, code = "not_found"))
/// ```
FsError = (operation = String, path = String, reason = FsProblem, code = String)

ReadOnly = ()
WriteTruncate = ()
Append = ()
ReadWrite = ()
OpenMode = ReadOnly | WriteTruncate | Append | ReadWrite

Metadata = (size = Int, kind = String)

@host("i:fs/v1", "request")
Fs = (
	open = (String, String) -> Request(Result(Int, String)),
	read = (Int, Int) -> Request(Result(BitArray, String)),
	write = (Int, BitArray) -> Request(Result((), String)),
	flush = (Int) -> Request(Result((), String)),
	close = (Int) -> Request(()),
	remove = (String) -> Request(Result((), String)),
	rename = (String, String) -> Request(Result((), String)),
	mkdir = (String) -> Request(Result((), String)),
	list = (String) -> Request(Result(String, String)),
	size = (String) -> Request(Result(Int, String)),
	kind = (String) -> Request(Result(String, String)),
	temp_dir = () -> Request(Result(String, String)),
)

// The one map from the host's error codes to reasons.
reason_of = (code : String) -> FsProblem {
	code |>
		("not_found") { NotFound() }
		& ("denied") { Denied() }
		& ("exists") { Exists() }
		& ("closed") { Closed() }
		& ("busy") { Busy() }
		& ("invalid" | "interrupted") { Invalid() }
		& ("unsupported") { Unsupported() }
		& (_) { Other() }
}

failure = (operation : String, path : String, code : String) {
	failure_with(operation, path, reason_of(code), code)
}

mode_string =
	(ReadOnly()) { "read" }
	& (WriteTruncate()) { "truncate" }
	& (Append()) { "append" }
	& (ReadWrite()) { "read_write" }

// Opens the host handle. A cancel while the open is in flight cancels the
// request and closes a handle that has arrived already.
open_handle = (path, mode) <- (raw : Fs) {
	r = raw.open(path, mode_string(mode))
	number = resource.ensure(
		() {
			request.await(r) |>
				(Ok(value)) { value }
				& (Error(code)) { failure("open", path, code) }
		},
		() {
			request.state(r) |>
				(request.Ready(Ok(value))) {
					request.await_owned(raw.close(value))
				}
				& (_) { () }
			request.cancel(request.ticket(r))
		},
	)
	File(number = number, path = path)
}

/// Opens the file at `path` and gives a `File` owned by the current
/// `resource.Scope`: its `close` is attached to the scope before `open`
/// gives the `File`, and runs when the scope ends.
///
/// Needs `resource.Scope` and `Fs`. Raises `FsError` with operation
/// `"open"`, for example `NotFound` for a missing path with `ReadOnly`.
///
/// ```i
/// resource.scoped(() {
///   f = fs.open(path, fs.WriteTruncate)
///   fs.write(f, <<"hi":utf8>>)
/// })  // the file is closed here
/// ```
open = (path : String, mode : OpenMode) {
	resource.acquire(() { open_handle(path, mode) }, (file) { close(file) })
}

/// Reads up to `max_bytes` (at most 65536) bytes from `file`; an empty
/// `BitArray` at the end of the file.
///
/// Needs `Fs`. Raises `FsError` with operation `"read"` and the file's path,
/// for example `Closed` after the file's scope has ended.
///
/// ```i
/// resource.scoped(() { fs.read(fs.open(path, fs.ReadOnly), 5) })
/// // <<"hello":utf8>>
/// ```
read = (file : File, max_bytes : Int) <- (raw : Fs) {
	request.await_owned(raw.read(file.number, max_bytes)) |>
		(Ok(value)) { value }
		& (Error(code)) { failure("read", file.path, code) }
}

/// Writes all of `bytes` to `file`.
///
/// Needs `Fs`. Raises `FsError` with operation `"write"` and the file's path.
///
/// ```i
/// resource.scoped(() { fs.write(fs.open(path, fs.Append), <<"x":utf8>>) })
/// ```
write = (file : File, bytes : BitArray) <- (raw : Fs) {
	request.await_owned(raw.write(file.number, bytes)) |>
		(Ok(value)) { value }
		& (Error(code)) { failure("write", file.path, code) }
}

/// Flushes what was written to `file`.
///
/// Needs `Fs`. Raises `FsError` with operation `"flush"` and the file's path.
///
/// ```i
/// resource.scoped(() {
///   f = fs.open(path, fs.Append)
///   fs.write(f, <<"x":utf8>>)
///   fs.flush(f)
/// })
/// ```
flush = (file : File) <- (raw : Fs) {
	request.await_owned(raw.flush(file.number)) |>
		(Ok(value)) { value }
		& (Error(code)) { failure("flush", file.path, code) }
}

/// Closes `file`. Closing a file that is closed already does nothing. The
/// scope that owns the file calls this; a program does not need to.
///
/// Needs `Fs`. Raises nothing.
///
/// ```i
/// resource.scoped(() { fs.close(fs.open(path, fs.ReadOnly)) })
/// ```
close = (file : File) <- (raw : Fs) {
	request.await_owned(raw.close(file.number))
}

/// Removes the file or the empty directory at `path`.
///
/// Needs `Fs`. Raises `FsError` with operation `"remove"`.
///
/// ```i
/// fs.remove(path)
/// ```
remove = (path : String) <- (raw : Fs) {
	request.await_owned(raw.remove(path)) |>
		(Ok(value)) { value }
		& (Error(code)) { failure("remove", path, code) }
}

/// Moves `from` to `to`.
///
/// Needs `Fs`. Raises `FsError` with operation `"rename"` and path `from`.
///
/// ```i
/// fs.rename(old, new)
/// ```
rename = (from : String, to : String) <- (raw : Fs) {
	request.await_owned(raw.rename(from, to)) |>
		(Ok(value)) { value }
		& (Error(code)) { failure("rename", from, code) }
}

/// Makes the directory `path`; its parent must exist.
///
/// Needs `Fs`. Raises `FsError` with operation `"mkdir"`, for example
/// `Exists`.
///
/// ```i
/// fs.mkdir(path)
/// ```
mkdir = (path : String) <- (raw : Fs) {
	request.await_owned(raw.mkdir(path)) |>
		(Ok(value)) { value }
		& (Error(code)) { failure("mkdir", path, code) }
}

slice = (text, start, length) {
	core.byte_slice(text, start, length) |>
		(Ok(value)) { value }
		& (Error(_)) { panic("fs: invalid string slice") }
}

list_parts = (text : String, start : Int) -> List(String) {
	core.index_of(text, "\u{0}", start) |>
		(Ok(end)) {
			[slice(text, start, end - start), ..list_parts(text, end + 1)]
		}
		& (Error(_)) { [] }
}

/// The names of the entries of the directory `path`, without `.` and `..`.
///
/// Needs `Fs`. Raises `FsError` with operation `"list"`.
///
/// ```i
/// fs.list(directory)  // ["a.txt", "b"]
/// ```
list = (path : String) <- (raw : Fs) {
	request.await_owned(raw.list(path)) |>
		(Ok(value)) { list_parts(value, 0) }
		& (Error(code)) { failure("list", path, code) }
}

/// The size in bytes and the kind (`"file"`, `"directory"` or `"other"`) of
/// `path`. Symbolic links are followed.
///
/// Needs `Fs`. Raises `FsError` with operation `"metadata"`.
///
/// ```i
/// fs.metadata(path)  // Metadata(size = 5, kind = "file")
/// ```
metadata = (path : String) <- (raw : Fs) {
	size = request.await_owned(raw.size(path)) |>
		(Ok(value)) { value }
		& (Error(code)) { failure("metadata", path, code) }
	kind = request.await_owned(raw.kind(path)) |>
		(Ok(value)) { value }
		& (Error(code)) { failure("metadata", path, code) }
	Metadata(size = size, kind = kind)
}

/// The directory for temporary files.
///
/// Needs `Fs`. Raises `FsError` with operation `"temp_dir"` and path `""`.
///
/// ```i
/// fs.temp_dir()  // "/tmp"
/// ```
temp_dir = () <- (raw : Fs) {
	request.await_owned(raw.temp_dir()) |>
		(Ok(value)) { value }
		& (Error(code)) { failure("temp_dir", "", code) }
}

read_chunks = (file) {
	bytes = read(file, 65536)
	core.bit_size(bytes) |>
		(0) { <<>> }
		& (_) { core.bit_array_concat([bytes, read_chunks(file)]) }
}

/// All the bytes of the file at `path`. The file is open only during the
/// call.
///
/// Needs `Fs`. Raises `FsError` of `open` or `read`.
///
/// ```i
/// fs.read_file(path)  // <<"hello":utf8>>
/// ```
read_file = (path : String) {
	resource.scoped(() { read_chunks(open(path, ReadOnly())) })
}

/// Makes the file at `path` hold `bytes` and nothing else. The file is open
/// only during the call.
///
/// Needs `Fs`. Raises `FsError` of `open` or `write`.
///
/// ```i
/// fs.write_file(path, <<"hello":utf8>>)
/// ```
write_file = (path : String, bytes : BitArray) {
	resource.scoped(() { write(open(path, WriteTruncate()), bytes) })
}

line_parts = (text : String, start : Int) -> List(String) {
	core.index_of(text, "\n", start) |>
		(Ok(end)) {
			[slice(text, start, end - start), ..line_parts(text, end + 1)]
		}
		& (Error(_)) {
			done = start == core.byte_size(text)
			done |>
				(True()) { [] }
				& (False()) {
					[slice(text, start, core.byte_size(text) - start)]
				}
		}
}

/// The lines of the UTF-8 file at `path`, without their `"\n"`. A last line
/// without a `"\n"` is a line too.
///
/// Needs `Fs`. Raises `FsError` of `read_file`, or one with operation
/// `"read_lines"`, reason `Invalid` and code `"invalid"` when the file is not
/// UTF-8.
///
/// ```i
/// fs.read_lines(path)  // ["first", "second"]
/// ```
read_lines = (path : String) {
	core.bit_array_to_string(read_file(path)) |>
		(Ok(text)) { line_parts(text, 0) }
		& (Error(_)) { failure("read_lines", path, "invalid") }
}

// Writes each chunk before it reads the next; gives the bytes written.
copy_chunks = (source : File, target : File, total : Int) {
	bytes = read(source, 65536)
	count = core.shift_right(core.bit_size(bytes), 3)
	count |>
		(0) { total }
		& (_) {
			write(target, bytes)
			copy_chunks(source, target, total + count)
		}
}

/// Copies the bytes of the file `from` into the file `to`, which is made or
/// truncated, and gives the number of bytes copied. The copy goes by chunks
/// of 65536 bytes; each chunk is written before the next is read. Both files
/// are open only during the call. `from` and `to` must differ: the same path
/// raises `Invalid` with code `"same"` instead of truncating the source.
///
/// Needs `Fs`. Raises `FsError` of `open`, `read` or `write`.
///
/// ```i
/// fs.copy("/tmp/a.txt", "/tmp/b.txt")  // 5
/// ```
copy = (from : String, to : String) {
	{ from == to } |>
		(True()) { failure_with("copy", from, Invalid(), "same") }
		& (False()) { () }
	resource.scoped(() {
		source = open(from, ReadOnly())
		target = open(to, WriteTruncate())
		copy_chunks(source, target, 0)
	})
}

max_walk_depth = 32

child = (directory : String, name : String) -> String {
	size = core.byte_size(directory)
	slashed = size > 0 && slice(directory, size - 1, 1) == "/"
	slashed |>
		(True()) { directory <> name }
		& (False()) { directory <> "/" <> name }
}

walk_from = (path : String, depth : Int) -> List(String) {
	{ depth > max_walk_depth } |>
		(True()) { failure_with("walk", path, Invalid(), "depth") }
		& (False()) { () }
	metadata(path).kind |>
		("directory") {
			names = lists.sort(list(path), string.compare)
			below = (name) { walk_from(child(path, name), depth + 1) }
			[path, ..lists.flat_map(names, below)]
		}
		& (_) { [path] }
}

failure_with = (
	operation : String,
	path : String,
	reason : FsProblem,
	code : String
) {
	fail.raise(
		FsError(
			operation = operation,
			path = path,
			reason = reason,
			code = code,
		),
	)
}

/// Every path under `root`: `root` first, then depth first, the entries of
/// each directory in name order, and a directory before its contents. Only
/// `"directory"` entries are entered.
///
/// The walk goes at most 32 levels below `root`. A deeper path raises
/// `FsError(operation = "walk", path = <that path>, reason = Invalid,
/// code = "depth")`. `metadata` follows symbolic links, so a link to a
/// directory above it is a cycle: the depth cap ends the walk, but the
/// number of paths it visits before that grows with the tree's branching
/// factor, so a cycle with two entries per level is not finished in
/// practice. Do not walk a tree that can contain a directory link back up.
///
/// Needs `Fs`. Raises `FsError` of `list` or `metadata`, or the depth error.
///
/// ```i
/// fs.walk("/tmp/tree")
/// // ["/tmp/tree", "/tmp/tree/a", "/tmp/tree/a/x.txt", "/tmp/tree/b.txt"]
/// ```
walk = (root : String) { walk_from(root, 0) }
