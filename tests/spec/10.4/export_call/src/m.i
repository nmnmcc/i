(
	i/hostcall
	i/request
)

(
	f
)

f = () -> hostcall.Call(Int) { request.ready(1) }
