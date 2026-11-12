(
	i/fs
	i/stdio
	i/fail
	i/clock
	i/core
)

// Program entry.
main = () {
	dir = fs.temp_dir() <>
		"/i-io-fs-example-" <>
		core.inspect(clock.monotonic_ns())
	file = dir <> "/first"
	moved = dir <> "/second"
	fs.mkdir(dir)
	fs.write_file(file, <<"""
		hello
		world\n
		""":utf8>>)
	assert(fs.read_file(file) == <<"hello\nworld\n":utf8>>)
	assert(fs.read_lines(file) == ["hello", "world"])
	assert(fs.list(dir) == ["first"])
	assert(fs.metadata(file) == fs.Metadata(12, "file"))
	fs.rename(file, moved)
	fs.remove(moved)
	fs.remove(dir)
	assert(
		fail.catch(() { fs.open(moved, fs.ReadOnly) }) ==
			Error(fs.FsError("open", moved, "not_found"))
	)
	stdio.print("fs ok")
}
