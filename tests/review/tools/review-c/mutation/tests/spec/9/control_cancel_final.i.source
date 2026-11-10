// spec: 9.4, 9.5, 11.7
// expect: stop "panic: final"
// at: spec/control_cancel_final:16:12
// at: spec/control_cancel_final:14:5
// at: spec/control_cancel_final:15:37
// at: spec/control_cancel_final:13:3
import (i/control, i/resource)

// `cancel` unwinds the paused computation: its final functions run (§9.5),
// on top of the place of `cancel`.
@main
main = () {
  control.boundary((b) {
    resource.ensure(
      () { control.suspend(b, (k) { control.cancel(k)  Nil }) },
      () { panic("final") },
    )
    Nil
  })
  Nil
}
