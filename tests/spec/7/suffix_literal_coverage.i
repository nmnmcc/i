// spec: 7.4
// expect: pass
part = (s: "abc" | "xyz") { s |> { r <> "bc" -> r  r <> "yz" -> r } }
main = () { assert(part("abc") == "a")  assert(part("xyz") == "x")  Nil }
