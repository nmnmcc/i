// spec: 9.4, 9.3, 9.5
// expect: pass
(i/control, i/core, i/fail, i/resource)

check = (got: String, want: String) {
  { got == want } |> {
    True -> Nil
    False -> panic(got <> " is not " <> want)
  }
}

show = (x) { core.inspect(x) }

Paused { Paused(Continuation(Nil, Paused, Never, Never))  Done }
Held { Held(Continuation(Nil, Held, Never, Never))  Over }

// Puts `text` in front of the value of the boundary `log`.
note = (log, text) {
  control.suspend(log, (k) { text <> " " <> control.resume(k, Nil) })
}

// A frame whose final function puts its name in the log.
named = (log, name, body) {
  x = resource.ensure(body, () { note(log, name) })
  x + 1
}

// A cancel goes through a `fail.catch` that waits: it does not stop there.
@test
cancel_through_catch = () {
  r = control.boundary((log) {
    p = control.boundary((b) {
      named(log, "outer", () {
        r = fail.catch(() {
          named(log, "inner", () {
            control.suspend(b, (k) { Paused(k) })
            0
          })
        })
        r |> {
          Ok(x) -> x
        }
      })
      Done
    })
    p |> {
      Paused(k) -> control.cancel(k)
      Done -> panic("not paused")
    }
    "end"
  })
  check(r, "inner outer end")
}

// A final function that suspends is paused with the frames around it. A
// final function cannot raise an error (§9.5), so this conformance case
// cancels its record. The error case uses D15's cancel behaviour and is
// checked against eager in `control_lazy.rs`, not as a conformance rule.
paused_final = (log, h) {
  named(log, "outer", () {
    named(log, "inner", () {
      resource.ensure(() { 0 }, () {
        control.suspend(h, (k2) { Held(k2) })
        Nil
      })
    })
  })
  Over
}

// A cancel goes through the record of a final function that waits.
@test
cancel_through_final = () {
  r = control.boundary((log) {
    held = control.boundary((h) { paused_final(log, h) })
    held |> {
      Held(k2) -> control.cancel(k2)
      Over -> panic("not held")
    }
    "end"
  })
  check(r, "inner outer end")
}
