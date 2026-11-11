// spec: 9.11
// expect: pass
// runtime.md 5.1: ready is fresh, reads do not consume, take consumes once,
// terminal tickets return immediately, and all aliases share identity.
(i/request, i/core)

same = (a, b) { a == b }

@main
main = () {
  r = request.ready(37)
  other = request.ready(37)
  t = request.ticket(r)
  assert(r == r)
  assert(r != other)
  assert(t == request.ticket(r))
  assert(core.not(same(r, t)))
  assert(request.state(r) == request.Ready(37))
  assert(request.state(r) == request.Ready(37))
  assert(request.settled(t))
  request.wait_any([t, t])
  assert(request.take(r) == 37)
  assert(request.state(r) == request.Taken)
  request.cancel(t)
  assert(request.state(r) == request.Taken)
  assert(core.inspect(r) == "//request")
  assert(core.inspect(t) == "//ticket")
  assert(core.not(core.portable(r)))
  assert(core.not(core.portable(t)))
  assert(core.hash(t) == core.hash(request.ticket(r)))
}
