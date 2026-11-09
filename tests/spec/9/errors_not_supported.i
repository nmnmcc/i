// spec: 9.3
// expect: compile-error E0701
import (i/fail)

@test
raises = () {
  case fail.catch(() { fail.raise("e") }) {
    Ok(_) -> panic("no")
    Error(_) -> Nil
  }
}
