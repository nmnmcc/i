// spec: 9.11
// expect: stop "panic: request: not ready"
// runtime.md 5.1: take on Waiting stops before the host has a progress turn.
(i/request)
@host("test:async/v1", "request")
Later { Later(echo: (String) -> Request(String)) }

main = (later ! Later) { request.take(later.echo("pending")) }
