// old-syntax-check: E0101
import (task = app/task, net = app/net, i/fail, i/resource)
copy = (stream) {
  case net.read(stream, 65536) {
    <<>> -> Nil
    bytes -> { net.write(stream, bytes) copy(stream) }
  }
}
serve_next = (scope, listener) {
  resource.acquire(
    () { net.accept(listener) },
    (stream) { net.close_stream(stream) },
    (stream) {
      task.go(scope, () { serve_next(scope, listener) })
      fail.catch(() { copy(stream) })
      Nil
    }
  )
}
entry_body = () {
  resource.acquire(
    () { net.listen("127.0.0.1", 8080) },
    (listener) { net.close_listener(listener) },
    (listener) {
      task.local_scope((scope) {
        task.go(scope, () { serve_next(scope, listener) })
        Nil
      })
    }
  )
}
