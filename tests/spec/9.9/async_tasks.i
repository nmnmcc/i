// spec: 9.9, 9.11
// expect: pass
// output: double 21
// output: echo answer
// runtime.md D3/5.4: every async namespace routes through the task Await service.
(
	i/task
)

@host("test:async/v1", "async")
Later = (double = (Int) <- (Fail(String)) -> Int, echo = (String) -> String)

main = () <- (later: Later) {
	// §9.9 leaves the order of the two host calls open ("some serial order of
	// task steps"), so the second starts after the first has answered.
	c: task.Channel(Nil) = task.channel()
	assert(
		task.both(
			() {
				value = later.double(21)
				task.send(c, Nil)
				value
			},
			() {
				task.receive(c)
				later.echo("answer")
			},
		) ==
			(42, "answer"),
	)
}
