(
	i/fs
	i/fail
	i/clock
	i/core
)

@test
file_operations = () {
	path = fs.temp_dir() <> "/i-fs-test-" <> core.inspect(clock.monotonic_ns())
	fs.write_file(path, <<"a\nb\n":utf8>>)
	f = fs.open(path, fs.ReadOnly)
	assert(fs.read(f, 2) == <<"a\n":utf8>>)
	assert(fs.read(f, 10) == <<"b\n":utf8>>)
	assert(fs.read(f, 10) == <<>>)
	assert(
		fail.catch(() { fs.read(f, 0) }) ==
			Error(fs.FsError("read", "", "invalid"))
	)
	fs.close(f)
	fs.close(f)
	assert(
		fail.catch(() { fs.read(f, 1) }) ==
			Error(fs.FsError("read", "", "closed"))
	)
	assert(fs.read_lines(path) == ["a", "b"])
	f2 = fs.open(path, fs.Append)
	fs.write(f2, <<"c":utf8>>)
	fs.close(f2)
	assert(fs.read_file(path) == <<"a\nb\nc":utf8>>)
	fs.remove(path)
	Nil
}
