// spec: 13, 11.8
// expect: pass
import (i/core)

// `core.inspect` and `core.hash` are linear in the size of the value, so
// long lists take little time.
build = (n: Int, acc: List(Int)) -> List(Int) {
  case n { 0 -> acc  _ -> build(n - 1, [n % 10, ..acc]) }
}

@test
inspect_long_list = () {
  // "[" and "]", one digit and ", " for each part but the last.
  case core.byte_size(core.inspect(build(100000, []))) {
    300000 -> Nil
    n -> panic(core.inspect(n))
  }
}

@test
hash_long_list = () {
  case core.hash(build(300000, [])) == core.hash(build(300000, [])) {
    True -> Nil
    False -> panic("hash")
  }
}

@test
inspect_nested = () {
  case core.inspect([(1, "a\n"), (-2, "b")]) {
    "[(1, \"a\\n\"), (-2, \"b\")]" -> Nil
    s -> panic(s)
  }
}
