// spec: 9.11
// expect: stop "panic: request: taken"
// runtime.md 5.1: a second take is a defined stop.
(
	i/request
)

main = () {
	r = request.ready(1)
	request.take(r)
	request.take(r)
}
