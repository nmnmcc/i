// spec: 9.9
// expect: stop "panic: task: shared value is gone"
// A queue's ref belongs to its task scope; a resource scope that outlives
// that task scope releases the queue after the ref is gone.
(
	i/queue
	i/resource
	i/task
)

main = () { resource.scoped(() { task.scope((g) { queue.unbounded() }) }) }
