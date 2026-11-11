// spec: 9.2, 9.4
// expect: pass
// multicore.md D4: every call keeps all services; each call makes a fresh function.
// multicore.md 4: retained bases survive repeated resume and cancellation.
(i/control, i/core, i/resource)

Name { Name(text: String) }
Hop { Paused(Continuation(Nil, Hop, Never, Never)) Done(String) }
name = (value ! Name) { value.text }

walk = (b, n) {
  { n == 3 } |> {
    True -> Done(name())
    False -> {
      control.suspend(b, (k) { Paused(k) })
      walk(b, n + 1)
    }
  }
}

identity = { value -> value }

make = () { control.keep_env((b) { walk(b, 0) }) }


main = () {
  ! Name = Name("kept")
  kept = make()
  pure = (n: Int) { n + 1 }
  a = control.keep_env(pure)
  b = control.keep_env(pure)
  assert(a != b)
  assert(a(37) == 38)
  inner = control.keep_env((boundary) {
    ! Name = Name("inner")
    identity(walk(boundary, 0))
  })
  ! Name = Name("caller")
  Paused(k1) = control.boundary((boundary) { kept(boundary) })
  Paused(k2) = control.resume(k1, Nil)
  Paused(k3) = control.resume(k2, Nil)
  assert(control.resume(k3, Nil) == Done("kept"))
  Paused(ki1) = control.boundary((boundary) { inner(boundary) })
  Paused(ki2) = control.resume(ki1, Nil)
  Paused(ki3) = control.resume(ki2, Nil)
  assert(control.resume(ki3, Nil) == Done("inner"))
  ! Name = Name("final")
  with_final = control.keep_env((boundary) {
    resource.ensure(() {
      control.suspend(boundary, (k) { Paused(k) })
      Done("unreachable")
    }, () { assert(name() == "final") })
  })
  ! Name = Name("canceller")
  Paused(k4) = control.boundary((boundary) { with_final(boundary) })
  control.cancel(k4)
  control.boundary((boundary) {
    ! Name = Name("unused")
    ! Extra(Nil, Never, Assert) = Extra(boundary)
    f = control.keep_env(pure)
    assert(core.not(core.portable(f)))
  })
}

Extra(A, E, N) { Extra(Boundary(A, E, N)) }
