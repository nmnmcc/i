// spec: 9.2, 9.4, 9.9
// expect: pass
(
	i/task
	i/fail
)

Name = (text = String)
name = () <- (n: Name) { n.text }

main = () {
	made = fail.catch(() {
		task.keep((_: Nil) {
			task.yield()
			fail.raise(name())
		})
	}) <- (Name(text = "saved"))
	made |>
		(Ok(callback)) {
			assert(
				fail.catch(() { callback(Nil) <- (Name(text = "caller")) }) ==
					Error("saved"),
			)
		}
		| (_) { panic("task.keep raised at creation") }
}
