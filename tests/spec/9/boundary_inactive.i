// spec: 9.4, 9.6, 11.7
// expect: stop "control: boundary is not active"
import (i/assertion)

get = (a ! Assert) { a }

// The `Assert` of `assertion.catch` is used after `catch` has ended.
@main
main = () {
  r = assertion.catch(() { get() })
  case r {
    Ok(a) -> {
      ! Assert = a
      assert(False)
    }
    Error(_) -> Nil
  }
}
