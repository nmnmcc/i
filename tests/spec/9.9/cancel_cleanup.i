// spec: 9.9
// expect: pass
// output: cleanup-child
// output: cleanup-parent
// runtime.md R2/R4: cancellation waits for shielded finals and fresh scopes;
// the older cancellation does not cancel their cleanup children.
(
	i/task
	i/resource
	i/fail
)

@host("test:io/v1")
Out = (print = (String) -> Nil)

print = (text: String) <- (out: Out) { out.print(text) }

main = () {
	result = fail.catch(() {
		task.scope((s) {
			gate: task.Channel(Nil) = task.channel()
			task.go(s, () {
				resource.ensure(
					() {
						task.send(gate, Nil)
						task.sync(task.never())
					},
					() {
						task.scope((cleanup) {
							task.go(cleanup, () {
								task.yield()
								print("cleanup-child")
							})
							task.yield()
						})
						print("cleanup-parent")
					},
				)
			})
			task.go(s, () {
				task.receive(gate)
				fail.raise("boom")
			})
			task.sync(task.never())
		})
	})
	assert(result == Error("boom"))
}
