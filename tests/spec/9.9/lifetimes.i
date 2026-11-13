// spec: 9.9
// expect: pass
// output: body
// output: child
// output: after
// runtime.md R3: a normal exit waits for every child, even without join.
(
	i/task
)

@host("test:io/v1")
Out = (print = (String) -> Nil)

print = (text: String) <- (out: Out) { out.print(text) }

main = () {
	assert(
		task.scope((s) {
			ready: task.Channel(Nil) = task.channel()
			task.go(s, () {
				task.receive(ready)
				task.yield()
				print("child")
			})
			print("body")
			// Order the log while retaining the no-join structured exit check.
			task.send(ready, Nil)
			42
		}) ==
			42,
	)
	print("after")
}
