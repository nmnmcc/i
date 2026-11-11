(
	i/process
	i/stdio
	i/task
	i/clock
	i/fail
)

@main
main = () {
	assert(process.args() == ["alpha", "beta"])
	process.env("PATH")
	child = process.spawn_piped(
		"sh"
		["-c", "echo hi; exit 3"]
		process.Null
		process.Piped
		process.Inherit
	)
	assert(process.read_stdout(child, 100) == <<"hi\n":utf8>>)
	assert(process.wait(child) == 3)
	process.close(child)
	cat = process.spawn_piped(
		"cat"
		[]
		process.Piped
		process.Piped
		process.Inherit
	)
	process.write_stdin(cat, <<"hello\n":utf8>>)
	assert(process.read_stdout(cat, 100) == <<"hello\n":utf8>>)
	process.close_stdin(cat)
	assert(process.wait(cat) == 0)
	process.close(cat)
	configured = process.spawn_config(
		"sh"
		["-c", "echo $I_TEST_VAR"]
		process.Null
		process.Piped
		process.Inherit
		process.InheritEnvironment(["I_TEST_VAR=xyz"])
		process.InheritDirectory
	)
	assert(process.read_stdout(configured, 100) == <<"xyz\n":utf8>>)
	assert(process.wait(configured) == 0)
	process.close(configured)
	start = clock.monotonic_ns()
	child = task.scope((_) {
		ready = task.channel()
		task.race(() {
			child = process.spawn("sleep", ["30"])
			task.send(ready, child)
			task.sync(task.never())
		}, () { task.receive(ready) })
	})
	assert(
		fail.catch(() { process.wait(child) }) ==
			Error(process.ProcessError("wait", "closed"))
	)
	assert(clock.monotonic_ns() - start < 2000000000)
	check = process.spawn_piped(
		"sh"
		["-c", "ps -C sleep -o args= | grep -qx 'sleep 30'; test $? -eq 1"]
		process.Null
		process.Null
		process.Inherit
	)
	assert(process.wait(check) == 0)
	process.close(check)
	assert(process.entropy(16) != process.entropy(16))
	stdio.print("process ok")
}
