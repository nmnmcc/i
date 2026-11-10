// spec: 9.4, 9.5
// expect: pass
import (i/control, i/resource)

// §9.4 rule 6: a continuation that is never resumed or cancelled does not
// run its final functions.
@main
main = () {
  r = control.boundary((b) {
    resource.ensure(() { control.suspend(b, (k) { 1 }) }, () { panic("final") })
  })
  case r {
    1 -> Nil
    _ -> panic("not the value of f")
  }
}
