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
			gate: task.Channel(Nil) = task.channel()
			task.go(s, () {
				task.receive(gate)
				print("child")
			})
			print("body")
			task.send(gate, Nil)
			42
		}) ==
			42,
	)
	print("after")
}
