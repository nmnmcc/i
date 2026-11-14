// spec: 9.4, 9.9
// expect: pass
// output: child-final
// runtime.md R3/R4 and resource.ensure: abandoning a scope through an enclosing
// control delimiter must still cancel and drain its child before the final ends.
(
	i/task
	i/control
	i/resource
)

@host("test:io/v1")
Out = (print = (String) -> Nil)

main = () <- (out : Out) {
	control.boundary((b) {
		task.scope((s) {
			armed : task.Channel(Nil) = task.channel()
			task.go(s, () {
				resource.ensure(
					() {
						task.send(armed, Nil)
						task.sync(task.never())
					},
					() {
						task.yield()
						out.print("child-final")
					},
				)
			})
			task.receive(armed)
			control.suspend(b, (k) { control.cancel(k) })
			panic("abandoned scope resumed")
		})
	})
}
