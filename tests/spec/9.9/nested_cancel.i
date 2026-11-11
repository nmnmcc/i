// spec: 9.9
// expect: pass
// output: nested-final
// runtime.md R2/R4/6.5: parent cancellation drains every nested child and final
// before the parent scope returns; a nested loop does not own a second host k.
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
				task.scope((inner) {
					task.go(inner, () {
						resource.ensure(() {
							notify(Nil)
							task.sync(task.never())
						}, () {
							task.yield()
							print("nested-final")
						})
					})
					task.sync(task.never())
				})
			})
		})
	})
	assert(result == Error("outer"))
}
