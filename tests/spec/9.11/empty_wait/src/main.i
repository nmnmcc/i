// spec: 9.11
// expect: stop "panic: request: empty wait"
// runtime.md 5.1: wait_any([]) always stops, including in a sync entry.
(
	i/request
)

main = () { request.wait_any([]) }
