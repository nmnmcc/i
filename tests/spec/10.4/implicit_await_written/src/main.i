(
	i/hostcall
	i/request
)

(
	main
)

wait = (c : hostcall.Call(Int)) <- (w : request.Await) -> Int { hostcall.await(c) }

main = () { () }
