// spec: 10.1
// expect: compile-error E0208
// host-boundary §3.2: the accessor name is `make_` and the type name in
// snake case, so `HttpClient` gives `make_http_client`.
(
	HttpClient(..)
	main
)

@import("app:http/v1")
HttpClient = (get = (String) -> Int)

make_http_client = 1

main = () { () }
