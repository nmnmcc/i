(
	testlib = (test) = i/test
	i/process
	i/resource
	i/task
	i/clock
	i/fail
	i/core
)

// These tests run real children through POSIX `sh` and coreutils.

// The marker child touches its file 300 ms after it starts (`sleep 0.3` in
// `marker_script`); the check waits five times that before it looks.
check_delay_ns = 1500000000

nap = (ns : Int) {
	fail.catch(() { clock.sleep_ns(ns) }) |>
		(Ok(_)) { () }
		& (Error(_)) { panic("process_test: sleep failed") }
}

now = () {
	fail.catch(() { clock.monotonic_ns() }) |>
		(Ok(t)) { t }
		& (Error(_)) { panic("process_test: clock failed") }
}

text = (bytes : BitArray) -> String {
	core.bit_array_to_string(bytes) |>
		(Ok(s)) { s }
		& (Error(_)) { panic("process_test: not UTF-8") }
}

// A new empty directory, made by `mktemp -d`.
temp_dir = () {
	resource.scoped(() {
		c = process.spawn_piped(
			"sh",
			["-c", "d=$(mktemp -d) && printf %s \"$d\""],
			process.Null(),
			process.Piped(),
			process.Piped(),
		)
		out = process.collect(c)
		assert(process.wait(c) == 0)
		text(out.stdout)
	})
}

remove_dir = (path : String) {
	resource.scoped(() {
		c = process.spawn("rm", ["-rf", path])
		assert(process.wait(c) == 0)
		()
	})
}

exists = (path : String) -> Bool {
	resource.scoped(() {
		c = process.spawn("sh", ["-c", "test -e \"$1\"", "sh", path])
		process.wait(c) == 0
	})
}

// A child that creates `path` after the marker delay and then lives on.
marker_script = "sleep 0.3; touch \"$1\"; sleep 30"

test("spawn_collect_test", () {
	resource.scoped(() {
		c = process.spawn_piped(
			"sh",
			["-c", "printf hello"],
			process.Null(),
			process.Piped(),
			process.Piped(),
		)
		out = process.collect(c)
		assert(out.stdout == <<"hello":utf8>>)
		assert(out.stderr == <<>>)
		assert(process.wait(c) == 0)
		()
	})
})

// `collect` drains both pipes at once. The child writes 256 KiB to stdout and
// then 256 KiB to stderr; stdout reaches end of file only when the child
// exits, so a reader that drained stdout before stderr would deadlock on the
// full stderr pipe.
test("collect_drains_both_pipes_concurrently_test", () {
	resource.scoped(() {
		c = process.spawn_piped(
			"sh",
			["-c", "yes x | head -c 262144; yes y | head -c 262144 >&2"],
			process.Null(),
			process.Piped(),
			process.Piped(),
		)
		out = process.collect(c)
		assert(core.bit_size(out.stdout) == 262144 * 8)
		assert(core.bit_size(out.stderr) == 262144 * 8)
		assert(process.wait(c) == 0)
		()
	})
})

test("spawn_piped_stdin_echo_test", () {
	resource.scoped(() {
		c = process.spawn_piped(
			"sh",
			["-c", "cat"],
			process.Piped(),
			process.Piped(),
			process.Inherit(),
		)
		process.write_stdin(c, <<"echo me":utf8>>)
		process.close_stdin(c)
		assert(process.read_stdout(c, 100) == <<"echo me":utf8>>)
		// Zero bytes is end of file.
		assert(process.read_stdout(c, 100) == <<>>)
		assert(process.wait(c) == 0)
		()
	})
})

test("spawn_config_at_directory_test", () {
	resource.scoped(() {
		c = process.spawn_config(
			"sh",
			["-c", "test \"$I_PROCESS_TEST\" = yes && test \"$PWD\" = /"],
			process.Null(),
			process.Null(),
			process.Inherit(),
			process.ReplaceEnvironment(["I_PROCESS_TEST=yes"]),
			process.AtDirectory("/"),
		)
		assert(process.wait(c) == 0)
		()
	})
})

test("terminate_then_wait_test", () {
	start = now()
	resource.scoped(() {
		c = process.spawn("sh", ["-c", "sleep 30"])
		process.terminate(c)
		code = process.wait(c)
		assert(code != 0)
		// Terminating a child that already exited is not an error.
		process.terminate(c)
		()
	})
	// Far less than the child's 30 seconds: wait did not hang.
	assert(now() - start < 10000000000)
	()
})

test("scope_owns_child_test", () {
	dir = temp_dir()
	// The probe works: a child that is let run creates its marker.
	shown = dir <> "/shown"
	resource.scoped(() {
		c = process.spawn("sh", ["-c", "touch \"$1\"", "sh", shown])
		assert(process.wait(c) == 0)
		()
	})
	assert(exists(shown))
	// A child that would outlive its scope is stopped and reaped with it.
	marker = dir <> "/marker"
	start = now()
	resource.scoped(() {
		process.spawn("sh", ["-c", marker_script, "sh", marker])
		()
	})
	// The scope did not wait for the child's 30 seconds.
	assert(now() - start < 10000000000)
	nap(check_delay_ns)
	assert(exists(marker) == False())
	remove_dir(dir)
})

test("spawn_missing_program_test", () {
	r = fail.catch(() {
		resource.scoped(() {
			process.spawn("/nonexistent/i-process-test", [])
			()
		})
	})
	assert(
		r ==
			Error(
				process.ProcessError(
					operation = "spawn",
					reason = process.NotFound(),
					code = "not_found",
				),
			),
	)
	()
})

// The host keeps the exit status: a second wait gives the same code at once.
test("second_wait_gives_same_code_test", () {
	resource.scoped(() {
		c = process.spawn("sh", ["-c", "exit 3"])
		assert(process.wait(c) == 3)
		assert(process.wait(c) == 3)
		()
	})
})

test("wait_after_close_is_closed_test", () {
	resource.scoped(() {
		c = process.spawn("sh", ["-c", "exit 0"])
		process.close(c)
		r = fail.catch(() { process.wait(c) })
		assert(
			r ==
				Error(
					process.ProcessError(
						operation = "wait",
						reason = process.Closed(),
						code = "closed",
					),
				),
		)
		()
	})
})

// A task that holds a scope with a running child is cancelled when its task
// scope fails. The release terminates and reaps the child, and the run goes on.
test("cancelled_scope_stops_child_test", () {
	dir = temp_dir()
	marker = dir <> "/marker"
	stop = process.ProcessError(
		operation = "test",
		reason = process.Other(),
		code = "stop",
	)
	start = now()
	r = fail.catch(() {
		task.scope((s) {
			gate : task.Channel(()) = task.channel()
			task.go(s, () {
				resource.scoped(() {
					process.spawn("sh", ["-c", marker_script, "sh", marker])
					task.send(gate, ())
					task.sync(task.never())
				})
			})
			task.receive(gate)
			fail.raise(stop)
		})
	})
	assert(r == Error(stop))
	assert(now() - start < 10000000000)
	nap(check_delay_ns)
	assert(exists(marker) == False())
	remove_dir(dir)
})
