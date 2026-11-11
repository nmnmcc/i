(i/task, i/request, i/resource, i/core)
@host("test:async/v1", "request")
Later { Later(echo: (text: String) -> Request(String)) }

main = (later ! Later, root ! request.Root) {
 owner = request.open_owner()
 ! request.Owner = owner
 a = later.echo("a")
 b = later.echo("b")
 assert(request.count(owner) == 2)
 task.scope((s) {
  task.go(s, () { request.wait_none(owner) })
  task.go(s, () { request.wait_none_root(root) })
 })
 assert(request.count(owner) == 0)
 request.close_owner(owner)
 assert(request.take(a) == "a")
 assert(request.take(b) == "b")
 request.wait_none(owner)
 request.close_owner(owner)
}