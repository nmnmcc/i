// spec: 11.7, 9.4, 9.3
// expect: stop "panic: after"
// at: spec/main:36:3
// at: spec/main:44:5
// at: spec/main:48:18
(i/control, i/core, i/fail)

check = (got: String, want: String) {
  { got == want } |> {
    True -> Nil
    False -> panic(got <> " is not " <> want)
  }
}

show = (x) { core.inspect(x) }

Paused { Paused(Continuation(Nil, Paused, Never, Never))  Done }

// The call of `fail.raise` is not a tail call, so the frame of `inner` is
// still on the shadow stack when the error goes out of it.
inner = (b) {
  control.suspend(b, (k) { Paused(k) })
  x = fail.raise(1)
  x + 1
}

// Not in tail position, so the frame of `mid` waits, and the error goes
// through it.
mid = (b) {
  x = inner(b)
  x + 1
}

outer = (b) {
  check(show(fail.catch(() { mid(b) })), "Error(1)")
  panic("after")
}

// The stop shows the frame of `outer`, not the one of `inner` that the
// error went out of.
@main
main = () {
  p = control.boundary((b) {
    outer(b)
    Done
  })
  p |> {
    Paused(k) -> control.resume(k, Nil)
    Done -> panic("not paused")
  }
  Nil
}
