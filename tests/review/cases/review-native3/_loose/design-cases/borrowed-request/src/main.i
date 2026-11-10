// Design counterexample; not run. Host eventually releases gate.
(i/task, i/request)
@host("review:gate/v1", "request")
Gate { Gate(wait: () -> Request(Nil)) }
@main
main = (gate ! Gate) {
  pending = gate.wait()
  task.run(() { request.await(pending) })
}
