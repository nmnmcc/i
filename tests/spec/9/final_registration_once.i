// spec: 9.4, 9.5
// expect: pass
// output: body
// output: start
// output: outer
(
	i/control
	i/resource
)

@host("test:io/v1")
Out = (print = (String) -> Nil)

main = () <- (out : Out) {
	control.boundary((b) {
		resource.ensure(
			() {
				resource.ensure(
					() { out.print("body") },
					() {
						out.print("start")
						control.suspend(b, (k) { control.cancel(k) })
						out.print("end")
					},
				)
			},
			() { out.print("outer") },
		)
	})
}
