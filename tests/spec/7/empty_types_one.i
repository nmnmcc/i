// spec: 7.1, 4.8
// expect: pass
// A type test of a custom type with no constructors, when it is the only
// one at its place, or tested together with the others.

Window {}
Socket {}

kind = (h: Window | Int) {
  case h {
    _: Window -> "window"
    _ -> "other"
  }
}

both = (h: Window | Socket | Int) {
  case h {
    _: Window | Socket -> "host"
    _ -> "other"
  }
}

@test
t = () {
  case (kind(1), both(2)) {
    ("other", "other") -> Nil
    _ -> panic("no")
  }
}
