// spec: 10.4
// expect: compile-error E0323
// host-boundary §5.1: `i/request` is gated by the same switch.
(
	i/request
)

main = () { request.take(request.ready(1)) }
