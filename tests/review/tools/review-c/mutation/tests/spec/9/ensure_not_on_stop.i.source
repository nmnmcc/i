// spec: 9.5, 11.7
// expect: stop "panic: body"
import (i/resource)

// When the program stops, final functions are not run.
@main
main = () {
  resource.ensure(() { panic("body") }, () { panic("final") })
}
