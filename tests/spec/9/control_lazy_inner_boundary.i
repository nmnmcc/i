// spec: 9.4
// expect: pass
(i/control, i/core)

check = (got: String, want: String) {
  { got == want } |> {
    True -> Nil
    False -> panic(got <> " is not " <> want)
  }
}

show = (x) { core.inspect(x) }

// `n` frames that wait.
wait = (n, f) {
  n |> {
    0 -> f()
    _ -> {
      x = wait(n - 1, f)
      x <> ""
    }
  }
}

// Two suspends to `outer` go through `inner`, which is paused with them.
// Then a suspend to `inner`, and a last one to `outer`.
steps = (outer, inner) {
  a = control.suspend(outer, (k) { "o1 " <> control.resume(k, 1) })
  c = control.suspend(outer, (k) { "o2 " <> control.resume(k, a + 1) })
  d = control.suspend(inner, (k) { "i " <> control.resume(k, c + 1) })
  e = control.suspend(outer, (k) { "o3 " <> control.resume(k, d + 1) })
  show(e)
}

@test
inner_boundary_after_resuspend = () {
  r = control.boundary((outer) {
    wait(3, () {
      control.boundary((inner) { wait(3, () { steps(outer, inner) }) })
    })
  })
  check(r, "o1 o2 o3 i 4")
}

// With no frames between: the computation that the `f` of the suspend to
// `inner` resumes suspends to `outer` again.
@test
inner_then_outer = () {
  r = control.boundary((outer) {
    control.boundary((inner) {
      x = control.suspend(outer, (k) { "o " <> control.resume(k, 1) })
      y = control.suspend(inner, (k) { "i " <> control.resume(k, x + 1) })
      z = control.suspend(outer, (k) { "o " <> control.resume(k, y + 1) })
      show(z)
    })
  })
  check(r, "o o i 3")
}
