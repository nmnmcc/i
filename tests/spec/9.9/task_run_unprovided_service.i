// spec: 9.9
// expect: compile-error E0501
// An unrelated need still belongs to run's caller.
(
	i/task
)

Dep = (value = Int)
missing_dep: (() <- (Dep) -> ()) -> () = (body) { task.run(body) }
