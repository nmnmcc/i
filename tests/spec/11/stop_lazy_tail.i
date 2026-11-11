// spec: 11.7, 11.6, 9.4
// expect: stop "panic: after"
// at: spec/main:30:10
// at: spec/main:18:7
// at: spec/main:13:7
// at: spec/main:38:5
// at: spec/main:43:11
(i/control)

Paused { Paused(Continuation(Nil, Paused, Never, Never))  Done }

w1 = (b) {
  x = w2(b)
  x
}

w2 = (b) {
  x = step(b)
  x
}

// After the resume, the segment of `step` makes a tail call.
step = (b) {
  control.suspend(b, (k) { Paused(k) })
  last(3)
}

last = (n) {
  n |> {
    0 -> panic("after")
    _ -> last(n - 1)
  }
}


main = () {
  p = control.boundary((b) {
    w1(b)
    Done
  })
  p |> {
    Paused(k) -> {
      x = control.resume(k, Nil)
      x
    }
    Done -> panic("not paused")
  }
  Nil
}
