// spec: 9.11, 10.2
// expect: stop "panic: request: no active entry"
// runtime.md D5 and 5.1: an explicit poll does not turn a pure entry async.
(
	i/request
)

main = () { request.poll() }
