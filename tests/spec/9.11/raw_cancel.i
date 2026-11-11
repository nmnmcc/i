// spec: 9.11
// expect: pass
// runtime.md 5.1: group cancel-all leaves it open; group close cancels only
// outstanding work; an old alias cannot cancel a reused host slot.
(i/request)
@host("test:async/v1", "request")
Later { Later(echo: (text: String) -> Request(String)) }
@main
main = (later ! Later, owner ! request.Owner, root ! request.Root) {
  old = later.echo("old")
  request.cancel(request.ticket(old))
  assert(request.state(old) == request.Cancelled)
  fresh = later.echo("fresh")
  request.cancel(request.ticket(old))
  assert(request.state(fresh) == request.Waiting)
  assert(request.await_owned(fresh) == "fresh")
  discarded = later.echo("discarded")
  request.cancel_all(owner)
  assert(request.state(discarded) == request.Cancelled)
  assert(request.count(owner) == 0)
  again = later.echo("again")
  request.cancel_all_root(root)
  assert(request.state(again) == request.Cancelled)
  assert(request.count_root(root) == 0)
  last = later.echo("last")
  assert(request.await_owned(last) == "last")
}
