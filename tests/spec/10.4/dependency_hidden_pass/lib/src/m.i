(
	i/hostcall
	i/http
	i/request
)

(
	Held
	Svc(..)
	held
	answer
	given
	method
)

@import("app:svc/v1")
Svc = (ping = () -> Int)

Held = (call = hostcall.Call(Int))

held = () -> Held { Held(call = request.ready(1)) }

answer = (h : Held) -> Int { hostcall.take(h.call) }

given = (w : request.Await) -> Int { 1 }

method = (r : http.Request) -> Int { 1 }
