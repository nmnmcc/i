(
	i/hostcall
	i/request
)

(
	f
	g
)

f = () -> hostcall.Call(Int) { request.ready(1) }

g = () -> Int { 1 }
