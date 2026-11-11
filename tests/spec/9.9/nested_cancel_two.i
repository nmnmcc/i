// spec: 9.9
// expect: pass
// output: deepest-final
// runtime.md R2/R4/6.5: cancellation drains recursively nested runs; retained
// outer callbacks still route to their resolved calling-task boundary.
(
	i/task
	i/control
	i/resource
	i/fail
)

@host("test:io/v1")
Out {
	Out(print: (text: String) -> Nil)
}

print = (text: String, out ! Out) { out.print(text) }


main = () {
	result = fail.catch(() {
		task.scope((outer) {
			gate: task.Channel(Nil) = task.channel()
			task.go(outer, () {
				task.receive(gate)
				fail.raise("outer")
			})
			notify = control.keep_env((_: Nil) { task.send(gate, Nil) })
			task.run(() {
				task.run(() {
					task.scope((inner) {
						task.go(inner, () {
							resource.ensure(() {
								notify(Nil)
								task.sync(task.never())
							}, () {
								task.yield()
								print("deepest-final")
							})
						})
						task.sync(task.never())
					})
				})
			})
		})
	})
	assert(result == Error("outer"))
}
