// spec: 6.12, 7.4
// expect: warning W0501

name = (b: Bool) {
  case b {
    True -> "yes"
    _ -> "no"
    False -> "never"
  }
}

@test
names = () {
  case name(False) { "no" -> Nil  _ -> panic("no") }
}
