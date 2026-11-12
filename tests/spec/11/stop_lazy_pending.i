// spec: 11.7, 9.4
// expect: stop "panic: after"
// at: spec/main:31:3
// at: spec/main:25:7
// at: spec/main:20:7
// at: spec/main:15:7
// at: spec/main:39:5
// at: spec/main:43:18
(i/control)

Paused = PausedCase | Done
PausedCase { Paused(Continuation(Nil, Paused, Never, Never)) }
Done { Done }

// Three frames that wait.
f1 = (b) {
  x = f2(b)
  x
}

f2 = (b) {
  x = f3(b)
  x
}

f3 = (b) {
  x = leaf(b)
  x
}

leaf = (b) {
  control.suspend(b, (k) { Paused(k) })
  panic("after")
}

// After the resume, the stop shows the innermost frame, the three frames
// that wait, then the place of `resume`.

main = () {
  p = control.boundary((b) {
    f1(b)
    Done
  })
  p |> {
    Paused(k) -> control.resume(k, Nil)
    Done -> panic("not paused")
  }
  Nil
}
