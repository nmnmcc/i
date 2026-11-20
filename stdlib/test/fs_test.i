(
	testlib = (test) = i/test
	i/fs
	i/fail
	i/resource
	i/clock
	i/core
	i/string
	i/list
	i/task
	option = (Option, Some, None) = i/option
)

// A path under the temporary directory that no other test or run uses.
fresh = (name : String) {
	fs.temp_dir() <>
		"/i-fs-test-" <>
		name <>
		"-" <>
		core.inspect(clock.monotonic_ns())
}

// Makes `n` nested directories `d` under `path`; gives them outermost first.
chain = (path : String, n : Int) {
	n |>
		(0) { [] }
		& (_) {
			next = path <> "/d"
			fs.mkdir(next)
			[next, ..chain(next, n - 1)]
		}
}

test("fs_round_trip_test", () {
	path = fresh("round-trip")
	resource.scoped(() {
		f = fs.open(path, fs.WriteTruncate)
		fs.write(f, <<"hello":utf8>>)
		fs.flush(f)
	})
	resource.scoped(() {
		f = fs.open(path, fs.ReadOnly)
		assert(fs.read(f, 3) == <<"hel":utf8>>)
		assert(fs.read(f, 10) == <<"lo":utf8>>)
		assert(fs.read(f, 10) == <<>>)
		// Errors on a file carry the file's path.
		assert(
			fail.catch(() { fs.read(f, 0) }) ==
				Error(
					fs.FsError(
						operation = "read",
						path = path,
						reason = fs.Invalid,
						code = "invalid",
					),
				),
		)
		()
	})
	assert(fs.metadata(path) == fs.Metadata(size = 5, kind = "file"))
	fs.remove(path)
	()
})

test("fs_scope_owns_handle_test", () {
	path = fresh("scope")
	fs.write_file(path, <<"x":utf8>>)
	// The handle leaves its scope; the scope has closed it.
	f = resource.scoped(() { fs.open(path, fs.ReadOnly) })
	assert(
		fail.catch(() { fs.read(f, 1) }) ==
			Error(
				fs.FsError(
					operation = "read",
					path = path,
					reason = fs.Closed,
					code = "closed",
				),
			),
	)
	// Closing a closed handle does nothing.
	fs.close(f)
	fs.remove(path)
	()
})

test("fs_open_missing_test", () {
	path = fresh("missing")
	assert(
		fail.catch(() { resource.scoped(() { fs.open(path, fs.ReadOnly) }) }) ==
			Error(
				fs.FsError(
					operation = "open",
					path = path,
					reason = fs.NotFound,
					code = "not_found",
				),
			),
	)
	()
})

test("fs_copy_test", () {
	from = fresh("copy-from")
	to = fresh("copy-to")
	// 70000 bytes: more than one 65536-byte chunk.
	text = string.repeat("0123456789", 7000)
	fs.write_file(from, <<text:utf8>>)
	assert(
		fail.catch(() { fs.metadata(to) }) ==
			Error(
				fs.FsError(
					operation = "metadata",
					path = to,
					reason = fs.NotFound,
					code = "not_found",
				),
			),
	)
	assert(fs.copy(from, to) == 70000)
	assert(fs.metadata(to) == fs.Metadata(size = 70000, kind = "file"))
	assert(fs.read_file(to) == <<text:utf8>>)
	// An empty file copies as zero bytes and truncates the destination.
	fs.write_file(from, <<>>)
	assert(fs.copy(from, to) == 0)
	assert(fs.read_file(to) == <<>>)
	fs.remove(from)
	fs.remove(to)
	()
})

test("fs_walk_order_test", () {
	root = fresh("walk")
	fs.mkdir(root)
	// Made out of name order; the walk sorts them.
	fs.mkdir(root <> "/c")
	fs.write_file(root <> "/b.txt", <<"b":utf8>>)
	fs.mkdir(root <> "/a")
	fs.write_file(root <> "/a/y.txt", <<"y":utf8>>)
	fs.mkdir(root <> "/a/x")
	fs.write_file(root <> "/a/x/z.txt", <<"z":utf8>>)
	paths = fs.walk(root)
	assert(
		paths ==
			[
				root,
				root <> "/a",
				root <> "/a/x",
				root <> "/a/x/z.txt",
				root <> "/a/y.txt",
				root <> "/b.txt",
				root <> "/c",
			],
	)
	// A file walks as itself.
	assert(fs.walk(root <> "/b.txt") == [root <> "/b.txt"])
	// Contents come after their directory, so the reverse removes them first.
	list.each(list.reverse(paths), fs.remove)
	()
})

// `i:fs/v1` has no operation that makes a symbolic link, so a symlink cycle
// cannot be made here. The depth cap is tested with a chain of 40 nested
// directories instead.
test("fs_walk_depth_cap_test", () {
	root = fresh("walk-deep")
	fs.mkdir(root)
	dirs = chain(root, 40)
	assert(
		fail.catch(() { fs.walk(root) }) ==
			Error(
				fs.FsError(
					operation = "walk",
					path = root <> string.repeat("/d", 33),
					reason = fs.Invalid,
					code = "depth",
				),
			),
	)
	// Exactly 32 levels below the root is allowed.
	assert(
		list.length(
			fs.walk(root <> string.repeat("/d", 8)),
		) ==
			33,
	)
	list.each(list.reverse(dirs), fs.remove)
	fs.remove(root)
	()
})

test("fs_whole_file_test", () {
	path = fresh("whole")
	fs.write_file(path, <<"a\nb\n":utf8>>)
	assert(fs.read_file(path) == <<"a\nb\n":utf8>>)
	assert(fs.read_lines(path) == ["a", "b"])
	// No trailing newline: the last line is still a line.
	fs.write_file(path, <<"a\nb":utf8>>)
	assert(fs.read_file(path) == <<"a\nb":utf8>>)
	assert(fs.read_lines(path) == ["a", "b"])
	fs.write_file(path, <<>>)
	assert(fs.read_lines(path) == [])
	fs.remove(path)
	()
})

// The body fails while the scope owns an open file. The release (`close`)
// needs no `Fail`, so it runs after the error has left the body, and the run
// goes on. The handle is kept in a ref so the test can see that it is closed.
test("fs_scope_error_releases_handle_test", () {
	path = fresh("error")
	boom = fs.FsError(
		operation = "test",
		path = path,
		reason = fs.Other,
		code = "boom",
	)
	slot : task.Ref(Option(fs.File)) = task.ref(None)
	r = fail.catch(() {
		resource.scoped(() {
			f = fs.open(path, fs.WriteTruncate)
			task.set(slot, Some(f))
			fs.write(f, <<"partial":utf8>>)
			fs.flush(f)
			{ fs.metadata(path).size == 7 } |>
				(True) { fail.raise(boom) }
				& (False) { () }
		})
	})
	assert(r == Error(boom))
	task.get(slot) |>
		(Some(f)) {
			assert(
				fail.catch(() { fs.read(f, 1) }) ==
					Error(
						fs.FsError(
							operation = "read",
							path = path,
							reason = fs.Closed,
							code = "closed",
						),
					),
			)
		}
		& (None) { panic("fs_test: the handle was not kept") }
	assert(fs.read_file(path) == <<"partial":utf8>>)
	fs.remove(path)
	()
})

// A cancelled task closes the file its own scope owns: the losing race branch
// is cancelled while it waits, and the handle it sent out is closed.
test("fs_cancel_releases_handle_test", () {
	path = fresh("cancel")
	fs.write_file(path, <<"x":utf8>>)
	got = task.run(() {
		task.scope((_) {
			ready : task.Channel(fs.File) = task.channel()
			task.race(
				() {
					resource.scoped(() {
						f = fs.open(path, fs.ReadOnly)
						task.send(ready, f)
						task.sync(task.never())
					})
				},
				() { task.receive(ready) },
			)
		})
	})
	assert(
		fail.catch(() { fs.read(got, 1) }) ==
			Error(
				fs.FsError(
					operation = "read",
					path = path,
					reason = fs.Closed,
					code = "closed",
				),
			),
	)
	fs.remove(path)
	()
})

test("fs_copy_same_path_test", () {
	path = fresh("copy-same")
	fs.write_file(path, <<"keep":utf8>>)
	assert(
		fail.catch(() { fs.copy(path, path) }) ==
			Error(
				fs.FsError(
					operation = "copy",
					path = path,
					reason = fs.Invalid,
					code = "same",
				),
			),
	)
	assert(fs.read_file(path) == <<"keep":utf8>>)
	fs.remove(path)
	()
})

test("fs_read_lines_invalid_utf8_test", () {
	path = fresh("lines")
	fs.write_file(path, <<255, 254>>)
	assert(
		fail.catch(() { fs.read_lines(path) }) ==
			Error(
				fs.FsError(
					operation = "read_lines",
					path = path,
					reason = fs.Invalid,
					code = "invalid",
				),
			),
	)
	fs.remove(path)
	()
})
