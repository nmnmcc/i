(
	testlib = (test) = std/test
	i/process
	i/request
	i/fail
	i/core
)

test("fake_capabilities", () {
	! process.Arguments = process.Arguments(() {
		request.ready(Ok(<<2, 0, 0, 0, 1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99>>))
	})
	assert(process.args() == ["a", "bc"])
	! process.Environment = process.Environment((name) {
		request.ready(Error(name))
	})
	assert(
		fail.catch(() { process.env("missing") }) ==
			Error(process.ProcessError("env", "missing"))
	)
	! process.Entropy = process.Entropy((n) { request.ready(Ok(<<n>>)) })
	assert(process.entropy(7) == <<7>>)
	! process.WorkingDirectory = process.WorkingDirectory(() {
		request.ready(Ok("/fake"))
	})
	assert(process.cwd() == "/fake")
	Nil
})

test("host_capabilities", () {
	assert(core.bit_size(process.entropy(16)) == 128)
	assert(
		fail.catch(() { process.entropy(-1) }) ==
			Error(process.ProcessError("entropy", "invalid"))
	)
	child = process.spawn("sh", ["-c", "exit 3"])
	assert(process.wait(child) == 3)
	assert(process.wait(child) == 3)
	process.close(child)
	assert(
		fail.catch(() { process.wait(child) }) ==
			Error(process.ProcessError("wait", "closed"))
	)
	Nil
})
