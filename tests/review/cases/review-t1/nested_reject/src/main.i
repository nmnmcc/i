g = (s: String - "a") { s |> { "b" -> 1  _: String - "a" - "b" -> 2 } }
f = (p: (String,Bool)) { p |> {
 ("a",True) -> 0
 (s,_) -> g(s)
} }
