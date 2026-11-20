(
	i/process
	i/stdio
	i/task
	i/clock
	i/fail
	i/resource
)

// Program entry.
main = () {
	resource.scoped(() {
		assert(process.args() == ["alpha", "beta"])
		process.env("PATH")
		child = process.spawn_piped(
			"sh",
			["-c", "echo hi; exit 3"],
			process.Null(),
			process.Piped(),
			process.Inherit(),
		)
		assert(process.read_stdout(child, 100) == <<"hi\n":utf8>>)
		assert(process.wait(child) == 3)
		process.close(child)
		cat = process.spawn_piped(
			"cat",
			[],
			process.Piped(),
			process.Piped(),
			process.Inherit(),
		)
		process.write_stdin(cat, <<"hello\n":utf8>>)
		assert(process.read_stdout(cat, 100) == <<"hello\n":utf8>>)
		process.close_stdin(cat)
		assert(process.wait(cat) == 0)
		process.close(cat)
		configured = process.spawn_config(
			"sh",
			["-c", "echo $I_TEST_VAR"],
			process.Null(),
			process.Piped(),
			process.Inherit(),
			process.InheritEnvironment(["I_TEST_VAR=xyz"]),
			process.InheritDirectory(),
		)
		assert(process.read_stdout(configured, 100) == <<"xyz\n":utf8>>)
		assert(process.wait(configured) == 0)
		process.close(configured)
		replaced = process.spawn_config(
			"/bin/sh",
			[
				"-c",
				"test \"$I_TEST_VAR\" = xyz && test -z \"${HOME+x}\" && test \"$PWD\" = /",
			],
			process.Null(),
			process.Null(),
			process.Inherit(),
			process.ReplaceEnvironment(["I_TEST_VAR=xyz"]),
			process.AtDirectory("/"),
		)
		assert(process.wait(replaced) == 0)
		process.close(replaced)
		removed = process.spawn_config(
			"/bin/sh",
			["-c", "test -z \"${HOME+x}\""],
			process.Null(),
			process.Null(),
			process.Inherit(),
			process.InheritEnvironment(["HOME"]),
			process.InheritDirectory(),
		)
		assert(process.wait(removed) == 0)
		process.close(removed)
		// A child belongs to the resource scope around it. A task other than
		// the one that made the scope opens its own `resource.scoped`; here
		// that scope is inside the losing race branch, so cancelling the
		// branch ends the scope and terminates and reaps the child.
		start = clock.monotonic_ns()
		handle = task.scope((_) {
			ready = task.channel()
			task.race(
				() {
					resource.scoped(() {
						c = process.spawn("sleep", ["30"])
						task.send(ready, c)
						task.sync(task.never())
					})
				},
				() { task.receive(ready) },
			)
		})
		assert(
			fail.catch(() { process.wait(handle) }) ==
				Error(
					process.ProcessError(
						operation = "wait",
						reason = process.Closed(),
						code = "closed",
					),
				),
		)
		assert(clock.monotonic_ns() - start < 2000000000)
		// A failing scope releases its child on the way out.
		start = clock.monotonic_ns()
		assert(
			fail.catch(() {
				task.scope((_) {
					resource.scoped(() {
						process.spawn("sleep", ["30"])
						fail.raise("scope failed")
					})
				})
			}) ==
				Error("scope failed"),
		)
		assert(clock.monotonic_ns() - start < 2000000000)
		// Check this run's sleep children; other runs may have their own.
		check = process.spawn_piped(
			"sh",
			[
				"-c",
				// `ps -A` must work, or the check would pass without looking.
				"ps -A >/dev/null 2>&1 || exit 2; ps -A -o ppid=,args= | grep -Eq \"^ *$PPID +sleep 30$\" && exit 1; exit 0",
			],
			process.Null(),
			process.Null(),
			process.Inherit(),
		)
		assert(process.wait(check) == 0)
		process.close(check)
		assert(process.entropy(16) != process.entropy(16))
		stdio.print("process ok")
	})
}
