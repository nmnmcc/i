// spec: 13, 11.3
// expect: stop "float: result is not finite"
// at: spec/stop_to_float:10:3
// at: spec/stop_to_float:15:3

import (i/core)

// A library function of the runtime in tail position keeps its position.
value = () {
  core.to_float(core.shift_left(1, 1024))
}

@main
main = () {
  value()
  Nil
}
