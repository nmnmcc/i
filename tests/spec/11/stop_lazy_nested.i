// spec: 11.7, 9.4
// expect: stop "panic: inner"
// at: spec/main:33:3
// at: spec/main:27:7
// at: spec/main:22:7
// at: spec/main:68:5
// at: spec/main:56:11
// at: spec/main:49:7
// at: spec/main:43:7
// at: spec/main:38:7
// at: spec/main:72:5
// at: spec/main:56:11
// at: spec/main:75:3
(i/control)

Paused = PausedCase | Done
PausedCase { Paused(Continuation(Nil, Paused, Never, Never)) }
Done { Done }

// Two frames that wait in the first computation.
a1 = (b) {
  x = a2(b)
  x
}

a2 = (b) {
  x = a_leaf(b)
  x
}

a_leaf = (b) {
  control.suspend(b, (k) { Paused(k) })
  panic("inner")
}

// Two frames that wait in the second computation, which resumes the first.
b1 = (c, first) {
  x = b2(c, first)
  x
}

b2 = (c, first) {
  x = b_leaf(c, first)
  x
}

b_leaf = (c, first) {
  control.suspend(c, (k) { Paused(k) })
  x = resume(first)
  x
}

resume = (p) {
  p |> {
    Paused(k) -> {
      x = control.resume(k, Nil)
      x
    }
    Done -> panic("not paused")
  }
}

// A stop inside a resume inside a resumed computation shows the frames
// that wait in both.

main = () {
  first = control.boundary((b) {
    a1(b)
    Done
  })
  second = control.boundary((c) {
    b1(c, first)
    Done
  })
  resume(second)
  Nil
}
