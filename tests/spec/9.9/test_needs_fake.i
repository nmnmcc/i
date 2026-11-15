// spec: 9.9
// expect: stop "panic: task: test needs fake IO"
// output: echo real
// runtime.md 6.5: test never root-polls an unsettled real request.
(
	i/task
	i/random
)

@host("test:async/v1", "async")
Later = (echo = (String) -> String)

main = () <- (later : Later) {
	task.test(random.from_seed(1), [], () { later.echo("real") })
}
