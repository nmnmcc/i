// spec: 9.5
// expect: stop "panic: final"
// at: spec/ensure_normal:7:14
// at: spec/ensure_normal:12:3
import (i/resource)

final = () { panic("final") }

// `with` runs after `body` ends normally.
@main
main = () {
  resource.ensure(() { 1 }, final)
}
