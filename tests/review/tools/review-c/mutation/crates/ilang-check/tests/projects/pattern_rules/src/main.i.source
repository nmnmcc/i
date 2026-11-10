(a, a) = (1, 2)

f = () {
  [x, ..x] = [1]
  y = (z, z) = (1, 2)
  "a" <> w = "ab"
  (b = (c, b)) = (1, 2)
  Nil
}

params = (p, p) { p }
labels = (x = l, y = l) { x }
env = (s ! Assert, s) { s }
typed = (n: Int, n = m) { n }

choices = (v) {
  case v {
    (a, b) | (a, _) -> a
    Ok(x) | Error(y) -> x
    Ok(x) | Error(x) -> x
    [k, ..] | [_, k, ..] | [] -> 0
  }
}

ctor = (v) {
  case v {
    Nope(x) -> x
    Ok(n) -> n
  }
}
