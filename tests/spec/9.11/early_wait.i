// spec: 9.11
// expect: stop "panic: request: wait returned early"
// runtime.md D3 and 5.1: a fake Await cannot fabricate an answer.
(i/request)
@host("test:async/v1", "request")
Later { Later(echo: (String) -> Request(String)) }

main = (later ! Later) {
  ! request.Await = request.Await(wait: (_) { Nil })
  request.await(later.echo("pending"))
}
