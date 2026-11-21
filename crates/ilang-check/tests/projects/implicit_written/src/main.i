(
	request = i/request
	task = i/task
)

(
	main
)

// A need row may not name an implicit service.
wait = () <- (waiter : request.Await) -> () { () }

run = (f : () <- (request.Owner, task.Scheduler) -> ()) { f() }

// A value parameter and a type definition may.
Holder = (root = request.Root, start = () <- (task.Runner) -> ())
pass = (root : request.Root) -> Holder { Holder(root = root, start = () { () }) }

main = () { () }
