// spec: 11.7, 9.3
// expect: stop "panic: boom"
// at: spec/stop_tail_catch:8:33
// at: spec/stop_tail_catch:12:3
import (i/fail)

// A call of `fail.catch` in tail position counts as a tail call: it is gone.
safe = () { fail.catch(() { 1 + panic("boom") }) }

@main
main = () {
  safe()
  Nil
}
