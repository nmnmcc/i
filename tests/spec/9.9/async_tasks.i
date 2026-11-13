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
	task.scope((s) {
		ready: task.Channel(Nil) = task.channel()
		// Both children use the async Await route. Order the host log calls
		// explicitly: workers may otherwise start either call first (§9.9).
		assert(
			task.both(
				() {
					value = later.double(21)
					task.send(ready, Nil)
					value
				},
				() {
					task.receive(ready)
					later.echo("answer")
				},
			) ==
				(42, "answer"),
		)
	})
}
