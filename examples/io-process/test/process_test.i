(
	testlib = (test) = i/test
	i/process
	i/request
	i/fail
	i/core
	i/resource
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
							reason = process.Other(),
							code = "missing",
						),
					),
			)
			{
				assert(process.entropy(7) == <<7>>)
				{
					assert(process.cwd() == "/fake")
					()
				} <- (
					process.WorkingDirectory(
						cwd = () { Ok("/fake") },
					),
				)
			} <- (process.Entropy(entropy = (n) { Ok(<<n>>) }))
		} <- (process.Environment(env = (name) { Error(name) }))
	} <- (
		process.Arguments(
			args = () { Ok(<<2, 0, 0, 0, 1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99>>) },
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
				process.ProcessError(
					operation = "entropy",
					reason = process.Invalid(),
					code = "invalid",
				),
			),
	)
	resource.scoped(() {
		child = process.spawn("sh", ["-c", "exit 3"])
		assert(process.wait(child) == 3)
		assert(process.wait(child) == 3)
		process.close(child)
		assert(
			fail.catch(() { process.wait(child) }) ==
				Error(
					process.ProcessError(
						operation = "wait",
						reason = process.Closed(),
						code = "closed",
					),
				),
		)
	})
	()
})
