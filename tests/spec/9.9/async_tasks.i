// spec: 9.9, 9.11
// expect: pass
// output: double 21
// output: echo answer
// runtime.md D3/5.4: every async namespace routes through the task Await service.
(
	i/task
)

@host("test:async/v1", "async")
Later {
	Later(double: (n: Int) -> Int ? String, echo: (text: String) -> String)
}

@main
main = (later ! Later) {
	assert(
		task.both(() { later.double(21) }, () { later.echo("answer") }) ==
			(42, "answer")
	)
}
