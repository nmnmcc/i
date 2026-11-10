// spec: 9.5, 9.9
// expect: pass
// output: start
// output: nested-final
// output: end
// output: Error("parent")
// An older cancellation cannot interrupt error cleanup, but a new cleanup
// scope must still unwind its own body when one of its children fails.
(
	i/task
	i/resource
	i/fail
	i/core
)

@host("test:io/v1")
Out {
	Out(print: (String) -> Nil)
}

print = (text: String, out ! Out) { out.print(text) }

forever = () -> Never ! task.Scheduler {
	task.yield()
	forever()
}

@main
main = () {
	result = fail.catch(() {
		task.scope((s) {
			gate: task.Channel(Nil) = task.channel()
			task.go(s, () {
				resource.ensure(() { fail.raise("child") }, () {
					print("start")
					task.send(gate, Nil)
					local = fail.catch(() {
						task.run(() {
							task.scope((fresh) {
								task.go(fresh, () {
									task.yield()
									fail.raise("fresh")
								})
								sibling = task.go(fresh, () {
									resource.ensure(forever, () {
										task.yield()
										print("nested-final")
									})
								})
								task.join(sibling)
							})
						})
					})
					assert(local == Error("fresh"))
					print("end")
				})
			})
			task.receive(gate)
			fail.raise("parent")
		})
	})
	print(core.inspect(result))
}
