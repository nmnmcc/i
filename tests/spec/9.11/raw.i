// spec: 9.7, 9.11
// expect: pass
// runtime.md 5.1, 5.3: starts return immediately; answers stay on typed cells.
// Closing a group cancels Waiting members and preserves detached Ready answers.
(i/request)
@host("test:async/v1", "request")
Later { Later(double: (n: Int) -> Request(Result(Int, String)), echo: (text: String) -> Request(String)) }
@main
main = (later ! Later, root ! request.Root, owner ! request.Owner) {
  a = later.double(21)
  b = later.echo("answer")
  assert(request.count(owner) == 2)
  assert(request.count_root(root) == 2)
  assert(request.state(a) == request.Waiting)
  assert(request.await_owned(b) == "answer")
  assert(request.await_owned(a) == Ok(42))
  assert(request.count_root(root) == 0)
  r = request.within(() {
    value = later.double(-1)
    request.wait_any([request.ticket(value)])
    value
  })
  assert(request.state(r) == request.Ready(Error("negative")))
  assert(request.take(r) == Error("negative"))
}
