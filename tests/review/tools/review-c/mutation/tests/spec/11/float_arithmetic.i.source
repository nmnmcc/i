// spec: 11.3
// expect: pass
import (i/core)

check = (got: String, want: String) {
  case got == want {
    True -> Nil
    False -> panic(got <> " is not " <> want)
  }
}

show = (x) { core.inspect(x) }

@test
operations = () {
  check(show(1.5 +. 2.25), "3.75")
  check(show(1.0 -. 3.5), "-2.5")
  check(show(2.0 *. 0.25), "0.5")
  check(show(1.0 /. 4.0), "0.25")
  check(show(-.1.5), "-1.5")
  check(show(0.1 +. 0.2), "0.30000000000000004")
  check(show(1.0 /. 0.0), "0.0")
  check(show(-.1.0 /. 0.0), "0.0")
  check(show(0.0 /. 0.0), "0.0")
}

@test
compare = () {
  check(show(1.0 <. 2.0), "True")
  check(show(2.0 <=. 2.0), "True")
  check(show(3.0 >. 4.0), "False")
  check(show(-.0.0 >=. 0.0), "True")
  check(show(0.0 == -.0.0), "True")
}
