(i/task, i/request, i/resource)
@host("test:async/v1", "request")
Later { Later(echo: (text: String) -> Request(String)) }
cycle = (n: Int, later ! Later) {
 { n == 0 } |> { True -> Nil
 False -> {
  owner = request.open_owner()
  ! request.Owner = owner
  old = later.echo("old")
  request.cancel_all(owner)
  fresh = later.echo("fresh")
  request.cancel(request.ticket(old))
  assert(request.await_owned(fresh) == "fresh")
  assert(request.state(old) == request.Cancelled)
  request.close_owner(owner)
  cycle(n - 1)
 } }
}

main = () { cycle(200) }