(
	testlib = (test) = std/test
	i/process
	i/request
	i/fail
	i/core
)

test("fake_capabilities", () {
	{
		assert(process.args() == ["a", "bc"])
		{
			assert(
				fail.catch(() { process.env("missing") }) ==
					Error(
						process.ProcessError(
							operation = "env",
							code = "missing",
						),
					),
			)
			{
				assert(process.entropy(7) == <<7>>)
				{
					assert(process.cwd() == "/fake")
					Nil
				} <- (
					process.WorkingDirectory(
						cwd = () { request.ready(Ok("/fake")) },
					),
				)
			} <- (process.Entropy(entropy = (n) { request.ready(Ok(<<n>>)) }))
		} <- (process.Environment(env = (name) { request.ready(Error(name)) }))
	} <- (
		process.Arguments(
			args = () {
				request.ready(
					Ok(<<2, 0, 0, 0, 1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99>>),
				)
			},
		),
	)
})

test("host_capabilities", () {
	assert(
		core.bit_size(process.entropy(16)) == 128,
	)
	assert(
		fail.catch(() { process.entropy(-1) }) ==
			Error(
				process.ProcessError(operation = "entropy", code = "invalid"),
			),
	)
	child = process.spawn("sh", ["-c", "exit 3"])
	assert(process.wait(child) == 3)
	assert(process.wait(child) == 3)
	process.close(child)
	assert(
		fail.catch(() { process.wait(child) }) ==
			Error(process.ProcessError(operation = "wait", code = "closed")),
	)
	Nil
})
