// spec: 11.7, 9.4
// expect: stop "panic: inside"
// at: spec/main:23:3
// at: spec/main:17:7
// at: spec/main:28:9
// at: spec/main:27:7
// at: spec/main:39:5
// at: spec/main:44:11
(i/control)

Paused = PausedCase | Done
PausedCase { Paused(Continuation(Nil, Paused, Never, Never)) }
Done { Done }

// A frame that waits inside `inner`, and one that waits outside it.
in1 = (outer, inner) {
  x = leaf(outer, inner)
  x
}

leaf = (outer, inner) {
  control.suspend(outer, (k) { Paused(k) })
  panic("inside")
}

out1 = (outer) {
  x = control.boundary((inner) {
    y = in1(outer, inner)
    y
  })
  x
}

// The resume goes into `inner` again: the stop inside it shows the frames
// of both lists.

main = () {
  p = control.boundary((outer) {
    out1(outer)
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
