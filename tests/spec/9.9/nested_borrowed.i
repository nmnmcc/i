// spec: 9.9, 9.11
// expect: pass
// output: echo borrowed
// runtime.md 6.4/6.5: a nested run includes explicitly awaited borrowed outer
// requests; changing the run Root does not cancel or hide that producer.
(
	i/task
	i/request
)

@host("test:async/v1", "request")
Later {
	Later(echo: (text: String) -> Request(String))
}


main = (later ! Later) {
	r = later.echo("borrowed")
	assert(task.run(() { request.await_owned(r) }) == "borrowed")
}
